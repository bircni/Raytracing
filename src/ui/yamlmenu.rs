use std::path::Path;

use anyhow::Context;
use egui::{hex_color, include_image, Align, ImageButton, Layout, Ui};
use egui_file::FileDialog;
use log::{info, warn};

use crate::scene::{Camera, Scene, Settings};

pub struct YamlMenu {
    scene: Option<Scene>,
    open_dialog: Option<FileDialog>,
    create_dialog: Option<FileDialog>,
}

impl YamlMenu {
    pub fn new() -> Self {
        Self {
            scene: None,
            open_dialog: None,
            create_dialog: None,
        }
    }

    pub fn scene(&self) -> Option<&Scene> {
        self.scene.as_ref()
    }

    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        self.scene.as_mut()
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if let Some(dialog) = self.open_dialog.as_mut() {
            if dialog.show(ui.ctx()).selected() {
                if let Some(path) = dialog.path() {
                    info!("Loading scene from {}", path.display());
                    Scene::load(path)
                        .map_err(|e| {
                            warn!("{}", e);
                        })
                        .map(|scene| {
                            self.scene = Some(scene);
                        })
                        .ok();
                }

                self.open_dialog = None;
            }
        }

        if let Some(dialog) = self.create_dialog.as_mut() {
            if dialog.show(ui.ctx()).selected() {
                if let Some(path) = dialog.path() {
                    info!("New scene at {}", path.display());
                    self.scene = Some(Scene {
                        path: path.to_path_buf(),
                        objects: vec![],
                        lights: vec![],
                        camera: Camera::default(),
                        settings: Settings::default(),
                    });

                    self.save_scene();
                }

                self.create_dialog = None;
            }
        }

        ui.horizontal(|ui| {
            ui.heading("YAML");

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let tint_color = if ui.visuals().dark_mode {
                    hex_color!("#ffffff")
                } else {
                    hex_color!("#000000")
                };

                ui.add_sized(
                    [20.0, 20.0],
                    ImageButton::new(include_image!("../../res/icons/folder-open-solid.svg"))
                        .tint(tint_color),
                )
                .on_hover_text("Load Scene")
                .clicked()
                .then(|| {
                    if !self
                        .open_dialog
                        .as_ref()
                        .is_some_and(egui_file::FileDialog::visible)
                    {
                        let mut dialog =
                            FileDialog::open_file(None).filename_filter(Box::new(|p| {
                                Path::new(p)
                                    .extension()
                                    .map_or(false, |ext| ext.eq_ignore_ascii_case("yaml"))
                            }));

                        dialog.open();
                        self.open_dialog = Some(dialog);
                    }
                });

                ui.add_enabled_ui(self.scene.is_some(), |ui| {
                    ui.add_sized(
                        [20.0, 20.0],
                        ImageButton::new(include_image!("../../res/icons/floppy-disk-solid.svg"))
                            .tint(tint_color),
                    )
                    .on_hover_text("Save Scene")
                    .clicked()
                    .then(|| self.save_scene());
                });

                ui.add_sized(
                    [20.0, 20.0],
                    ImageButton::new(include_image!("../../res/icons/plus-solid.svg"))
                        .tint(tint_color),
                )
                .on_hover_text("New Scene")
                .clicked()
                .then(|| {
                    if !self
                        .create_dialog
                        .as_ref()
                        .is_some_and(egui_file::FileDialog::visible)
                    {
                        let mut dialog =
                            FileDialog::save_file(None).filename_filter(Box::new(|p| {
                                Path::new(p)
                                    .extension()
                                    .map_or(false, |ext| ext.eq_ignore_ascii_case("yaml"))
                            }));

                        dialog.open();
                        self.create_dialog = Some(dialog);
                    }
                });

                ui.add_enabled_ui(self.scene.is_some(), |ui| {
                    ui.add_sized(
                        [20.0, 20.0],
                        ImageButton::new(include_image!(
                            "../../res/icons/arrow-rotate-left-solid.svg"
                        ))
                        .tint(tint_color),
                    )
                    .on_hover_text("Reload Scene")
                    .clicked()
                    .then(|| {
                        if let Some(scene) = self.scene.as_mut() {
                            self.scene = Scene::load(scene.path.as_path())
                                .map_err(|e| {
                                    warn!("{}", e);
                                })
                                .ok();
                        }
                    });
                });
            });
        });

        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label("Loaded scene:");
                ui.label(
                    self.scene
                        .as_ref()
                        .map_or("None".to_string(), |s| s.path.display().to_string()),
                );
            });
        });
    }

    pub fn save_scene(&self) {
        if let Some(scene) = self.scene.as_ref() {
            serde_yaml::to_string(scene)
                .context("Failed to serialize scene")
                .and_then(|str| {
                    std::fs::write(scene.path.as_path(), str).context("Failed to save config")
                })
                .unwrap_or_else(|e| {
                    warn!("{}", e);
                });
        }
    }
}
