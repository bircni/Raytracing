mod preview;
mod properties;
mod render;
mod renderresult;
mod status;
mod yamlmenu;

use self::preview::Preview;
use self::render::Render;
use self::renderresult::RenderResult;
use self::status::Status;
use self::yamlmenu::YamlMenu;

use crate::ui::properties::Properties;
use anyhow::Context;
use eframe::CreationContext;
use egui::mutex::Mutex;
use egui::{
    vec2, Align, CentralPanel, ColorImage, Direction, ImageData, Layout, ScrollArea, SidePanel,
    TextStyle, TextureOptions,
};
use image::ImageBuffer;

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct App {
    current_tab: Tab,
    render: Render,
    properties: Properties,
    status: Status,
    preview: Preview,
    render_result: RenderResult,
    yaml_menu: YamlMenu,
}

/// Main application
/// This holds all the UI elements and manages the application state
impl App {
    pub fn new(cc: &CreationContext) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Initialize the preview renderer with the wgpu context
        Preview::init(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );

        // create initial render texture
        let (render_texture, image_buffer) = {
            let texture = cc.egui_ctx.load_texture(
                "render",
                ImageData::Color(Arc::new(ColorImage::example())),
                TextureOptions::default(),
            );
            let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(0, 0)));
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
            current_tab: Tab::Preview,
            render: Render::new(render_texture, image_buffer),
            properties: Properties::new(),
            status: Status::new(),
            preview: Preview::new(),
            render_result: RenderResult::new(),
            yaml_menu: YamlMenu::new(),
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

        CentralPanel::default().show(ctx, |ui| {
            self.status.show(
                ui,
                self.yaml_menu.scene_mut(),
                &mut self.render,
                &mut self.current_tab,
            );

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                Tab::Preview => {
                    SidePanel::right("panel")
                        .show_separator_line(true)
                        .show_inside(ui, |ui| {
                            ScrollArea::new([false, true]).show(ui, |ui| {
                                self.yaml_menu.show(ui);

                                ui.separator();

                                if let Some(scene) = self.yaml_menu.scene_mut() {
                                    self.properties.show(scene, ui, &mut self.render);
                                }
                            });
                        });

                    match self.yaml_menu.scene_mut() {
                        Some(scene) => self.preview.show(ui, scene),
                        None => {
                            ui.with_layout(
                                Layout::centered_and_justified(Direction::LeftToRight)
                                    .with_main_align(Align::Center),
                                |ui| {
                                    ui.heading("No scene loaded");
                                },
                            );
                        }
                    }
                }
                Tab::RenderResult => {
                    if let Some(scene) = self.yaml_menu.scene() {
                        self.render_result.show(ui, scene, &self.render);
                    }
                }
            }
        });
    }
}

#[derive(PartialEq)]
enum Tab {
    Preview,
    RenderResult,
}
