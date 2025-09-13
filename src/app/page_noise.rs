use adw::prelude::NavigationPageExt;
use galaxy_buds_rs::message::extended_status_updated::ExtendedStatusUpdate;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use tracing::debug;

#[derive(Debug)]
pub struct PageNoiseModel {}

#[derive(Debug)]
pub enum PageNoiseInput {
    StatusUpdate(ExtendedStatusUpdate),
}

#[derive(Debug)]
pub enum PageNoiseOutput {}

#[relm4::component(pub)]
impl SimpleComponent for PageNoiseModel {
    type Input = PageNoiseInput;
    type Output = PageNoiseOutput;
    type Init = ();

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

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PageNoiseInput::StatusUpdate(status) => {
                debug!("ambient_noise: {:?}", status.ambient_sound_enabled);
            }
        }
    }
}
