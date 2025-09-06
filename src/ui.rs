use adw::{glib::object::ObjectExt, prelude::AdwApplicationWindowExt};
use bluer::{Device, DeviceProperty};
use galaxy_buds_rs::message::{
    extended_status_updated::ExtendedStatusUpdate, status_updated::StatusUpdate,
};
use gtk4::prelude::{GtkWindowExt, WidgetExt};
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

pub struct App {
    active_page: String,
    bt_worker: WorkerController<BluetoothWorker>,
    connection_state: ConnectionState,
    bt_device: Option<DeviceInfo>,
    buds_status: Option<BudsStatus>,
}

pub struct AppWidgets {
    banner: adw::Banner,
    split_view: adw::NavigationSplitView,
    content_page: adw::NavigationPage,
    sidebar_list: gtk4::ListBox,
    view_stack: adw::ViewStack,
    device_name_label: gtk4::Label,
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

impl SimpleComponent for App {
    type Input = AppInput;
    type Output = AppOutput;
    type Init = AppInit;
    type Root = adw::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .default_height(800)
            .default_width(800)
            .build()
    }

    fn init(
        _init: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = App {
            active_page: "home".into(),
            bt_worker: BluetoothWorker::builder()
                .detach_worker(())
                .forward(sender.input_sender(), AppInput::BluetoothEvent),
            connection_state: ConnectionState::Disconnected,
            bt_device: None,
            buds_status: None,
        };

        let builder = gtk4::Builder::from_string(include_str!("gtk/main.ui"));

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

        let widgets = AppWidgets {
            split_view,
            banner: builder.object("banner").expect("Missing banner in UI file"),
            content_page: builder
                .object("content_page")
                .expect("Missing content_page in UI file"),
            sidebar_list: builder
                .object("sidebar_list")
                .expect("Missing sidebar_list in UI file"),
            view_stack: builder
                .object("view_stack")
                .expect("Missing view_stack in UI file"),
            device_name_label: builder
                .object("device_name")
                .expect("Missing device_name in UI file"),
        };

        // Sidebar row selection handler
        widgets.sidebar_list.connect_row_selected({
            let sender = sender.clone();
            move |_, row| {
                if let Some(row) = row {
                    sender.input(AppInput::SelectRow(row.widget_name().to_string()));
                }
            }
        });

        // Back button/content visibility handler
        widgets
            .split_view
            .connect_notify_local(Some("show-content"), {
                let sender = sender.clone();
                move |s, _| {
                    sender.input(AppInput::ShowContent(s.shows_content()));
                }
            });

        widgets.banner.connect_button_clicked({
            let sender = sender.clone();
            move |_| {
                sender.input(AppInput::Connect);
            }
        });

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
                    }
                    BudsMessage::ExtendedStatusUpdate(ext_status) => {
                        println!("Extended Status Update: {:?}", ext_status);
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

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        match self.active_page.as_str() {
            "home" => {
                widgets.split_view.set_show_content(false);
                if let Some(first_row) = widgets.sidebar_list.row_at_index(0) {
                    widgets.sidebar_list.select_row(Some(&first_row));
                }
            }
            _ => widgets.split_view.set_show_content(true),
        }

        match self.connection_state {
            ConnectionState::Connected => widgets.banner.set_revealed(false),
            ConnectionState::Disconnected => {
                widgets.banner.set_revealed(true);
                widgets.banner.set_title(&"Disconnected");
                if self.bt_device.is_some() {
                    widgets.banner.set_button_label(Some(&"Connect"));
                }
            }
            ConnectionState::Connecting => {
                widgets.banner.set_revealed(true);
                widgets.banner.set_title(&format!(
                    "Connecting to {}...",
                    self.bt_device
                        .as_ref()
                        .map(|d| d.name.as_str())
                        .unwrap_or("Galaxy Buds")
                ));
                widgets.banner.set_button_label(None);
            }
            ConnectionState::Error(ref err) => {
                widgets.banner.set_revealed(true);
                widgets.banner.set_title(&format!("Error: {}", err));
                widgets.banner.set_button_label(Some(&"Retry"));
            }
        }

        // Update device name label
        if let Some(device) = &self.bt_device {
            widgets.device_name_label.set_text(device.name.as_str());
        } else {
            widgets.device_name_label.set_text("Not connected");
        }

        widgets.view_stack.set_visible_child_name(&self.active_page);
    }
}
