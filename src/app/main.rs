use gtk4::prelude::GtkWindowExt;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, SimpleComponent,
    prelude::{AsyncComponent, AsyncComponentController, AsyncController},
};

use crate::{
    app::{
        page_connection::{PageConnectionModel, PageConnectionOutput},
        page_manage::PageManageModel,
    },
    model::device_info::DeviceInfo,
};
pub enum Page {
    Connection(AsyncController<PageConnectionModel>),
    Manage(Controller<PageManageModel>),
}

impl Page {
    pub fn widget(&self) -> &adw::NavigationPage {
        match self {
            Page::Connection(controller) => controller.widget(),
            Page::Manage(controller) => controller.widget(),
        }
    }
}

pub struct AppModel {
    // pages: AppPages,
    active_page: Page,
}

#[derive(Debug)]
pub enum AppInput {
    SelectDevice(DeviceInfo),
    Disconnect,
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
            set_default_width: 800,
            set_default_height: 800,

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::NavigationView {
                    #[watch]
                    replace: &[model.active_page.widget().to_owned()],
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        window: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let page_connection = PageConnectionModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                PageConnectionOutput::SelectDevice(device) => AppInput::SelectDevice(device),
            },
        );
        let model = AppModel {
            active_page: Page::Connection(page_connection),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppInput::SelectDevice(device) => {
                println!("AppInput::SelectDevice");
                let page = PageManageModel::builder()
                    .launch(device)
                    .forward(sender.input_sender(), |msg| match msg {});
                self.active_page = Page::Manage(page);
            }
            AppInput::Disconnect => todo!(),
        }
    }
}
