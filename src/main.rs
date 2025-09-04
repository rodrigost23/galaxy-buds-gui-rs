use adw::{NavigationPage, NavigationSplitView, ViewStack, Window, prelude::*};
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
    let content_page: NavigationPage = builder
        .object("content_page")
        .expect("Could not get content_page");
    let sidebar_list: ListBox = builder
        .object("sidebar_list")
        .expect("Could not get sidebar_list");
    let split_view: NavigationSplitView = builder
        .object("split_view")
        .expect("Could not get split_view");
    let view_stack: ViewStack = builder
        .object("view_stack")
        .expect("Could not get view_stack");

    window.set_application(Some(app));
    sidebar_list.unselect_all();

    // Connect sidebar row selection to show the appropriate content
    sidebar_list.connect_row_selected({
        let split_view = split_view.clone();
        let view_stack = view_stack.clone();

        move |_, row| {
            if let Some(row) = row {
                view_stack.set_visible_child_name("home");

                let name = match row.index() {
                    1 => Some("page-noise"),
                    2 => Some("page-touch"),
                    3 => Some("page-equalizer"),
                    4 => Some("page-find"),
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
