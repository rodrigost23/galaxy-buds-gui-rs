mod app;
mod bluetooth_worker;

use relm4::RelmApp;

use crate::app::AppInit;

const APP_ID: &str = "com.github.rodrigost23.galaxy-buds-gui-rs";

fn main() {
    // Relm4 handles the initialization of Libadwaita and the GTK Application.
    let app = RelmApp::new(APP_ID);
    // Run the main application component.
    app.run::<app::AppModel>(AppInit {});
}
