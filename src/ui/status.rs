use std::sync::atomic::Ordering;

use egui::{Align, Button, Color32, Layout, ProgressBar, RichText, Ui};
use egui_file::FileDialog;
use log::{info, warn};

use crate::scene::Scene;

use super::render::Render;

pub struct Status {
    save_image_dialog: Option<FileDialog>,
}

impl Status {
    pub fn new() -> Self {
        Self {
            save_image_dialog: None,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        render: &mut Render,
        current_tab: &mut usize,
    ) {
        ui.horizontal(|ui| {
            ui.selectable_label(*current_tab == 0, "Preview")
                .clicked()
                .then(|| {
                    *current_tab = 0;
                });

            ui.selectable_label(*current_tab == 1, "Render")
                .clicked()
                .then(|| {
                    *current_tab = 1;
                });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.export_button(ui, render);
                Self::render_button(ui, render, scene, current_tab);
                Self::progress_bar(ui, render);
            });
        });
    }

    pub fn export_button(&mut self, ui: &mut Ui, render: &mut Render) {
        if ui
            .add_enabled(
                render.progress.load(Ordering::Relaxed) == u16::MAX,
                Button::new("Export"),
            )
            .clicked()
        {
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

        if let Some(dialog) = self.save_image_dialog.as_mut() {
            if dialog.show(ui.ctx()).selected() {
                if let Some(file) = dialog.path() {
                    log::info!("Saving image to {:?}", file);
                    render.rimage.lock().save(file).unwrap_or_else(|e| {
                        warn!("Failed to save image: {}", e);
                    });
                }
            }
        }
    }

    pub fn render_button(
        ui: &mut Ui,
        render: &mut Render,
        scene: &mut Scene,
        current_tab: &mut usize,
    ) {
        if render.thread.is_some() {
            ui.button("Cancel").clicked().then(|| {
                render.cancel.store(true, Ordering::Relaxed);
            });
        } else {
            ui.add_enabled_ui(render.thread.is_none(), |ui| {
                ui.button("Render").clicked().then(|| {
                    render.render(ui.ctx().clone(), scene);
                    *current_tab = 1;
                })
            });
        }
    }

    pub fn progress_bar(ui: &mut Ui, render: &Render) {
        let progress = f32::from(render.progress.load(Ordering::Relaxed)) / f32::from(u16::MAX);
        #[allow(clippy::float_cmp)]
        ui.add(
            ProgressBar::new(progress)
                .desired_width(ui.available_width() / 3.0)
                .text(
                    RichText::new(if progress == 1.0 {
                        format!(
                            "Done in: {:.2} s",
                            render.time.load(Ordering::Relaxed) as f32 / 1000.0
                        )
                    } else if progress > 0.0 {
                        format!("{:.1}%", progress * 100.0)
                    } else {
                        String::new()
                    })
                    .color(Color32::WHITE),
                )
                .fill(Color32::DARK_BLUE),
        );

        ui.label("Rendering progress");
    }
}