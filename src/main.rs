#![expect(
    deprecated,
    reason = "ImageButton is deprecated but doesn't have all the new features yet"
)]
use std::process;

use anyhow::Context;
use eframe::{Renderer, icon_data};
use egui::ViewportBuilder;
use log::{LevelFilter, error, info};
use rust_i18n::i18n;
use scene::Scene;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};
use sys_locale::get_locale;

mod raytracer;
mod scene;
mod ui;
i18n!("locales", fallback = "en");

fn main() -> anyhow::Result<()> {
    rust_i18n::set_locale(
        get_locale()
            .unwrap_or_else(|| String::from("en-US"))
            .as_str(),
    );
    simplelog::TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::Trace,
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            // suppress all logs from dependencies
            .add_filter_allow_str("trayracer")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")?;
    info!(
        "available translations: {:?}",
        rust_i18n::available_locales!()
    );
    let viewport = ViewportBuilder::default()
        .with_title("Trayracer")
        .with_app_id("raytracer")
        .with_inner_size(egui::vec2(1600.0, 900.0))
        .with_icon(
            icon_data::from_png_bytes(include_bytes!("../res/icon.png")).unwrap_or_default(),
        );

    eframe::run_native(
        "TrayRacer",
        eframe::NativeOptions {
            viewport,
            renderer: Renderer::Wgpu,
            depth_buffer: 32,
            centered: true,
            ..Default::default()
        },
        Box::new(|cc| {
            Ok(Box::new(ui::App::new(cc).unwrap_or_else(|e| {
                error!("Failed to create app: {e}");
                process::exit(1);
            })))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
    .context("Failed to run native")
}
