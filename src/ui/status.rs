use std::sync::atomic::Ordering;

use egui::{Align, Button, Color32, Layout, Pos2, ProgressBar, RichText, Ui, Window};
use egui_file::FileDialog;
use log::{info, warn};

use crate::scene::Scene;

use super::{render::Render, Tab};

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
        scene: Option<&mut Scene>,
        render: &mut Render,
        current_tab: &mut Tab,
        show_popup: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.selectable_label(*current_tab == Tab::Preview, "Preview")
                .clicked()
                .then(|| {
                    *current_tab = Tab::Preview;
                });

            ui.selectable_label(*current_tab == Tab::RenderResult, "Render")
                .clicked()
                .then(|| {
                    *current_tab = Tab::RenderResult;
                });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.about_us_button(ui, show_popup);
                self.export_button(ui, render);
                Self::render_button(ui, render, scene, current_tab);
                Self::progress_bar(ui, render);
            });
        });
    }

    pub fn about_us_button(&mut self, ui: &mut Ui, show_popup: &mut bool) {
        ui.button(" ? ").clicked().then(|| {
            *show_popup = true;
        });

        if *show_popup {
            Window::new("About us")
                .resizable(true)
                .collapsible(false)
                .movable(false)
                .min_size((500.0, 100.0))
                .show(ui.ctx(), |ui| {
                    ui.add_space(10.0);
                    ui.label("We are Team TrayRacer.\n\nNicolas Bircks: Product Owner\nJonas Kluger: Scrum Master\nFabian Lippolt: Rust Profi\nDeveloper: Deniz Karagöz, Philipp Hamann, Marcel Süß, Tim Lanzinger");
                    ui.separator();
                    if ui.button("Close").clicked() {
                        *show_popup = false;
                    }
                });
        }
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
                    render.image_buffer.lock().save(file).unwrap_or_else(|e| {
                        warn!("Failed to save image: {}", e);
                    });
                }
            }
        }
    }

    pub fn render_button(
        ui: &mut Ui,
        render: &mut Render,
        scene: Option<&mut Scene>,
        current_tab: &mut Tab,
    ) {
        if render.thread.is_some() {
            ui.button("Cancel").clicked().then(|| {
                render.cancel.store(true, Ordering::Relaxed);
            });
        } else {
            ui.add_enabled_ui(render.thread.is_none() && scene.is_some(), |ui| {
                ui.button("Render").clicked().then(|| {
                    if let Some(scene) = scene {
                        render.render(ui.ctx().clone(), scene);
                        *current_tab = Tab::RenderResult;
                    }
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
                .fill(Color32::BLUE),
        );

        ui.label("Rendering progress");
    }
}
