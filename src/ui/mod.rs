use self::filemanager::FileManager;
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
    vec2, Align, CentralPanel, ColorImage, CursorIcon, DroppedFile, ImageData, Layout, RichText,
    ScrollArea, SidePanel, TextStyle, TextureOptions,
};
use image::ImageBuffer;
use log::info;
use rust_i18n::t;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

mod filemanager;
mod preview;
mod properties;
mod renderresult;
mod statusbar;
mod yamlmenu;

/// Main application
/// This holds all the UI elements and application state
pub struct App {
    current_tab: Tab,
    cursor_icon: egui::CursorIcon,
    render: Render,
    properties: Properties,
    statusbar: StatusBar,
    preview: Preview,
    render_result: RenderResult,
    yaml_menu: YamlMenu,
    scene: Arc<RwLock<Option<Scene>>>,
    dropped_files: Vec<DroppedFile>,
}

#[derive(PartialEq)]
enum Tab {
    Preview,
    RenderResult,
}

impl App {
    pub fn new(cc: &CreationContext) -> anyhow::Result<Self> {
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
            s.spacing.item_spacing = vec2(10.0, std::f32::consts::PI * 1.76643);
        });

        let scene = Arc::new(RwLock::new(None));

        Ok(Self {
            current_tab: Tab::Preview,
            cursor_icon: CursorIcon::Default,
            render: Render::new(render_texture, image_buffer),
            properties: Properties::new(),
            statusbar: StatusBar::new(),
            preview: Preview::new(scene.clone()),
            render_result: RenderResult::new(),
            yaml_menu: YamlMenu::new(),
            scene,
            dropped_files: vec![],
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
        // check if the scene has been dropped
        if self.current_tab == Tab::Preview {
            ctx.input(|i| {
                if !i.raw.dropped_files.is_empty() {
                    self.dropped_files =
                        FileManager::check_file_extensions(i.raw.dropped_files.clone());
                    if let Some(path) = self.dropped_files.first().and_then(|p| p.path.as_ref()) {
                        FileManager::handle_file(path, &mut scene);
                    }
                    self.dropped_files.clear();
                }
            });
            FileManager::hovered_file(ctx, &scene.as_mut());
        }
        CentralPanel::default().show(ctx, |ui| {
            if self.cursor_icon != CursorIcon::Default {
                info!("Cursor icon: {:?}", self.cursor_icon);
            }
            ui.output_mut(|o| o.cursor_icon = self.cursor_icon);
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
                                self.yaml_menu.show(&mut scene, ui, &mut self.cursor_icon);

                                ui.separator();

                                if let Some(scene) = scene.as_mut() {
                                    self.properties.show(scene, ui, &mut self.render);
                                }
                            });
                        });

                    if let Some(scene) = scene.as_mut() {
                        self.preview.show(ui, scene);
                    } else {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading(t!("no_scene_loaded"));
                                    ui.label(RichText::new(t!("drop_yaml")));
                                });
                            });
                        });
                    }
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
