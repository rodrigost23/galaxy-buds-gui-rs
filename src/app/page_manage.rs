use adw::prelude::{ActionRowExt, NavigationPageExt, PreferencesRowExt};
use gtk4::prelude::{BoxExt, ButtonExt, ListBoxRowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent, WorkerController,
};

use tracing::{debug, error};

use crate::{
    app::{
        dialog_find::DialogFindOutput,
        page_noise::{PageNoiseInput, PageNoiseModel},
    },
    buds_worker::{BluetoothWorker, BudsWorkerInput, BudsWorkerOutput},
    define_page_enum,
    model::{
        buds_message::{BudsCommand, BudsMessage},
        buds_status::{BudsStatus, UpdateFrom},
        device_info::DeviceInfo,
        util::OptionNaExt,
    },
};

#[derive(Debug)]
enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

define_page_enum!(PageId, Page {
    Noise(Controller<PageNoiseModel>),
});

#[derive(Debug)]
pub struct PageManageModel {
    bt_worker: WorkerController<BluetoothWorker>,
    connection_state: ConnectionState,
    buds_status: Option<BudsStatus>,
    device: DeviceInfo,
    active_page: Option<Page>,
}

#[derive(Debug)]
pub enum PageManageInput {
    Connect,
    Disconnect,
    BluetoothEvent(BudsWorkerOutput),
    BluetoothCommand(BudsCommand),
    OpenFindDialog,
    FindDialogCommand(DialogFindOutput),
    Navigate(PageId),
}

#[derive(Debug)]
pub enum PageManageOutput {
    OpenFindDialog,
    Disconnect,
    Navigate(adw::NavigationPage),
}

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
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::Clamp {
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
                                            set_label: &model.buds_status.or_na(BudsStatus::battery_text),
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
                                            set_label: &model.buds_status.or_na(BudsStatus::case_battery_text),
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
                                add_suffix = &gtk4::Label {
                                    #[watch]
                                    set_label: &model.buds_status.or_na(BudsStatus::noise_control_mode_text),
                                    add_css_class: "dim-label",
                                },
                                add_suffix: &gtk4::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => PageManageInput::Navigate(PageId::Noise),
                            },
                            adw::ActionRow {
                                set_title: "Touch options",
                                #[watch]
                                set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                                set_activatable: true,
                                add_suffix: &gtk4::Image::from_icon_name("go-next-symbolic"),

                            },
                            adw::ActionRow {
                                set_title: "Equalizer",
                                #[watch]
                                set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                                set_activatable: true,
                                add_suffix: &gtk4::Image::from_icon_name("go-next-symbolic"),

                            },
                            adw::ActionRow {
                                set_title: "Find my Buds",
                                #[watch]
                                set_sensitive: matches!(model.connection_state, ConnectionState::Connected),
                                set_activatable: true,
                                add_suffix: &gtk4::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => PageManageInput::OpenFindDialog,
                            },
                        }
                    }
                }
            },
        }
    }

    fn init(
        device: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PageManageModel {
            device: device.clone(),
            bt_worker: BluetoothWorker::builder()
                .detach_worker(device.clone())
                .forward(sender.input_sender(), PageManageInput::BluetoothEvent),
            connection_state: ConnectionState::Disconnected,
            buds_status: None,
            active_page: None,
        };

        let widgets = view_output!();

        sender.input(PageManageInput::Connect);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            PageManageInput::BluetoothEvent(output) => match output {
                BudsWorkerOutput::DataReceived(data) => match data {
                    BudsMessage::StatusUpdate(status) => {
                        debug!("Status Update: {:?}", status);
                        if let Some(buds_status) = self.buds_status.as_mut() {
                            buds_status.update(&status);
                        }
                    }
                    BudsMessage::ExtendedStatusUpdate(ext_status) => {
                        debug!("Extended Status Update: {:?}", ext_status);
                        let buds_status = BudsStatus::from(&ext_status);
                        if let Some(Page::Noise(page)) = &self.active_page {
                            page.emit(PageNoiseInput::ModeUpdate(buds_status.noise_control_mode()));
                        }
                        self.buds_status = Some(buds_status);
                    }
                    BudsMessage::NoiseControlsUpdate(noise_controls_updated) => {
                        debug!("Noise Controls Update: {:?}", noise_controls_updated);
                        if let Some(buds_status) = self.buds_status.as_mut() {
                            buds_status.update(&noise_controls_updated);
                        }
                        if let Some(Page::Noise(page)) = &self.active_page {
                            page.emit(PageNoiseInput::ModeUpdate(noise_controls_updated.noise_control_mode));
                        }
                    }
                    BudsMessage::Unknown { id, buffer: _ } => {
                        debug!("Unknown message ID: {}", id);
                    }
                },
                BudsWorkerOutput::Connected => {
                    debug!("Bluetooth connected");
                    self.connection_state = ConnectionState::Connected;
                }
                BudsWorkerOutput::Disconnected => {
                    debug!("Bluetooth disconnected");
                    self.connection_state = ConnectionState::Disconnected;
                }
                BudsWorkerOutput::Error(err) => {
                    error!("Bluetooth error: {}", err);
                    self.connection_state = ConnectionState::Error(err);
                }
            },
            PageManageInput::Connect => {
                if let ConnectionState::Disconnected | ConnectionState::Error(_) =
                    self.connection_state
                {
                    debug!("PageManageInput::Connect");
                    self.connection_state = ConnectionState::Connecting;
                    self.bt_worker
                        .sender()
                        .send(BudsWorkerInput::Connect)
                        .unwrap();
                }
            }
            PageManageInput::Disconnect => {
                self.bt_worker
                    .sender()
                    .send(BudsWorkerInput::Disconnect)
                    .unwrap();
                sender.output(PageManageOutput::Disconnect).unwrap();
            }
            PageManageInput::BluetoothCommand(command) => {
                self.bt_worker
                    .sender()
                    .send(BudsWorkerInput::SendCommand(command))
                    .unwrap();
            }
            PageManageInput::OpenFindDialog => {
                sender.output(PageManageOutput::OpenFindDialog).unwrap()
            }
            PageManageInput::FindDialogCommand(cmd) => {
                sender.input(PageManageInput::BluetoothCommand(match cmd {
                    DialogFindOutput::Find(active) => BudsCommand::Find(active),
                }));
            }
            PageManageInput::Navigate(page_id) => {
                match page_id {
                    PageId::Noise => {
                        // Replace page if not a match
                        if !matches!(self.active_page, Some(Page::Noise(_))) {
                            if let Some(buds_status) = &self.buds_status {
                                self.active_page = Some(Page::Noise(
                                    PageNoiseModel::builder()
                                        .launch(buds_status.noise_control_mode())
                                        .forward(sender.input_sender(), |msg| match msg {}),
                                ));
                            }
                        }
                    }
                };

                if let Some(page) = &self.active_page {
                    sender
                        .output(PageManageOutput::Navigate(page.widget().clone()))
                        .unwrap();
                }
            }
        }
    }
}
