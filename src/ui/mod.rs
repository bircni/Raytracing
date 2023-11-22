mod preview;
mod properties;
mod render;

use self::preview::Preview;

use crate::scene::Scene;
use anyhow::Context;
use eframe::CreationContext;
use egui::{
    pos2, Align, Button, CursorIcon, Frame, Layout, ProgressBar, Rect, Rounding, Stroke, Vec2,
};
use egui::{CentralPanel, Color32, ColorImage, ImageData, Sense, TextureHandle, TextureOptions};
use egui_file::FileDialog;
use image::{ImageBuffer, RgbImage};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

pub struct App {
    current_tab: usize,
    scene: Scene,
    preview: Preview,
    render_texture: TextureHandle,
    rendering_thread: Option<std::thread::JoinHandle<()>>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    save_img_dialog: Option<FileDialog>,
    render_size: [usize; 2],
    rendering_progress: Arc<AtomicU16>,
    preview_zoom: f32,
    preview_position: Vec2,
    image_buffer: Arc<Mutex<RgbImage>>,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        let preview = Preview::new(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        )
        .context("Failed to create preview")?;

        let render_size = [1920, 1080];

        let render_texture = cc.egui_ctx.load_texture(
            "render",
            ImageData::Color(Arc::new(ColorImage {
                size: render_size,
                pixels: {
                    let mut pixels = Vec::<Color32>::with_capacity(render_size[0] * render_size[1]);
                    pixels.resize(render_size[0] * render_size[1], Color32::BLACK);
                    pixels
                },
            })),
            TextureOptions::default(),
        );

        // Create a dummy ImageBuffer for illustration purposes
        let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(
            render_size[0] as u32,
            render_size[1] as u32,
        )));

        Ok(Self {
            current_tab: 0,
            scene,
            preview,
            preview_zoom: 0.0,
            preview_position: Vec2::ZERO,
            render_texture,
            rendering_thread: None,
            opened_file: None,
            open_file_dialog: None,
            save_img_dialog: None,
            render_size,
            rendering_progress: Arc::new(AtomicU16::new(0)),
            image_buffer,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rendering_thread
            .as_ref()
            .map(|t| t.is_finished())
            .unwrap_or(false)
            .then(|| {
                self.rendering_thread = None;
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.current_tab == 0, "Preview")
                    .clicked()
                {
                    self.current_tab = 0;
                }

                if ui
                    .selectable_label(self.current_tab == 1, "Render")
                    .clicked()
                {
                    self.current_tab = 1;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.current_tab == 0 {
                        if ui
                            .add(Button::new("Render"))
                            .on_hover_text("Start rendering")
                            .clicked()
                        {
                            self.render(ctx.clone());
                            self.current_tab = 1;
                        }
                    } else if self.rendering_progress.load(Ordering::Relaxed) == u16::MAX
                        && ui.button("Export").clicked()
                    {
                        log::info!("Exporting image");
                        let mut dialog = FileDialog::save_file(None).default_filename("Rendered-Image.png");
                        dialog.open();
                        self.save_img_dialog = Some(dialog);
                    }

                    if let Some(dialog) = &mut self.save_img_dialog {
                        if dialog.show(ctx).selected() {
                            if let Some(file) = dialog.path() {
                                log::info!("Saving image to {:?}", file);
                                self.image_buffer.lock().unwrap().save(file).unwrap();
                            }
                        }
                    }

                    ui.add(
                        ProgressBar::new(
                            self.rendering_progress.load(Ordering::Relaxed) as f32
                                / u16::MAX as f32,
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
                    self.properties(ctx, ui);

                    Frame::canvas(ui.style())
                        .outer_margin(10.0)
                        .fill(Color32::from_rgb(
                            (self.scene.settings.background_color[0] * 255.0) as u8,
                            (self.scene.settings.background_color[1] * 255.0) as u8,
                            (self.scene.settings.background_color[2] * 255.0) as u8,
                        ))
                        .show(ui, |ui| {
                            let (response, painter) =
                                ui.allocate_painter(ui.available_size(), Sense::drag());

                            painter.add(self.preview.paint(response.rect, &self.scene));
                        });
                }

                1 => {
                    Frame::canvas(ui.style()).outer_margin(10.0).show(ui, |ui| {
                        let (response, painter) =
                            ui.allocate_painter(ui.available_size(), Sense::drag());

                        let response = response.on_hover_and_drag_cursor(CursorIcon::Grab);

                        self.preview_zoom += ctx.input(|i| i.scroll_delta.y);
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
                n => panic!("Unknown tab: {}", n),
            }
        });
    }
}
