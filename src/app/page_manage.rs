use adw::prelude::{ActionRowExt, NavigationPageExt, PreferencesRowExt};
use galaxy_buds_rs::message::{
    extended_status_updated::ExtendedStatusUpdate, status_updated::StatusUpdate,
};
use gtk4::prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent, WorkerController,
};

use crate::{
    buds_worker::{BluetoothWorker, BudsWorkerInput, BudsWorkerOutput},
    model::{
        buds_message::{BudsCommand, BudsMessage},
        device_info::DeviceInfo,
    },
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
    None,
}

impl BudsStatus {
    pub fn battery_text(&self) -> String {
        let (battery_left, battery_right) = match self {
            BudsStatus::StatusUpdate(s) => (
                Some(s.battery_left.to_string()),
                Some(s.battery_right.to_string()),
            ),
            BudsStatus::ExtendedStatusUpdate(s) => (
                Some(s.battery_left.to_string()),
                Some(s.battery_right.to_string()),
            ),
            _ => (None, None),
        };

        match (battery_left, battery_right) {
            (Some(left), Some(right)) => {
                if left == right {
                    format!("L / R {}%", left)
                } else {
                    format!("L {}% / R {}%", left, right)
                }
            }
            _ => "N/A".to_string(),
        }
    }

    pub fn case_battery_text(&self) -> String {
        match self {
            BudsStatus::StatusUpdate(s) => format!("{}%", s.battery_case),
            BudsStatus::ExtendedStatusUpdate(s) => format!("{}%", s.battery_case),
            BudsStatus::None => "N/A".to_string(),
        }
    }
}

pub struct PageManageModel {
    active_page: String,
    bt_worker: WorkerController<BluetoothWorker>,
    connection_state: ConnectionState,
    buds_status: BudsStatus,
    device: DeviceInfo,
}

#[derive(Debug)]
pub enum PageManageInput {
    Connect,
    Disconnect,
    SelectRow(String),
    ShowContent(bool),
    BluetoothEvent(BudsWorkerOutput),
    BluetoothCommand(BudsCommand),
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
            set_title: model.device.name.as_str(),

            #[wrap(Some)]
            set_child = &adw::Clamp {
                gtk4::Box {
                    set_orientation: gtk4::Orientation::Vertical,
                    set_margin_horizontal: 4,
                    set_margin_vertical: 8,
                    set_spacing: 16,

                    gtk4::Box {
                        set_orientation: gtk4::Orientation::Vertical,
                        set_margin_horizontal: 4,
                        set_margin_vertical: 8,
                        set_spacing: 16,

                        gtk4::Image {
                            set_icon_name: Some("image-missing"),
                            set_icon_size: gtk4::IconSize::Large,
                            set_pixel_size: 128,
                        },

                        gtk4::Label {
                            #[watch]
                            set_label: model.device.name.as_str(),
                            add_css_class: "title-1",
                        },

                        #[transition = "SlideUp"]
                        match model.connection_state {
                            ConnectionState::Connected => gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_halign: gtk4::Align::Center,
                                set_spacing: 8,

                                gtk4::Box {
                                    set_spacing: 4,

                                    gtk4::Image {
                                        set_icon_name: Some("audio-headphones-symbolic"),
                                    },

                                    gtk4::Label {
                                        #[watch]
                                        set_label: model.buds_status.battery_text().as_str(),
                                        add_css_class: "heading",
                                    },
                                },

                                gtk4::Box {
                                    set_spacing: 4,

                                    gtk4::Image {
                                        set_icon_name: Some("printer-symbolic"),
                                    },

                                    gtk4::Label {
                                        #[watch]
                                        set_label: model.buds_status.case_battery_text().as_str(),
                                        add_css_class: "heading",
                                    },
                                },
                            },
                            ConnectionState::Connecting => gtk4::Label {
                                set_label: "Connecting..."
                            },
                            ConnectionState::Disconnected | ConnectionState::Error(_) => gtk4::Box {
                                set_orientation: gtk4::Orientation::Horizontal,
                                set_halign: gtk4::Align::Center,
                                set_spacing: 8,

                                gtk4::Label { set_label: "Disconnected" },
                                gtk4::Button {
                                    set_label: "Connect",
                                    connect_clicked => PageManageInput::Connect,
                                }
                            },
                        },
                    },

                    adw::PreferencesGroup {
                        adw::ActionRow {
                            set_title: "Noise control",
                            #[watch]
                            set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                            set_activatable: true,

                        },
                        adw::ActionRow {
                            set_title: "Touch options",
                            #[watch]
                            set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                            set_activatable: true,

                        },
                        adw::ActionRow {
                            set_title: "Equalizer",
                            #[watch]
                            set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                            set_activatable: true,

                        },
                        adw::ActionRow {
                            set_title: "Find Start",
                            #[watch]
                            set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                            set_activatable: true,
                            connect_activated => PageManageInput::BluetoothCommand(BudsCommand::FindStart),
                        },
                        adw::ActionRow {
                            set_title: "Find Stop",
                            #[watch]
                            set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                            set_activatable: true,
                            connect_activated => PageManageInput::BluetoothCommand(BudsCommand::FindStop),
                        },
                    }
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
            device: device.clone(),
            bt_worker: BluetoothWorker::builder()
                .detach_worker(device.clone())
                .forward(sender.input_sender(), PageManageInput::BluetoothEvent),
            connection_state: ConnectionState::Disconnected,
            buds_status: BudsStatus::None,
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
                BudsWorkerOutput::DataReceived(data) => match data {
                    BudsMessage::StatusUpdate(status) => {
                        println!("Status Update: {:?}", status);
                        self.buds_status = BudsStatus::StatusUpdate(status);
                    }
                    BudsMessage::ExtendedStatusUpdate(ext_status) => {
                        println!("Extended Status Update: {:?}", ext_status);
                        self.buds_status = BudsStatus::ExtendedStatusUpdate(ext_status);
                    }
                    BudsMessage::Unknown { id, buffer: _ } => {
                        println!("Unknown message ID: {}", id);
                    }
                },
                BudsWorkerOutput::Connected => {
                    println!("Bluetooth connected");
                    self.connection_state = ConnectionState::Connected;
                }
                BudsWorkerOutput::Disconnected => {
                    println!("Bluetooth disconnected");
                    self.connection_state = ConnectionState::Disconnected;
                }
                BudsWorkerOutput::Error(err) => {
                    eprintln!("Bluetooth error: {}", err);
                    self.connection_state = ConnectionState::Error(err);
                }
                BudsWorkerOutput::Discovered(device) => {
                    println!("Discovered device: {:?}", device);
                    self.device = device;
                }
            },
            PageManageInput::Connect => {
                if let ConnectionState::Disconnected | ConnectionState::Error(_) =
                    self.connection_state
                {
                    println!("PageManageInput::Connect");
                    self.connection_state = ConnectionState::Connecting;
                    self.bt_worker
                        .sender()
                        .send(BudsWorkerInput::Connect)
                        .unwrap();
                }
            }
            PageManageInput::Disconnect => todo!(),
            PageManageInput::BluetoothCommand(command) => {
                self.bt_worker
                    .sender()
                    .send(BudsWorkerInput::SendCommand(command))
                    .unwrap();
            }
        }
    }
}
