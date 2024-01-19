#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use anyhow::Context;
use eframe::Renderer;
use egui::ViewportBuilder;
use log::{error, LevelFilter};
use scene::Scene;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

mod raytracer;
mod scene;
mod ui;

fn main() -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::Trace,
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            // suppress all logs from dependencies
            .add_filter_allow_str("raytracing")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")?;

    let viewport = ViewportBuilder::default()
        .with_title("Trayracer")
        .with_app_id("raytracer")
        .with_inner_size(egui::vec2(1600.0, 900.0))
        .with_icon(
            eframe::icon_data::from_png_bytes(include_bytes!("../res/icon.png"))
                .unwrap_or_default(),
        );

    eframe::run_native(
        "TrayRacer",
        eframe::NativeOptions {
            viewport,
            renderer: Renderer::Wgpu,
            depth_buffer: 32,
            follow_system_theme: true,
            centered: true,
            ..Default::default()
        },
        Box::new(|cc| {
            Box::new(ui::App::new(cc).unwrap_or_else(|e| {
                error!("Failed to create app: {}", e);
                std::process::exit(1);
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
    .context("Failed to run native")
}
