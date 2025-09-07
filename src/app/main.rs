use adw::{HeaderBar, glib::object::ObjectExt, prelude::AdwApplicationWindowExt};
use galaxy_buds_rs::message::{
    extended_status_updated::ExtendedStatusUpdate, status_updated::StatusUpdate,
};
use gtk4::prelude::{GtkWindowExt, WidgetExt};
use relm4::{
    Component, ComponentParts, ComponentSender, Controller, SimpleComponent, WorkerController,
    prelude::{AsyncComponent, AsyncComponentController, AsyncController},
};

use crate::{
    app::page_connection::{PageConnectionModel, PageConnectionOutput},
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

pub struct AppModel {
    active_page: String,
    pages: AppPages,
    bt_worker: WorkerController<BluetoothWorker>,
    connection_state: ConnectionState,
    bt_device: Option<DeviceInfo>,
    buds_status: Option<BudsStatus>,
}

struct AppPages {
    connection: AsyncController<PageConnectionModel>,
}

#[derive(Debug)]
pub enum AppInput {
    Connect,
    Disconnect,
    SelectRow(String),
    ShowContent(bool),
    BluetoothEvent(BluetoothWorkerOutput),
}

#[derive(Debug)]
pub enum AppOutput {}

pub struct AppInit {}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Input = AppInput;
    type Output = AppOutput;
    type Init = AppInit;

    view! {
        #[root]
        adw::ApplicationWindow {
            set_default_width: 800,
            set_default_height: 800,

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::NavigationView {
                    add = model.pages.connection.widget(),
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            pages: {
                AppPages {
                    connection: PageConnectionModel::builder().launch(()).forward(
                        sender.input_sender(),
                        |msg| match msg {
                            PageConnectionOutput::Connect(device_info) => todo!(),
                        },
                    ),
                }
            },
            active_page: "home".into(),
            bt_worker: BluetoothWorker::builder()
                .detach_worker(())
                .forward(sender.input_sender(), AppInput::BluetoothEvent),
            connection_state: ConnectionState::Disconnected,
            bt_device: None,
            buds_status: None,
        };

        let builder = gtk4::Builder::from_string(include_str!("../gtk/main.ui"));

        let split_view: adw::NavigationSplitView = builder
            .object("split_view")
            .expect("Missing split_view in UI file");
        window.set_content(Some(&split_view));

        // Breakpoint for responsive layout
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            540.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split_view, "collapsed", Some(&true.into()));
        window.add_breakpoint(breakpoint);

        let widgets = view_output!();

        sender.input(AppInput::Connect);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppInput::SelectRow(row_name) => {
                self.active_page = match row_name.as_str() {
                    "row_noise" => "page-noise",
                    "row_touch" => "page-touch",
                    "row_equalizer" => "page-equalizer",
                    "row_find" => "page-find",
                    _ => "home",
                }
                .to_string();
            }
            AppInput::ShowContent(show) => {
                if !show {
                    self.active_page = "home".into();
                }
            }
            AppInput::BluetoothEvent(output) => match output {
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
                    self.bt_device = Some(device);
                }
            },
            AppInput::Connect => {
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
            AppInput::Disconnect => todo!(),
        }
    }
}
