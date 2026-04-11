#![expect(
    deprecated,
    reason = "ImageButton is deprecated but doesn't have all the new features yet"
)]
use std::process;
use std::sync::Arc;

use anyhow::Context;
use eframe::Renderer;
use egui::ViewportBuilder;
use egui_wgpu::WgpuSetup;
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
            eframe::icon_data::from_png_bytes(include_bytes!("../res/icon.png"))
                .unwrap_or_default(),
        );

    let mut native_options = eframe::NativeOptions {
        viewport,
        renderer: Renderer::Wgpu,
        depth_buffer: 32,
        centered: true,
        ..Default::default()
    };

    if let WgpuSetup::CreateNew(ref mut create_new) = native_options.wgpu_options.wgpu_setup {
        let prev = Arc::clone(&create_new.device_descriptor);
        create_new.device_descriptor = Arc::new(move |adapter| {
            let mut desc = prev(adapter);
            let al = adapter.limits();
            // Default WebGPU-style cap is 128 MiB; GPU raytrace triangle storage can exceed that.
            let want: u64 = 512 << 20;
            desc.required_limits.max_storage_buffer_binding_size = desc
                .required_limits
                .max_storage_buffer_binding_size
                .max(want.min(al.max_storage_buffer_binding_size));
            desc.required_limits.max_buffer_size = desc
                .required_limits
                .max_buffer_size
                .max(want.min(al.max_buffer_size));
            desc
        });
    }

    eframe::run_native(
        "TrayRacer",
        native_options,
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
