use adw::{NavigationPage, NavigationSplitView, NavigationView, Window, prelude::*};
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
    let nav_view: NavigationView = builder.object("nav_view").expect("Could not get nav_view");

    window.set_application(Some(app));
    sidebar_list.unselect_all();

    // Connect sidebar row selection to show the appropriate content
    sidebar_list.connect_row_selected({
        let split_view = split_view.clone();
        let nav_view = nav_view.clone();

        move |_, row| {
            if let Some(row) = row {
                nav_view.pop_to_tag("home");

                let tag = match row.index() {
                    1 => Some("page-noise"),
                    2 => Some("page-touch"),
                    3 => Some("page-equalizer"),
                    4 => Some("page-find"),
                    _ => None,
                };

                if let Some(tag) = tag {
                    nav_view.replace_with_tags(&["home", tag]);
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
        let nav_view = nav_view.clone();

        move |s, _| {
            if !s.shows_content() {
                nav_view.pop_to_tag("home");
                sidebar_list.select_row(sidebar_list.row_at_index(0).as_ref());
            }
        }
    });

    // Connect pop and push to update the headerbar title
    nav_view.connect_pushed({
        let content_page = content_page.clone();
        move |n| {
            update_title(n, &content_page);
        }
    });

    nav_view.connect_popped({
        let content_page = content_page.clone();
        move |n, _| {
            update_title(n, &content_page);
        }
    });

    window.present();
}

fn update_title(nav_view: &NavigationView, content_page: &NavigationPage) {
    if let Some(page) = nav_view.visible_page() {
        let title = page.title();
        content_page.set_title(title.as_str());
    }
}
