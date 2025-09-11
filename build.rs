use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Get the output directory from Cargo
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Tell Cargo to re-run if the schema file changes
    println!("cargo:rerun-if-changed=data/com.github.rodrigost23.GalaxyBudsGui.gschema.xml");

    // Compile the schema into the OUT_DIR
    let status = Command::new("glib-compile-schemas")
        .arg("--strict")
        .arg("--targetdir")
        .arg(&out_dir)
        .arg("data")
        .status()
        .expect("Failed to execute glib-compile-schemas");

    if !status.success() {
        panic!("glib-compile-schemas failed");
    }

    let generated_file_path = out_dir.join("settings_schema_path.rs");
    let content = format!(
        "pub const GSETTINGS_SCHEMA_DIR: &str = \"{}\";",
        out_dir.display()
    );
    fs::write(generated_file_path, content).expect("Failed to write generated settings path file");
}
