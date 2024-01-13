mod preview;
mod properties;
mod render;
mod renderresult;
mod status;

use self::preview::Preview;
use self::render::Render;
use self::renderresult::RenderResult;
use self::status::Status;

use crate::scene::Scene;
use crate::ui::properties::Properties;
use anyhow::Context;
use eframe::CreationContext;
use egui::mutex::Mutex;
use egui::{vec2, CentralPanel, Color32, ColorImage, ImageData, Key, TextStyle, TextureOptions};
use image::ImageBuffer;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct App {
    current_tab: usize,
    scene: Scene,
    render: Render,
    properties: Properties,
    line: Status,
    preview: Preview,
    render_result: RenderResult,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        Preview::init(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );

        let (render_texture, image_buffer) = {
            let render_size = scene.camera.resolution;
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
            s.spacing.item_spacing = vec2(10.0, std::f32::consts::PI * 1.76643);
        });

        Ok(Self {
            scene,
            current_tab: 0,
            render: Render::new(render_texture, image_buffer),
            properties: Properties::new(),
            line: Status::new(),
            preview: Preview::new(),
            render_result: RenderResult::new(),
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
            .then(|| Properties::save_scene(self.scene.clone()));

        CentralPanel::default().show(ctx, |ui| {
            self.line
                .show(ui, &mut self.scene, &mut self.render, &mut self.current_tab);

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                0 => {
                    self.properties.show(&mut self.scene, ui, &mut self.render);
                    self.preview.show(ui, &mut self.scene);
                }
                1 => self.render_result.show(ui, &self.scene, &self.render),
                n => unreachable!("Invalid tab index {}", n),
            }
        });
    }
}
