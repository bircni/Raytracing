use std::sync::atomic::Ordering;

use egui::special_emojis::GITHUB;
use egui::{
    Align, Align2, Button, Color32, Frame, Layout, ProgressBar, RichText, Ui, Window, vec2,
};
use egui_file::FileDialog;
use log::{info, warn};
use rust_i18n::t;

use crate::raytracer::render::Render;
use crate::scene::Scene;

use super::Tab;

pub struct StatusBar {
    save_render_dialog: Option<FileDialog>,
    /// Whether the about window should be shown
    show_about: bool,
}

impl StatusBar {
    pub const fn new() -> Self {
        Self {
            save_render_dialog: None,
            show_about: false,
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
            ui.selectable_label(*current_tab == Tab::Preview, t!("preview"))
                .clicked()
                .then(|| {
                    *current_tab = Tab::Preview;
                });

            ui.selectable_label(*current_tab == Tab::RenderResult, t!("render"))
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
        ui.add(Button::new(" ? ").corner_radius(40.0))
            .clicked()
            .then(|| {
                self.show_about = true;
            });
    }

    fn about_window(&mut self, ui: &Ui) {
        Window::new(t!("about"))
            .resizable(false)
            .collapsible(false)
            .open(&mut self.show_about)
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .fixed_size(vec2(200.0, 150.0))
            .frame(Frame::window(ui.style()).fill(ui.style().visuals.widgets.open.weak_bg_fill))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new(egui::include_image!("../../res/icon.png"))
                            .shrink_to_fit()
                            .corner_radius(10.0),
                    );

                    ui.label(format!("{}: {}", t!("version"), env!("CARGO_PKG_VERSION")));
                    ui.hyperlink_to(
                        format!("{GITHUB} {}", t!("github")),
                        "https://github.com/bircni/Raytracing",
                    );

                    ui.hyperlink_to(t!("built_with"), "https://docs.rs/egui/");
                    ui.label(t!("copyright"));
                });
            });
    }

    pub fn export_button(&mut self, ui: &mut Ui, render: &Render) {
        if ui
            .add_enabled(
                render.progress.load(Ordering::Relaxed) == u16::MAX,
                Button::new(RichText::new(t!("export")).size(14.0)),
            )
            .clicked()
        {
            info!("Exporting image");
            self.save_render_dialog
                .get_or_insert_with(|| {
                    let (x, y) = render.image.lock().dimensions();
                    FileDialog::save_file(None)
                        .default_filename(format!("render_{x}x{y}.png"))
                        .filename_filter(Box::new(|name| {
                            [".png", ".jpg", ".jpeg"]
                                .into_iter()
                                .any(|ext| name.ends_with(ext))
                        }))
                })
                .open();
        }

        if let Some(dialog) = self.save_render_dialog.as_mut() {
            if dialog.show(ui.ctx()).selected() {
                match dialog.path() {
                    Some(path) => {
                        log::info!("Saving image to {}", path.display());
                        render.image.lock().save(path).unwrap_or_else(|e| {
                            warn!("Failed to save image: {e}");
                        });
                    }
                    None => {
                        warn!("Save dialog returned no path");
                    }
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
            ui.button(t!("cancel")).clicked().then(|| {
                render.cancel.store(true, Ordering::Relaxed);
            });
        } else {
            ui.add_enabled_ui(render.thread.is_none() && scene.is_some(), |ui| {
                ui.button(RichText::new(t!("render")).size(14.0))
                    .clicked()
                    .then(|| {
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
        ui.add(
            ProgressBar::new(progress)
                .desired_width(ui.available_width() / 3.0)
                .text(
                    RichText::new(
                        #[expect(clippy::float_cmp, reason = "We want to compare floats")]
                        if progress == 1.0 {
                            format!(
                                "{}: {:.2} s",
                                t!("done"),
                                render.time.load(Ordering::Relaxed) as f32 / 1000.0
                            )
                        } else if progress > 0.0 {
                            format!("{:.1}%", progress * 100.0)
                        } else {
                            String::new()
                        },
                    )
                    .color(Color32::WHITE),
                )
                .fill(Color32::BLUE),
        );

        ui.label(t!("render_progress"));
    }
}
