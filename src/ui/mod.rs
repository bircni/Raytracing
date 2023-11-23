mod preview;
mod properties;
mod render;

use self::preview::Preview;

use crate::scene::Scene;
use anyhow::Context;
use eframe::CreationContext;
use egui::{
    mutex::Mutex, pos2, Align, CursorIcon, Frame, Layout, ProgressBar, Rect, Rounding, Stroke, Vec2,
};
use egui::{
    CentralPanel, Color32, ColorImage, ImageData, Key, Sense, TextStyle, TextureHandle,
    TextureOptions, Ui,
};
use egui_file::FileDialog;
use image::{ImageBuffer, RgbImage};
use log::{info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

pub struct App {
    current_tab: usize,
    scene: Scene,
    render_texture: TextureHandle,
    rendering_thread: Option<std::thread::JoinHandle<()>>,
    render_image: Arc<Mutex<RgbImage>>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    save_image_dialog: Option<FileDialog>,
    render_size: [u32; 2],
    rendering_progress: Arc<AtomicU16>,
    preview_zoom: f32,
    preview_position: Vec2,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        Preview::init(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );

        let render_size = [1920, 1080];

        let render_texture = cc.egui_ctx.load_texture(
            "render",
            ImageData::Color(Arc::new(ColorImage {
                size: [render_size[0] as usize, render_size[1] as usize],
                pixels: vec![Color32::BLACK; (render_size[0] * render_size[1]) as usize],
            })),
            TextureOptions::default(),
        );

        let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(render_size[0], render_size[1])));

        cc.egui_ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Name("subheading".into()),
                TextStyle::Monospace.resolve(s),
            );
        });

        Ok(Self {
            current_tab: 0,
            scene,
            preview_zoom: 0.0,
            preview_position: Vec2::ZERO,
            render_texture,
            rendering_thread: None,
            opened_file: None,
            open_file_dialog: None,
            save_image_dialog: None,
            render_size,
            rendering_progress: Arc::new(AtomicU16::new(0)),
            render_image: image_buffer,
        })
    }

    fn export_button(&mut self, ui: &mut Ui) {
        if ui.button("Export").clicked() {
            info!("Exporting image");
            self.save_image_dialog
                .get_or_insert_with(|| {
                    FileDialog::save_file(None)
                        .default_filename("render.png")
                        .filename_filter(Box::new(|name| {
                            [".png", ".jpg", ".jpeg"]
                                .into_iter()
                                .any(|ext| name.ends_with(ext))
                        }))
                })
                .open();
        }

        if let Some(dialog) = &mut self.save_image_dialog {
            if dialog.show(ui.ctx()).selected() {
                if let Some(file) = dialog.path() {
                    log::info!("Saving image to {:?}", file);
                    self.render_image.lock().save(file).unwrap_or_else(|e| {
                        warn!("Failed to save image: {}", e);
                    });
                }
            }
        }
    }

    fn render_button(&mut self, ui: &mut Ui) {
        ui.add_enabled_ui(self.rendering_thread.is_none(), |ui| {
            ui.button("Render").clicked().then(|| {
                self.render(ui.ctx().clone());
                self.current_tab = 1;
            })
        });
    }

    fn preview(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style())
            .outer_margin(10.0)
            .fill(Color32::from_rgb(
                (self.scene.settings.background_color[0] * 255.0) as u8,
                (self.scene.settings.background_color[1] * 255.0) as u8,
                (self.scene.settings.background_color[2] * 255.0) as u8,
            ))
            .show(ui, |ui| {
                let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

                painter.add(Preview::paint(response.rect, &self.scene));
            });
    }

    fn render_result(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style()).outer_margin(10.0).show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

            let response = response.on_hover_and_drag_cursor(CursorIcon::Grab);

            self.preview_zoom += ui.input(|i| i.scroll_delta.y);
            self.preview_zoom = self.preview_zoom.clamp(
                -response.rect.width().min(response.rect.height()) / 4.0,
                std::f32::INFINITY,
            );
            self.preview_position += response.drag_delta();

            response.double_clicked().then(|| {
                self.preview_zoom = 0.0;
                self.preview_position = Vec2::ZERO;
            });

            // paint gray grid
            let cell_size = 25.0;
            for y in 0..=response.rect.height() as usize / cell_size as usize {
                for x in 0..=response.rect.width() as usize / cell_size as usize {
                    painter.rect(
                        Rect::from_min_size(
                            pos2(
                                response.rect.left() + x as f32 * cell_size,
                                response.rect.top() + y as f32 * cell_size,
                            ),
                            Vec2::splat(cell_size),
                        ),
                        Rounding::default(),
                        if (x + y) % 2 == 0 {
                            Color32::GRAY
                        } else {
                            Color32::DARK_GRAY
                        },
                        Stroke::NONE,
                    );
                }
            }

            let render_aspect = self.render_size[0] as f32 / self.render_size[1] as f32;
            let rect = Rect::from_min_size(
                response.rect.min,
                // keep aspect ratio
                Vec2::new(
                    response
                        .rect
                        .width()
                        .min(response.rect.height() * render_aspect),
                    response
                        .rect
                        .height()
                        .min(response.rect.width() / render_aspect),
                ),
            );

            // center rect
            let rect = Rect::from_min_size(
                rect.min + (response.rect.size() - rect.size()) / 2.0,
                rect.size(),
            );

            painter.image(
                self.render_texture.id(),
                rect.translate(self.preview_position).expand2(Vec2::new(
                    self.preview_zoom * render_aspect,
                    self.preview_zoom,
                )),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        });
    }

    fn save_scene(&mut self) {
        serde_yaml::to_string(&self.scene)
            .context("Failed to serialize scene")
            .and_then(|str| std::fs::write("res/config.yaml", str).context("Failed to save config"))
            .unwrap_or_else(|e| {
                warn!("Failed to save config: {}", e);
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rendering_thread
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
            .then(|| {
                self.rendering_thread = None;
            });

        ctx.input(|input| input.key_pressed(Key::S) && input.modifiers.ctrl)
            .then(|| {
                self.save_scene();
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_label(self.current_tab == 0, "Preview")
                    .clicked()
                    .then(|| {
                        self.current_tab = 0;
                    });

                ui.selectable_label(self.current_tab == 1, "Render")
                    .clicked()
                    .then(|| {
                        self.current_tab = 1;
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    self.export_button(ui);
                    self.render_button(ui);

                    ui.add(
                        ProgressBar::new(
                            f32::from(self.rendering_progress.load(Ordering::Relaxed))
                                / f32::from(u16::MAX),
                        )
                        .desired_width(ui.available_width() / 3.0)
                        .show_percentage()
                        .fill(Color32::DARK_BLUE),
                    );

                    ui.label("Rendering progress");
                });
            });

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                0 => {
                    self.properties(ui);

                    self.preview(ui);
                }

                1 => {
                    self.render_result(ui);
                }
                n => unreachable!("Invalid tab index {}", n),
            }
        });
    }
}
