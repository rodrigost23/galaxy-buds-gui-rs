use adw::{NavigationPage, NavigationSplitView, ViewStack, Window, prelude::*};
use gtk4::{Application, Builder, ListBox};
use std::sync::mpsc;
use std::thread;
use tokio::runtime::Runtime;

use crate::bluetooth::bluetooth_loop;

pub fn build_window(app: &Application) -> Window {
    let builder = Builder::from_string(include_str!("gtk/main.ui"));

    let window: Window = builder.object("main_window").unwrap();
    let content_page: NavigationPage = builder.object("content_page").unwrap();
    let sidebar_list: ListBox = builder.object("sidebar_list").unwrap();
    let split_view: NavigationSplitView = builder.object("split_view").unwrap();
    let view_stack: ViewStack = builder.object("view_stack").unwrap();

    window.set_application(Some(app));
    sidebar_list.unselect_all();

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

    window
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
