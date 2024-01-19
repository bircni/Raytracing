use self::preview::Preview;
use self::render::Render;
use self::renderresult::RenderResult;
use self::statusbar::StatusBar;
use self::yamlmenu::YamlMenu;
use crate::scene::Scene;
use crate::ui::properties::Properties;
use anyhow::Context;
use eframe::CreationContext;
use egui::mutex::{Mutex, RwLock};
use egui::{
    hex_color, include_image, vec2, Align, CentralPanel, ColorImage, Direction, ImageButton,
    ImageData, Layout, ScrollArea, SidePanel, TextStyle, TextureOptions,
};
use image::ImageBuffer;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::JoinHandle;

mod preview;
mod properties;
mod render;
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
    pub fn new(cc: &CreationContext) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        // Initialize the preview renderer with the wgpu context
        preview::gpu::init_wgpu(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );

        // create initial render texture (GPU exclusive) and image buffer (CPU exclusive)
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

        let scene = Arc::new(RwLock::new(None));

        Ok(Self {
            current_tab: Tab::Preview,
            render: Render::new(render_texture, image_buffer),
            properties: Properties::new(),
            statusbar: StatusBar::new(),
            preview: Preview::new(scene.clone()),
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
                                    self.properties.show(scene, ui, &mut self.render);
                                }
                            });
                        });

                    match scene.as_mut() {
                        Some(scene) => self.preview.show(ui, scene),
                        None => {
                            // TODO: make this implicit somehow
                            let tint_color = if ui.visuals().dark_mode {
                                hex_color!("#ffffff")
                            } else {
                                hex_color!("#000000")
                            };
                            ui.with_layout(
                                Layout::centered_and_justified(Direction::LeftToRight)
                                    .with_main_align(Align::Center),
                                |ui| {
                                    //ui.heading("No scene loaded");
                                    ui.add_sized(
                                        [20.0, 20.0],
                                        ImageButton::new(include_image!(
                                            "../../res/icons/plus-solid.svg"
                                        ))
                                        .tint(tint_color),
                                    )
                                    .on_hover_text("New Scene")
                                    .clicked()
                                    .then(|| self.yaml_menu.create_scene());
                                    ui.add_sized(
                                        [20.0, 20.0],
                                        ImageButton::new(include_image!(
                                            "../../res/icons/folder-open-solid.svg"
                                        ))
                                        .tint(tint_color),
                                    )
                                    .on_hover_text("Load Scene")
                                    .clicked()
                                    .then(|| self.yaml_menu.load_scene());
                                },
                            );
                        }
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
