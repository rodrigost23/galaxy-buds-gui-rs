use adw::gio::prelude::SettingsExt;
use gtk4::gio::prelude::SettingsExtManual;
use gtk4::prelude::GtkWindowExt;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    prelude::{AsyncComponent, AsyncComponentController, AsyncController},
};
use tracing::debug;

use crate::{
    app::{
        dialog_find::{DialogFind, DialogFindInput, DialogFindOutput},
        page_connection::{PageConnectionInput, PageConnectionModel, PageConnectionOutput},
        page_manage::{PageManageInput, PageManageModel, PageManageOutput},
    },
    consts::DEVICE_ADDRESS_KEY,
    model::device_info::DeviceInfo,
    settings,
};

macro_rules! pages {
    ($($page_name:ident($controller_type:ty)),+ $(,)?) => {
        #[derive(Debug)]
        pub enum Page {
            // For each matched entry, create an enum page_name.
            $($page_name($controller_type)),+
        }

        impl Page {
            pub fn widget(&self) -> &adw::NavigationPage {
                match self {
                    // For each matched page_name, create a match arm that calls `.widget()`.
                    $(Page::$page_name(controller) => controller.widget()),+
                }
            }
        }
    };
}

pages! {
    Connection(AsyncController<PageConnectionModel>),
    Manage(Controller<PageManageModel>),
}

#[derive(Debug)]
pub struct AppModel {
    active_page: Option<Page>,
    find_dialog: Controller<DialogFind>,
    settings: adw::gio::Settings,
    connect_page: AsyncController<PageConnectionModel>,
}

#[derive(Debug)]
pub enum AppInput {
    SelectDevice(DeviceInfo),
    Disconnect,
    FromPageManage(PageManageOutput),
    FromDialogFind(DialogFindOutput),
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
            set_title: Some("Galaxy Buds Manager"),

            #[name = "nav_view"]
            adw::NavigationView {
                add: &connect_page_widget,
            },
        }
    }

    fn init(
        _init: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let settings = settings::get_settings();

        // -> Add these two lines to bind the window size
        settings
            .bind("window-width", &window, "default-width")
            .flags(gtk4::gio::SettingsBindFlags::DEFAULT)
            .build();

        settings
            .bind("window-height", &window, "default-height")
            .flags(gtk4::gio::SettingsBindFlags::DEFAULT)
            .build();

        let find_dialog = DialogFind::builder()
            .launch(window.clone())
            .forward(sender.input_sender(), AppInput::FromDialogFind);

        let connect_page = PageConnectionModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                PageConnectionOutput::SelectDevice(device) => AppInput::SelectDevice(device),
            },
        );

        let connect_page_widget = connect_page.widget().clone();

        let model = AppModel {
            active_page: None,
            connect_page,
            find_dialog,
            settings,
        };

        let widgets = view_output!();

        sender.input(AppInput::Disconnect);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppInput::SelectDevice(device) => {
                debug!("{:?}", device);
                let page = PageManageModel::builder()
                    .launch(device)
                    .forward(sender.input_sender(), AppInput::FromPageManage);
                self.active_page = Some(Page::Manage(page));
            }
            AppInput::Disconnect => {
                self.active_page = None;
            }
            AppInput::FromPageManage(msg) => match msg {
                PageManageOutput::OpenFindDialog => self.find_dialog.emit(DialogFindInput::Show),
                PageManageOutput::Disconnect => {
                    let _ = self.settings.set_string(DEVICE_ADDRESS_KEY, "");
                    sender.input(AppInput::Disconnect)
                }
            },
            AppInput::FromDialogFind(msg) => {
                if let Some(Page::Manage(page)) = &self.active_page {
                    page.emit(PageManageInput::FindDialogCommand(msg));
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        match &self.active_page {
            Some(page) => {
                widgets.nav_view.push(page.widget());
            }
            None => {
                widgets.nav_view.pop_to_page(self.connect_page.widget());
                self.connect_page
                    .sender()
                    .send(PageConnectionInput::LoadDevices)
                    .unwrap()
            }
        }
    }
}
