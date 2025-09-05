use adw::{NavigationPage, NavigationSplitView, ViewStack, Window, prelude::*};
use bluer::rfcomm::{Profile, Role};
use futures::StreamExt;
use galaxy_buds_rs::{
    message::{self, Message, extended_status_updated::ExtendedStatusUpdate, ids},
    model::Model,
};
use gtk4::{Application, Builder, ListBox};
use std::sync::mpsc;
use std::{thread, time::Duration};
use tokio::{io::AsyncReadExt, runtime::Runtime};

const APP_ID: &str = "com.github.rodrigost23.galaxy-buds-gui-rs";

fn main() {
    // Initialize Libadwaita and create the GTK Application.
    adw::init().expect("Failed to initialize Libadwaita.");
    let app = Application::builder().application_id(APP_ID).build();

    // Connect the "activate" signal to the build_ui function.
    app.connect_activate(build_ui);

    // Run the application.
    app.run();
}

pub async fn bluetooth_loop(tx: mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;
    // TODO: Discover devices or connect directly if already paired
    let device = adapter.device("MAC_ADDRESS".parse()?).unwrap();

    if !device.is_connected().await? {
        println!("Connecting...");
        device.connect().await?;
    }
    println!("Connected to device: {:?}", device.all_properties().await?);
    

    let uuids = device.uuids().await?.unwrap_or_default();
    let spp_uuid = bluer::id::ServiceClass::SerialPort.into();
    if !uuids.contains(&spp_uuid) {
        return Err("Device does not support Serial Port Profile (SPP)".into());
    }
    println!("Device supports Serial Port Profile (SPP).");

    println!("Registering SPP profile with UUID: {}", spp_uuid);
    let profile = Profile {
        uuid: spp_uuid,
        role: Some(Role::Client),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(true),
        ..Default::default()
    };
    let mut handle = session.register_profile(profile).await?;

    println!("Profile registered. Ready to connect.");

    if let Some(req) = handle.next().await {
        println!("Connection request from {:?} accepted.", req.device());
        let mut stream = req.accept()?;
        println!("RFCOMM stream established. Type messages to send.");

        let mut buffer = [0u8; 2048];

        loop {
            let num_bytes_read = stream.read(&mut buffer).await?;
            let buff = &buffer[..num_bytes_read];

            let id = buff[3].to_be();
            let message = Message::new(buff, Model::BudsLive);

            if id == 242 {
                continue;
            }

            if id == ids::STATUS_UPDATED {
                let msg: message::status_updated::StatusUpdate = message.into();
                tx.send(format!("{:?}", msg))?;
                continue;
            }

            if id == ids::EXTENDED_STATUS_UPDATED {
                let msg: ExtendedStatusUpdate = message.into();
                tx.send(format!("{:?}", msg))?;
                continue;
            }
        }
    } else {
        Err("No connection request received".into())
    }
}

fn build_ui(app: &Application) {
    let builder = Builder::from_string(include_str!("../data/ui/main.ui"));

    let window: Window = builder.object("main_window").unwrap();
    let content_page: NavigationPage = builder.object("content_page").unwrap();
    let sidebar_list: ListBox = builder.object("sidebar_list").unwrap();
    let split_view: NavigationSplitView = builder.object("split_view").unwrap();
    let view_stack: ViewStack = builder.object("view_stack").unwrap();

    window.set_application(Some(app));
    sidebar_list.unselect_all();

    // --- Rust native channel ---
    let (tx, rx) = mpsc::channel::<String>();

    // Spawn Tokio runtime in background thread for bluer
    thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            if let Err(e) = bluetooth_loop(tx).await {
                eprintln!("Bluetooth loop error: {e}");
            }
        });
    });

    // Poll the channel periodically in GTK main loop
    gtk4::glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(msg) = rx.try_recv() {
            println!("Got update: {}", msg);
            // TODO: Update GTK widgets here if needed
        }
        gtk4::glib::ControlFlow::Continue
    });

    // --- UI connections ---
    // Connect sidebar row selection to show the appropriate content
    sidebar_list.connect_row_selected({
        let split_view = split_view.clone();
        let view_stack = view_stack.clone();

        move |_, row| {
            if let Some(row) = row {
                view_stack.set_visible_child_name("home");
                let name = match row.widget_name().as_str() {
                    "row_noise" => Some("page-noise"),
                    "row_touch" => Some("page-touch"),
                    "row_equalizer" => Some("page-equalizer"),
                    "row_find" => Some("page-find"),
                    _ => None,
                };

                if let Some(name) = name {
                    view_stack.set_visible_child_name(name);
                    split_view.set_show_content(true);
                } else {
                    split_view.set_show_content(false);
                }
            }
        }
    });

    // Connect splitview to content shows to listen when the back button is pressed
    split_view.connect_notify_local(Some("show-content"), {
        let sidebar_list = sidebar_list.clone();
        let view_stack = view_stack.clone();

        move |s, _| {
            if !s.shows_content() {
                view_stack.set_visible_child_name("home");
                sidebar_list.select_row(sidebar_list.row_at_index(0).as_ref());
            }
        }
    });

    // Connect pop and push to update the headerbar title
    view_stack.connect_visible_child_notify({
        let content_page = content_page.clone();
        move |s| {
            update_title(s, &content_page);
        }
    });

    window.present();
}

fn update_title(view_stack: &ViewStack, content_page: &NavigationPage) {
    if let Some(widget) = view_stack.visible_child() {
        let page = view_stack.page(&widget);
        if let Some(title) = page.title() {
            content_page.set_title(title.as_str());
        } else {
            content_page.set_title("");
        }
    }
}
