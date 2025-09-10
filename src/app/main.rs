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
        page_connection::{PageConnectionModel, PageConnectionOutput},
        page_manage::{PageManageInput, PageManageModel, PageManageOutput},
    },
    model::device_info::DeviceInfo,
    settings,
};

#[derive(Debug)]
pub enum Page {
    Connection(AsyncController<PageConnectionModel>),
    Manage(Controller<PageManageModel>),
    Init(adw::NavigationPage),
}

impl Page {
    pub fn widget(&self) -> &adw::NavigationPage {
        match self {
            Page::Connection(controller) => controller.widget(),
            Page::Manage(controller) => controller.widget(),
            Page::Init(page) => page,
        }
    }
}

#[derive(Debug)]
pub struct AppModel {
    active_page: Page,
    find_dialog: Controller<DialogFind>,
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

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::NavigationView {
                    #[watch]
                    replace: &[model.active_page.widget().to_owned()],
                }
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

        let model = AppModel {
            active_page: Page::Init(adw::NavigationPage::default()),
            find_dialog,
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
                self.active_page = Page::Manage(page);
            }
            AppInput::Disconnect => {
                let page = PageConnectionModel::builder().launch(()).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        PageConnectionOutput::SelectDevice(device) => {
                            AppInput::SelectDevice(device)
                        }
                    },
                );
                self.active_page = Page::Connection(page);
            }
            AppInput::FromPageManage(msg) => match msg {
                PageManageOutput::OpenFindDialog => self.find_dialog.emit(DialogFindInput::Show),
            },
            AppInput::FromDialogFind(msg) => {
                if let Page::Manage(page) = &self.active_page {
                    page.emit(PageManageInput::FindDialogCommand(msg));
                }
            }
        }
    }
}
