use adw::gio::prelude::SettingsExt;
use gtk4::gio::prelude::SettingsExtManual;
use gtk4::prelude::GtkWindowExt;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    prelude::{AsyncComponent, AsyncComponentController, AsyncController},
};
use tracing::{debug, debug_span};

use crate::{
    app::{
        dialog_find::{DialogFind, DialogFindInput, DialogFindOutput},
        page_connection::{PageConnectionInput, PageConnectionModel, PageConnectionOutput},
        page_manage::{PageManageInput, PageManageModel, PageManageOutput},
    },
    consts::DEVICE_ADDRESS_KEY,
    define_page_enum,
    model::device_info::DeviceInfo,
    settings,
};

define_page_enum!(Page {
    Connection(AsyncController<PageConnectionModel>),
    Manage(Controller<PageManageModel>),
});

#[derive(Debug)]
pub struct AppModel {
    active_page: Option<Page>,
    find_dialog: Controller<DialogFind>,
    settings: adw::gio::Settings,
    connect_page: AsyncController<PageConnectionModel>,
    active_subpage: Option<adw::NavigationPage>,
}

#[derive(Debug)]
pub enum AppInput {
    SelectDevice(DeviceInfo),
    Disconnect,
    FromPageManage(PageManageOutput),
    FromDialogFind(DialogFindOutput),
    PagePopped(adw::NavigationPage),
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
                connect_popped[sender] => move |_, page| {
                    sender.input(AppInput::PagePopped(page.clone()));
                },
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
            active_subpage: None,
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
                PageManageOutput::Navigate(page) => {
                    self.active_subpage = Some(page);
                }
            },
            AppInput::FromDialogFind(msg) => {
                if let Some(Page::Manage(page)) = &self.active_page {
                    page.emit(PageManageInput::FindDialogCommand(msg));
                }
            }
            AppInput::PagePopped(popped_page) => {
                if let Some(subpage) = &self.active_subpage {
                    if popped_page == subpage.clone() {
                        self.active_subpage = None;
                    }
                }

                if let Some(active_page) = &self.active_page {
                    if popped_page == active_page.widget().clone() {
                        self.active_page = None;
                    }
                }
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        match &self.active_page {
            Some(page_to_push) => {
                let mut is_visible = false;

                if let Some(visible_page) = widgets.nav_view.visible_page() {
                    if &visible_page == page_to_push.widget() {
                        is_visible = true;
                    }
                }

                if !is_visible {
                    widgets.nav_view.push(page_to_push.widget());
                }

                if let Some(subpage) = &self.active_subpage {
                    widgets.nav_view.push(subpage);
                }
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
