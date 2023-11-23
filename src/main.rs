#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use anyhow::Context;
use eframe::{IconData, Renderer};
use log::{error, LevelFilter};
use nalgebra::Vector3;
use scene::Scene;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

mod raytracer;
mod scene;
mod ui;

type Color = Vector3<f32>;

fn main() -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        #[cfg(debug_assertions)]
        LevelFilter::Trace,
        #[cfg(not(debug_assertions))]
        LevelFilter::Info,
        ConfigBuilder::new()
            .add_filter_allow_str("raytracing")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")?;

    let scene = Scene::load("./res/config.yaml").context("Failed to load scene")?;

    eframe::run_native(
        "RayTracer",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(1200.0, 900.0)),
            renderer: Renderer::Wgpu,
            depth_buffer: 32,
            icon_data: IconData::try_from_png_bytes(include_bytes!("../res/icon.png")).ok(),
            app_id: Some("raytracer".to_owned()),
            follow_system_theme: true,
            centered: true,
            ..Default::default()
        },
        Box::new(|cc| {
            Box::new(ui::App::new(cc, scene).unwrap_or_else(|e| {
                error!("Failed to create app: {}", e);
                std::process::exit(1);
            }))
        }),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
    .context("Failed to run native")
}
