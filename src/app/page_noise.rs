use adw::prelude::{ActionRowExt, NavigationPageExt, PreferencesGroupExt, PreferencesRowExt};
use galaxy_buds_rs::message::bud_property::NoiseControlMode;
use gtk4::prelude::CheckButtonExt;
use relm4::{ComponentParts, ComponentSender, SimpleComponent};
use tracing::debug;

#[derive(Debug)]
pub struct PageNoiseModel {
    mode: NoiseControlMode,
}

#[derive(Debug)]
pub enum PageNoiseInput {
    ModeUpdate(NoiseControlMode),
}

#[derive(Debug)]
pub enum PageNoiseOutput {}

#[relm4::component(pub)]
impl SimpleComponent for PageNoiseModel {
    type Input = PageNoiseInput;
    type Output = PageNoiseOutput;
    type Init = NoiseControlMode;

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
                    adw::PreferencesPage {
                        adw::PreferencesGroup {
                            set_title: "Noise Control",

                            adw::ActionRow {
                                set_title: "Off",
                                #[name = "check_off"]
                                add_prefix = &gtk4::CheckButton::new() {
                                    #[watch]
                                    set_active: model.mode == NoiseControlMode::Off,
                                },
                                set_activatable_widget: Some(&check_off),
                            },
                            adw::ActionRow {
                                set_title: "Ambient sound",
                                #[name = "check_ambient"]
                                add_prefix = &gtk4::CheckButton::new() {
                                    set_group: Some(&check_off),
                                    #[watch]
                                    set_active: model.mode == NoiseControlMode::AmbientSound,
                                },
                                set_activatable_widget: Some(&check_ambient),
                            },
                            adw::ActionRow {
                                set_title: "Noise reduction",
                                #[name = "check_noise"]
                                add_prefix = &gtk4::CheckButton::new() {
                                    set_group: Some(&check_ambient),
                                    #[watch]
                                    set_active: model.mode == NoiseControlMode::NoiseReduction
                                },
                                set_activatable_widget: Some(&check_noise),
                            }
                        }
                    }
                }
            },
        }
    }

    fn init(
        mode: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = PageNoiseModel { mode };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PageNoiseInput::ModeUpdate(mode) => {
                debug!("Mode update: {:?}", mode);
                self.mode = mode;
            }
        }
    }
}
