use adw::prelude::{AdwApplicationWindowExt, NavigationPageExt};
use galaxy_buds_rs::message::{
    extended_status_updated::ExtendedStatusUpdate, status_updated::StatusUpdate,
};
use gtk4::prelude::GtkWindowExt;
use relm4::{Component, ComponentParts, ComponentSender, SimpleComponent, WorkerController};

use crate::{
    buds_worker::{BluetoothWorker, BluetoothWorkerInput, BluetoothWorkerOutput},
    model::{buds_message::BudsMessage, device_info::DeviceInfo},
};

enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

enum BudsStatus {
    StatusUpdate(StatusUpdate),
    ExtendedStatusUpdate(ExtendedStatusUpdate),
}

pub struct PageManageModel {
    active_page: String,
    bt_worker: WorkerController<BluetoothWorker>,
    connection_state: ConnectionState,
    buds_status: Option<BudsStatus>,
    device: DeviceInfo,
}

#[derive(Debug)]
pub enum PageManageInput {
    Connect,
    Disconnect,
    SelectRow(String),
    ShowContent(bool),
    BluetoothEvent(BluetoothWorkerOutput),
}

#[derive(Debug)]
pub enum PageManageOutput {}

#[relm4::component(pub)]
impl SimpleComponent for PageManageModel {
    type Input = PageManageInput;
    type Output = PageManageOutput;
    type Init = DeviceInfo;

    view! {
        #[root]
        adw::NavigationPage {
            set_title: "Connect",

            #[wrap(Some)]
            set_child = &adw::Clamp {
                gtk4::Label {
                    set_label: model.device.name.as_str()
                }
            }
        }
    }

    fn init(
        device: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PageManageModel {
            active_page: "home".into(),
            device,
            bt_worker: BluetoothWorker::builder()
                .detach_worker(())
                .forward(sender.input_sender(), PageManageInput::BluetoothEvent),
            connection_state: ConnectionState::Disconnected,
            buds_status: None,
        };

        let widgets = view_output!();

        sender.input(PageManageInput::Connect);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            PageManageInput::SelectRow(row_name) => {
                self.active_page = match row_name.as_str() {
                    "row_noise" => "page-noise",
                    "row_touch" => "page-touch",
                    "row_equalizer" => "page-equalizer",
                    "row_find" => "page-find",
                    _ => "home",
                }
                .to_string();
            }
            PageManageInput::ShowContent(show) => {
                if !show {
                    self.active_page = "home".into();
                }
            }
            PageManageInput::BluetoothEvent(output) => match output {
                BluetoothWorkerOutput::DataReceived(data) => match data {
                    BudsMessage::StatusUpdate(status) => {
                        println!("Status Update: {:?}", status);
                        self.buds_status = Some(BudsStatus::StatusUpdate(status));
                    }
                    BudsMessage::ExtendedStatusUpdate(ext_status) => {
                        println!("Extended Status Update: {:?}", ext_status);
                        self.buds_status = Some(BudsStatus::ExtendedStatusUpdate(ext_status));
                    }
                    BudsMessage::Unknown { id, buffer: _ } => {
                        println!("Unknown message ID: {}", id);
                    }
                },
                BluetoothWorkerOutput::Connected => {
                    println!("Bluetooth connected");
                    self.connection_state = ConnectionState::Connected;
                }
                BluetoothWorkerOutput::Disconnected => {
                    println!("Bluetooth disconnected");
                    self.connection_state = ConnectionState::Disconnected;
                }
                BluetoothWorkerOutput::Error(err) => {
                    eprintln!("Bluetooth error: {}", err);
                    self.connection_state = ConnectionState::Error(err);
                }
                BluetoothWorkerOutput::Discovered(device) => {
                    println!("Discovered device: {:?}", device);
                    self.device = device;
                }
            },
            PageManageInput::Connect => {
                if let ConnectionState::Disconnected | ConnectionState::Error(_) =
                    self.connection_state
                {
                    self.connection_state = ConnectionState::Connecting;
                    self.bt_worker
                        .sender()
                        .send(BluetoothWorkerInput::Connect)
                        .unwrap();
                }
            }
            PageManageInput::Disconnect => todo!(),
        }
    }
}
