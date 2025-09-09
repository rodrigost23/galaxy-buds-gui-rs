use adw::prelude::{AdwDialogExt, AlertDialogExt};
use gtk4::prelude::{ToggleButtonExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, SimpleComponent, gtk};

pub struct DialogFind {
    parent: adw::ApplicationWindow,
    is_visible: bool,
}

#[derive(Debug)]
pub enum DialogFindInput {
    Show,
    Toggle(bool),
}

#[derive(Debug)]
pub enum DialogFindOutput {
    Start,
    Stop,
}

#[relm4::component(pub)]
impl SimpleComponent for DialogFind {
    type Input = DialogFindInput;
    type Output = DialogFindOutput;
    type Init = adw::ApplicationWindow;

    view! {
        #[root]
        #[name="root"]
        adw::AlertDialog {
            set_heading: Some("Find my Buds"),
            set_body: "Your Galaxy Buds will make a loud noise when you press Start.\nMake sure not to be wearing them.",
            add_response: ("close", "Close"),
            set_close_response: "close",

            #[wrap(Some)]
            #[name="toggle"]
            set_extra_child = &gtk4::ToggleButton {
                add_css_class: "suggested-action",
                connect_toggled[sender] => move |btn| {
                    sender.input(DialogFindInput::Toggle(btn.is_active()))
                },

                adw::ButtonContent {
                    #[watch]
                    set_label: if toggle.is_active() { "Start" }  else { "Stop" }
                }
            },
        }
    }

    fn init(
        parent: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = DialogFind {
            parent,
            is_visible: true,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            DialogFindInput::Show => {
                self.is_visible = true;
            }
            DialogFindInput::Toggle(active) => sender
                .output(if active {
                    DialogFindOutput::Start
                } else {
                    DialogFindOutput::Stop
                })
                .unwrap(),
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        if self.is_visible {
            widgets.root.present(Some(&self.parent));
        } else {
            widgets.root.close();
        }
    }
}
