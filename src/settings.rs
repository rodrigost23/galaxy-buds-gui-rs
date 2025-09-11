use crate::consts::APP_ID;

#[cfg(debug_assertions)]
include!(concat!(env!("OUT_DIR"), "/settings_schema_path.rs"));

pub fn get_settings() -> gtk4::gio::Settings {
    #[cfg(debug_assertions)]
    {
        println!("Running in DEBUG mode. Loading schema from build directory.");

        let schema_source = gtk4::gio::SettingsSchemaSource::from_directory(
            GSETTINGS_SCHEMA_DIR,
            gtk4::gio::SettingsSchemaSource::default().as_ref(),
            false,
        )
        .expect("Could not create settings schema source in debug");

        let schema = schema_source
            .lookup(APP_ID, false)
            .expect("Schema not found in debug");

        gtk4::gio::Settings::new_full(&schema, None::<&gtk4::gio::SettingsBackend>, None)
    }
    #[cfg(not(debug_assertions))]
    {
        println!("Running in RELEASE mode. Loading schema from system path.");
        gtk4::gio::Settings::new(APP_ID)
    }
}
