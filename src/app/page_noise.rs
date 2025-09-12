use adw::prelude::NavigationPageExt;
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk};

pub struct PageNoiseModel {}

#[derive(Debug)]
pub enum PageNoiseInput {}

#[derive(Debug)]
pub enum PageNoiseOutput {}

pub struct PageNoiseInit {}

#[relm4::component(pub)]
impl SimpleComponent for PageNoiseModel {
    type Input = PageNoiseInput;
    type Output = PageNoiseOutput;
    type Init = PageNoiseInit;

    view! {
        #[root]
        adw::NavigationPage {
            set_title: "Noise Control",

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {},
                add_top_bar = &adw::Banner {},

                #[wrap(Some)]
                set_content = &adw::Clamp {
                }
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PageNoiseModel {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {}
    }
}
