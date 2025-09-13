use adw::{
    gio::prelude::SettingsExt,
    prelude::{ActionRowExt, NavigationPageExt, PreferencesGroupExt, PreferencesRowExt},
};
use bluer::{Device, Session, Uuid};
use futures::future;
use gtk4::prelude::{ButtonExt, ListBoxRowExt, WidgetExt};
use relm4::{
    AsyncComponentSender, FactorySender,
    component::{AsyncComponentParts, SimpleAsyncComponent},
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
};
use tracing::{debug, error};

use crate::{consts::{DEVICE_ADDRESS_KEY, SAMSUNG_SPP_UUID}, model::device_info::DeviceInfo, settings};

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
    settings: adw::gio::Settings,
    is_loading: bool,
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
            set_title: "Select a Device",

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::Clamp {

                    if model.devices.is_empty() {
                        adw::StatusPage {
                            set_icon_name: Some("bluetooth-disconnected-symbolic"),
                            set_title: "No Galaxy Buds detected",
                            set_description: Some("First you need to pair a Galaxy Buds device in your system settings."),

                            gtk4::Button {
                                set_label: "Refresh",
                                #[watch]
                                set_sensitive: !model.is_loading,
                                connect_clicked => PageConnectionInput::LoadDevices,
                            }
                        }
                    } else {
                        adw::PreferencesPage {
                            #[local_ref]
                            devices_group -> adw::PreferencesGroup {
                                set_title: "Discovered Galaxy Buds",
                            }
                        }
                    }
                }
            },
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let settings = settings::get_settings();
        let devices: FactoryVecDeque<DeviceComponent> = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |output| match output {
                DeviceOutput::Connect(device) => PageConnectionInput::SelectDevice(device),
            });

        let mut model = PageConnectionModel {
            devices,
            settings: settings.clone(),
            is_loading: true,
        };
        let devices_group = model.devices.widget();
        let widgets = view_output!();

        // Perform the initial device scan before showing the page.
        match discover_galaxy_buds().await {
            Ok(discovered_devices) => {
                let address = settings.string(DEVICE_ADDRESS_KEY).to_string();

                if !address.is_empty() {
                    for device in &discovered_devices {
                        if device.address().to_string() == address {
                            debug!(address = %address, "Found autoconnect device, sending output.");
                            let device_info = DeviceInfo::from_device(device.clone()).await;
                            let _ = sender.output(PageConnectionOutput::SelectDevice(device_info));
                            return AsyncComponentParts { model, widgets };
                        }
                    }
                    let _ = settings.set_string(DEVICE_ADDRESS_KEY, "");
                    debug!("Autoconnect address set, but device not found.");
                }

                debug!("Populating list with discovered devices.");
                model.populate_devices_list(discovered_devices).await;
            }
            Err(e) => {
                error!("Failed to discover devices: {}", e);
            }
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, message: Self::Input, sender: AsyncComponentSender<Self>) {
        match message {
            PageConnectionInput::LoadDevices => {
                debug!("PageConnectionInput::LoadDevices");
                self.is_loading = true;
                if let Ok(discovered_devices) = discover_galaxy_buds().await {
                    self.populate_devices_list(discovered_devices).await;
                }
            }

            PageConnectionInput::SelectDevice(device) => {
                debug!("Selected device");
                let _ = self
                    .settings
                    .set_string(DEVICE_ADDRESS_KEY, &device.address);
                let _ = sender.output(PageConnectionOutput::SelectDevice(device));
            }
        }
    }
}

impl PageConnectionModel {
    /// Clears the existing list and populates it with the given devices.
    async fn populate_devices_list(&mut self, discovered_devices: Vec<Device>) {
        let mut guard = self.devices.guard();
        guard.clear();
        for device in discovered_devices {
            guard.push_back(DeviceInfo::from_device(device).await);
        }
        self.is_loading = false;
    }
}

/// Scans for and returns the devices matching the Galaxy Buds SPP UUID.
async fn discover_galaxy_buds() -> Result<Vec<Device>, Box<dyn std::error::Error>> {
    let session = Session::new().await.unwrap();
    let adapter = session.default_adapter().await.unwrap();
    adapter.set_powered(true).await?;

    let custom_spp_uuid: Uuid = SAMSUNG_SPP_UUID.parse()?;

    // Get all known device addresses and create a future to check each one.
    let device_addrs = adapter.device_addresses().await?;
    let check_futures = device_addrs
        .into_iter()
        .filter_map(|addr| adapter.device(addr).ok())
        .map(|device| async move {
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
