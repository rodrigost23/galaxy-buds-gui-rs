use adw::{ViewStack, Window, prelude::*};
use gtk4::{Application, Builder, ListBox};

// A standard practice is to use a reverse-domain name for the app ID.
const APP_ID: &str = "com.example.adw-settings-app";

fn main() {
    // Initialize Libadwaita and create the GTK Application.
    adw::init().expect("Failed to initialize Libadwaita.");
    let app = Application::builder().application_id(APP_ID).build();

    // Connect the "activate" signal to the build_ui function.
    app.connect_activate(build_ui);

    // Run the application.
    app.run();
}

fn build_ui(app: &Application) {
    // Build the UI from the XML file.
    let builder = Builder::from_string(include_str!("../data/ui/main.ui"));

    // Get the widgets we need to interact with.
    let window: Window = builder
        .object("main_window")
        .expect("Could not get main_window");
    let sidebar_list: ListBox = builder
        .object("sidebar_list")
        .expect("Could not get sidebar_list");
    let view_stack: ViewStack = builder
        .object("view_stack")
        .expect("Could not get view_stack");

    window.set_application(Some(app));

    // Define the names of the pages in our ViewStack, matching the .ui file.
    let page_names = ["page_summary", "page_option_1"];

    // Connect sidebar row selection to view switching in the ViewStack.
    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(selected_row) = row {
            let index = selected_row.index() as usize;

            // Get the page name corresponding to the selected row's index.
            if let Some(page_name) = page_names.get(index) {
                view_stack.set_visible_child_name(page_name);
            }
        }
    });

    window.present();
}
