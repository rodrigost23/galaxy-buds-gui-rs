use relm4::{ComponentParts, ComponentSender, SimpleComponent, adw::prelude::*};

pub struct AppModel {}

pub struct AppWidgets {}

#[derive(Debug)]
pub enum ComponentInput {}

#[derive(Debug)]
pub enum ComponentOutput {}

pub struct ComponentInit {}

// #[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Input = ComponentInput;
    type Output = ComponentOutput;
    type Init = ComponentInit;
    type Root = adw::ApplicationWindow;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        let builder = gtk4::Builder::from_string(include_str!("gtk/main.ui"));

        let window: adw::ApplicationWindow = builder.object("main_window").unwrap();
        let content_page: adw::NavigationPage = builder.object("content_page").unwrap();
        let sidebar_list: gtk4::ListBox = builder.object("sidebar_list").unwrap();
        let split_view: adw::NavigationSplitView = builder.object("split_view").unwrap();
        let view_stack: adw::ViewStack = builder.object("view_stack").unwrap();

        window
    }
    // view! {
    //     #[root]
    //     adw::ApplicationWindow {
    //         set_default_width: 800,
    //         set_default_height: 800,

    //         // This breakpoint will trigger when the window is 540sp or narrower.
    //         add_breakpoint = adw::Breakpoint::new(
    //             adw::BreakpointCondition::new_length(
    //                 adw::BreakpointConditionLengthType::MaxWidth,
    //                 540.0,
    //                 adw::LengthUnit::Sp,
    //             )
    //         ) {
    //             // When the breakpoint is active, it sets the "collapsed" property
    //             // of the `split_view` widget (defined below) to `true`.
    //             add_setter: (
    //                 &split_view,
    //                 "collapsed",
    //                 Some(&true.into())
    //             )
    //         },

    //         // The main layout widget for the window.
    //         #[name="split_view"]
    //         adw::NavigationSplitView {
    //             // The sidebar of the split view
    //             #[wrap(Some)]
    //             set_sidebar = &adw::NavigationPage {
    //                 set_title: "Galaxy Buds Manager",
    //                 #[wrap(Some)]
    //                 set_child = &adw::ToolbarView {
    //                     add_top_bar = &adw::HeaderBar,
    //                 }
    //             },

    //             // The main content area of the split view
    //             #[wrap(Some)]
    //             set_content = &adw::NavigationPage {
    //                 #[wrap(Some)]
    //                 set_child = &adw::ToolbarView {
    //                     add_top_bar = &adw::HeaderBar,
    //                 }
    //             },
    //         }
    //     }
    // }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {};
        // let widgets = view_output!();

        let widgets = AppWidgets { };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {}
    }
}
