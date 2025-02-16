#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use log4rs::config::Deserializers;
use manga_overlay::OcrApp;
use std::{fs, path::Path};

#[tokio::main]
async fn main() -> eframe::Result {
    init_logger();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_always_on_top()
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../resources/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "Manga Overlay",
        native_options,
        Box::new(|cc| Ok(Box::new(OcrApp::new(cc)))),
    )
}

const LOG_CONFIG_DIR: &str = "config";
const LOG_CONFIG: &str = "config/log4rs.yaml";

fn init_logger() {
    fs::create_dir_all(LOG_CONFIG_DIR).expect("Config directory creation failed");
    if !Path::new(&LOG_CONFIG).exists() {
        fs::write(LOG_CONFIG, include_str!("../config/log4rs.yaml"))
            .expect("Config file creation failed");
    }

    log4rs::init_file("config/log4rs.yaml", Deserializers::default()).expect("Logger init failed");
}
