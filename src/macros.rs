/// Defines an enum that can hold different component types
/// and automatically implements a `widget()` method that delegates
/// the call to the inner component.
#[macro_export]
macro_rules! define_page_enum {
    // This pattern expects `IdName, Name { Page(Type), ... }` inside the macro's parentheses.
    // where IdName is the enum with only the identifiers, and Name the main enum.
    // TODO: Change this when [`macro_metavar_expr_concat`](https://github.com/rust-lang/rust/issues/124225) is stable
    (
        $identifier_enum:ident,
        $enum_name:ident { $($page_name:ident($controller_type:ty)),+ $(,)? }$(,)?
    ) => {
        define_page_enum!($enum_name { $($page_name($controller_type)),+ });

        #[derive(Debug)]
        pub enum $identifier_enum {
            $($page_name),+
        }

    };
    // This pattern expects `Name { Page(Type), ... }` inside the macro's parentheses.
    ($enum_name:ident { $($page_name:ident($controller_type:ty)),+ $(,)? }) => {
        #[derive(Debug)]
        pub enum $enum_name {
            // For each matched entry, create an enum page_name.
            $($page_name($controller_type)),+
        }

        impl $enum_name {
            pub fn widget(&self) -> &adw::NavigationPage {
                match self {
                    // For each matched page_name, create a match arm that calls `.widget()`.
                    $($enum_name::$page_name(controller) => controller.widget()),+
                }
            }
        }
    };
}
