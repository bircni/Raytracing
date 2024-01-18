use std::sync::atomic::Ordering;

use egui::special_emojis::GITHUB;
use egui::{Align, Button, Color32, Layout, ProgressBar, RichText, Ui, Window};
use egui_file::FileDialog;
use log::{info, warn};

use crate::scene::Scene;

use super::{render::Render, Tab};

pub struct Status {
    save_image_dialog: Option<FileDialog>,
    show_popup: bool,
}

impl Status {
    pub fn new() -> Self {
        Self {
            save_image_dialog: None,
            show_popup: false,
        }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: Option<&mut Scene>,
        render: &mut Render,
        current_tab: &mut Tab,
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
                self.about_us_button(ui);
                self.export_button(ui, render);
                Self::render_button(ui, render, scene, current_tab);
                Self::progress_bar(ui, render);
            });
            self.about_window(ui);
        });
    }

    pub fn about_us_button(&mut self, ui: &mut Ui) {
        ui.add(Button::new(" ? ").rounding(40.0))
            .clicked()
            .then(|| {
                self.show_popup = true;
            });
    }

    fn about_window(&mut self, ui: &mut Ui) {
        Window::new("About us")
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .open(&mut self.show_popup)
            .min_size((100.0, 100.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(5.0);
                    ui.add(
                        egui::Image::new(egui::include_image!("../../res/icon.png"))
                            .max_width(150.0)
                            .rounding(10.0),
                    );
                    ui.label(RichText::new("TrayRacer"));
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.hyperlink_to(
                        format!("{GITHUB} GitHub"),
                        "https://github.com/bircni/Raytracing",
                    );
                    ui.hyperlink_to("Built with egui", "https://docs.rs/egui/");
                    ui.label("© 2024 Team TrayRacer");
                    ui.add_space(5.0);
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
