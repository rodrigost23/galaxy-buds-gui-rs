mod bluetooth;
mod ui;

use adw::prelude::*;
use gtk4::Application;

const APP_ID: &str = "com.github.rodrigost23.galaxy-buds-gui-rs";

fn main() {
    // Initialize Libadwaita and create the GTK Application.
    adw::init().expect("Failed to initialize Libadwaita.");
    let app = Application::builder().application_id(APP_ID).build();

    // Connect the "activate" signal to the build_ui function.
    app.connect_activate(|app| {
        let window = ui::build_window(app);
        window.present();
    });

    // Run the application.
    app.run();
}
