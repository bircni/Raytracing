use self::preview::Preview;
use self::renderresult::RenderResult;
use self::statusbar::StatusBar;
use self::yamlmenu::YamlMenu;
use crate::raytracer::render::Render;
use crate::scene::Scene;
use crate::ui::properties::Properties;
use anyhow::Context;
use eframe::CreationContext;
use egui::mutex::{Mutex, RwLock};
use egui::{
    CentralPanel, ColorImage, ImageData, ScrollArea, SidePanel, TextStyle, TextureOptions, vec2,
};
use image::ImageBuffer;
use std::f32;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

mod preview;
mod properties;
mod renderresult;
mod statusbar;
mod yamlmenu;

/// Main application
/// This holds all the UI elements and application state
pub struct App {
    current_tab: Tab,
    render: Render,
    properties: Properties,
    statusbar: StatusBar,
    preview: Preview,
    render_result: RenderResult,
    yaml_menu: YamlMenu,
    scene: Arc<RwLock<Option<Scene>>>,
}

#[derive(PartialEq)]
enum Tab {
    Preview,
    RenderResult,
}

impl App {
    pub fn new(cc: &CreationContext<'_>) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Initialize the preview renderer with the wgpu context
        preview::gpu::init_wgpu(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );

        // create initial render texture (GPU exclusive) and image buffer (CPU exclusive)
        let render_texture = cc.egui_ctx.load_texture(
            "render",
            ImageData::Color(Arc::new(ColorImage::example())),
            TextureOptions::default(),
        );
        let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(0, 0)));

        cc.egui_ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Name("subheading".into()),
                TextStyle::Monospace.resolve(s),
            );
            s.text_styles
                .insert(TextStyle::Body, TextStyle::Monospace.resolve(s));
            s.spacing.item_spacing = vec2(10.0, f32::consts::PI * 1.76643);
        });

        let scene = Arc::new(RwLock::new(None));

        Ok(Self {
            current_tab: Tab::Preview,
            render: Render::new(render_texture, image_buffer),
            properties: Properties::new(),
            statusbar: StatusBar::new(),
            preview: Preview::new(Arc::<RwLock<Option<Scene>>>::clone(&scene)),
            render_result: RenderResult::new(),
            yaml_menu: YamlMenu::new(),
            scene,
        })
    }
}

/// Main application loop (called every frame)
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // check if the render thread has finished and reset it
        self.render
            .thread
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
            .then(|| {
                self.render.thread = None;
                self.render.cancel.store(false, Ordering::Relaxed);
            });

        // lock the scene for the duration of the frame
        let mut scene = self.scene.write();
        CentralPanel::default().show(ctx, |ui| {
            self.statusbar
                .show(ui, scene.as_mut(), &mut self.render, &mut self.current_tab);

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                Tab::Preview => {
                    SidePanel::right("panel")
                        .show_separator_line(true)
                        .show_inside(ui, |ui| {
                            ScrollArea::new([false, true]).show(ui, |ui| {
                                self.yaml_menu.show(&mut scene, ui);

                                ui.separator();

                                if let Some(scene) = scene.as_mut() {
                                    self.properties.show(scene, ui, &self.render);
                                }
                            });
                        });

                    //if let Some(scene) = scene.as_mut() {
                    //    self.preview.show(ui, scene);
                    //} else {
                    //    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    //        ui.horizontal(|ui| {
                    //            ui.vertical_centered(|ui| {
                    //                ui.heading(t!("no_scene_loaded"));
                    //                ui.label(RichText::new(t!("drop_yaml")));
                    //            });
                    //        });
                    //    });
                    //}
                    self.preview.show(ui, &mut scene);
                }
                Tab::RenderResult => {
                    if let Some(scene) = scene.as_ref() {
                        self.render_result.show(ui, scene, &self.render);
                    }
                }
            }
        });
    }
}
