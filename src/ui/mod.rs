mod line;
mod preview;
mod properties;
mod render;
mod renderresult;

use self::line::Line;
use self::preview::Preview;
use self::render::Render;

use crate::scene::Scene;
use crate::ui::properties::Properties;
use anyhow::Context;
use eframe::CreationContext;
use egui::mutex::Mutex;
use egui::{CentralPanel, Color32, ColorImage, ImageData, Key, TextStyle, TextureOptions};
use image::ImageBuffer;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(PartialEq, Eq)]
enum RenderSize {
    FullHD,
    Wqhd,
    Uhd1,
    Uhd2,
    Custom([u32; 2]),
}

impl RenderSize {
    fn as_size(&self) -> (u32, u32) {
        match self {
            RenderSize::FullHD => (1920, 1080),
            RenderSize::Wqhd => (2560, 1440),
            RenderSize::Uhd1 => (3840, 2160),
            RenderSize::Uhd2 => (7680, 4320),
            &RenderSize::Custom([x, y]) => (x, y),
        }
    }

    fn from_res(res: (u32, u32)) -> Self {
        match res {
            (1920, 1080) => RenderSize::FullHD,
            (2560, 1440) => RenderSize::Wqhd,
            (3840, 2160) => RenderSize::Uhd1,
            (7680, 4320) => RenderSize::Uhd2,
            (x, y) => RenderSize::Custom([x, y]),
        }
    }
}

impl std::fmt::Display for RenderSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderSize::FullHD => write!(f, "Full HD"),
            RenderSize::Wqhd => write!(f, "2k"),
            RenderSize::Uhd1 => write!(f, "4k"),
            RenderSize::Uhd2 => write!(f, "8k"),
            RenderSize::Custom(_) => write!(f, "Custom"),
        }
    }
}
pub struct App {
    current_tab: usize,
    scene: Scene,
    render: Render,
    properties: Properties,
    line: Line,
    preview: Preview,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        Preview::init(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );
        let render_size = RenderSize::from_res(scene.camera.resolution);

        let (render_texture, image_buffer) = {
            let render_size = render_size.as_size();
            let texture = cc.egui_ctx.load_texture(
                "render",
                ImageData::Color(Arc::new(ColorImage {
                    size: [render_size.0 as usize, render_size.1 as usize],
                    pixels: vec![Color32::BLACK; (render_size.0 * render_size.1) as usize],
                })),
                TextureOptions::default(),
            );
            let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(render_size.0, render_size.1)));
            (texture, image_buffer)
        };

        cc.egui_ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Name("subheading".into()),
                TextStyle::Monospace.resolve(s),
            );
        });

        Ok(Self {
            scene,
            current_tab: 0,
            render: Render::new(render_texture.clone(), image_buffer, render_size),
            properties: Properties::new(),
            line: Line::new(),
            preview: Preview::new(),
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render
            .thread
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
            .then(|| {
                self.render.thread = None;
                self.render.cancel.store(false, Ordering::Relaxed);
            });

        ctx.input(|input| input.key_pressed(Key::S) && input.modifiers.ctrl)
            .then(|| self.properties.save_scene(self.scene.clone()));

        CentralPanel::default().show(ctx, |ui| {
            self.line
                .show(ui, &mut self.scene, &mut self.render, &mut self.current_tab);

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                0 => {
                    self.properties
                        .properties(&mut self.scene, ui, &mut self.render);
                    self.preview.preview(ui, &mut self.scene, &self.render);
                }
                1 => renderresult::RenderResult::render_result(ui, &self.render, &mut self.preview),
                n => unreachable!("Invalid tab index {}", n),
            }
        });
    }
}
