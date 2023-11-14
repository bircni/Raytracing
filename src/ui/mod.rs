mod preview;
mod properties;
mod render;

use self::preview::Preview;
use crate::scene::Scene;
use eframe::CreationContext;
use egui::{
    load::SizedTexture, CentralPanel, Color32, ColorImage, ImageData, ImageSource, Sense,
    TextureHandle, TextureOptions,
};
use egui::{Align, Layout, ProgressBar};
use egui_file::FileDialog;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

pub struct App {
    current_tab: usize,
    scene: Scene,
    preview: Preview,
    render_texture: TextureHandle,
    rendering_thread: Option<std::thread::JoinHandle<()>>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    render_size: [usize; 2],
    rendering_progress: Arc<AtomicU16>,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        let preview = Preview::from_scene(cc.gl.clone().unwrap(), &scene)?;

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

        Ok(Self {
            current_tab: 0,
            scene,
            preview,
            render_texture,
            rendering_thread: None,
            opened_file: None,
            open_file_dialog: None,
            render_size,
            rendering_progress: Arc::new(AtomicU16::new(0)),
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

                    egui::Frame::canvas(ui.style())
                        .outer_margin(10.0)
                        .show(ui, |ui| {
                            let (response, painter) =
                                ui.allocate_painter(ui.available_size(), Sense::drag());
                            self.preview.paint(response.rect, &painter, &self.scene);
                        });
                }

                1 => {
                    egui::ScrollArea::new([true, true]).show(ui, |ui| {
                        ui.image(ImageSource::Texture(SizedTexture::from_handle(
                            &self.render_texture,
                        )));
                    });
                }
                n => panic!("Unknown tab: {}", n),
            }
        });
    }
}
