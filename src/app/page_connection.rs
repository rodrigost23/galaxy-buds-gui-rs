use adw::prelude::{ActionRowExt, NavigationPageExt, PreferencesGroupExt, PreferencesRowExt};
use bluer::{Device, Session, Uuid};
use futures::future;
use gtk4::prelude::ListBoxRowExt;
use relm4::{
    AsyncComponentSender, FactorySender,
    component::{AsyncComponentParts, SimpleAsyncComponent},
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
};
use tracing::{debug};

use crate::model::device_info::DeviceInfo;

#[derive(Debug)]
struct DeviceComponent {
    device: DeviceInfo,
}

#[derive(Debug)]
enum DeviceInput {
    Connect,
}

#[derive(Debug)]
enum DeviceOutput {
    Connect(DeviceInfo),
}

#[relm4::factory]
impl FactoryComponent for DeviceComponent {
    type Init = DeviceInfo;
    type Input = DeviceInput;
    type Output = DeviceOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::ActionRow {
            set_activatable: true,
            connect_activated => DeviceInput::Connect,
            set_title: self.device.name.as_str(),
        }
    }

    fn init_model(device: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { device }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            DeviceInput::Connect => {
                let _ = sender.output(DeviceOutput::Connect(self.device.clone()));
            }
        }
    }
}

#[derive(Debug)]
pub struct PageConnectionModel {
    devices: FactoryVecDeque<DeviceComponent>,
}

#[derive(Debug)]
pub enum PageConnectionInput {
    SelectDevice(DeviceInfo),
    LoadDevices,
}

#[derive(Debug)]
pub enum PageConnectionOutput {
    SelectDevice(DeviceInfo),
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for PageConnectionModel {
    type Input = PageConnectionInput;
    type Output = PageConnectionOutput;
    type Init = ();

    view! {
        #[root]
        adw::NavigationPage {
            set_title: "Connect",

            #[wrap(Some)]
            set_child = &adw::Clamp {
                adw::PreferencesPage {
                    #[local_ref]
                    devices_group -> adw::PreferencesGroup {
                        set_title: "Discovered Galaxy Buds",
                    }
                }
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let devices: FactoryVecDeque<DeviceComponent> = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |output| match output {
                DeviceOutput::Connect(device) => PageConnectionInput::SelectDevice(device),
            });

        let model = PageConnectionModel { devices };
        let devices_group = model.devices.widget();
        let widgets = view_output!();

        sender.input(PageConnectionInput::LoadDevices);

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, message: Self::Input, sender: AsyncComponentSender<Self>) {
        match message {
            PageConnectionInput::LoadDevices => {
                debug!("PageConnectionInput::LoadDevices");
                self.devices.guard().clear();
                if let Ok(discovered_devices) = self.discover_galaxy_buds().await {
                    for device in discovered_devices.iter() {
                        self.devices
                            .guard()
                            .push_back(DeviceInfo::from_device(device.clone()).await);
                    }
                }
            }

            PageConnectionInput::SelectDevice(device_info) => {
                debug!("Selected device");
                let _ = sender.output(PageConnectionOutput::SelectDevice(device_info));
            }
        }
    }
}

impl PageConnectionModel {
    /// Scans for and returns the devices matching the Galaxy Buds SPP UUID.
    async fn discover_galaxy_buds(self: &Self) -> Result<Vec<Device>, Box<dyn std::error::Error>> {
        let session = Session::new().await.unwrap();
        let adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;

        let custom_spp_uuid: Uuid = "2e73a4ad-332d-41fc-90e2-16bef06523f2".parse()?;

        // Get all known device addresses and create a future to check each one.
        let device_addrs = adapter.device_addresses().await?;
        let check_futures = device_addrs
            .into_iter()
            .filter_map(|addr| adapter.device(addr).ok())
            .map(|device| async {
                // Check for the specific UUID. If found, return the device.
                let has_uuid = match device.uuids().await {
                    Ok(Some(uuids)) => uuids.contains(&custom_spp_uuid),
                    _ => false,
                };

                if has_uuid { Some(device) } else { None }
            });

        // Run all checks concurrently and filter out the `None` results.
        let found_devices: Vec<Device> = future::join_all(check_futures)
            .await
            .into_iter()
            .flatten()
            .collect();

        // Log the found devices.
        for device in &found_devices {
            debug!(device = ?device, "Found device");
        }

        Ok(found_devices)
    }
}
