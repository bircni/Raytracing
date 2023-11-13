use anyhow::Context;
use eframe::Renderer;
use log::LevelFilter;
use nalgebra::Vector3;
use scene::Scene;
use simplelog::{ColorChoice, ConfigBuilder, TerminalMode};

mod raytracer;
mod scene;
mod ui;

type Color = Vector3<f32>;

fn main() -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        LevelFilter::Trace,
        ConfigBuilder::new()
            .add_filter_allow_str("raytracing")
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .context("Failed to initialize logger")?;

    let scene = Scene::load("./res/config.yaml").context("Failed to load scene")?;

    eframe::run_native(
        "RayTracer!",
        eframe::NativeOptions {
            initial_window_size: Some(egui::vec2(1200.0, 900.0)),
            renderer: Renderer::Glow,
            depth_buffer: 1,
            ..Default::default()
        },
        Box::new(|cc| Box::new(ui::App::new(cc, scene).expect("Failed to create app"))),
    )
    .map_err(|e| anyhow::anyhow!(e.to_string()))
    .context("Failed to run native")
}
