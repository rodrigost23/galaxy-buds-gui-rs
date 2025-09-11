mod app;
mod buds_worker;
mod consts;
mod model;
mod settings;

use crate::app::main::{AppInit, AppModel};
use relm4::RelmApp;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .unwrap()
        .add_directive("relm4=error".parse().unwrap());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();

    let app = RelmApp::new(consts::APP_ID);
    app.run::<AppModel>(AppInit {});
}
