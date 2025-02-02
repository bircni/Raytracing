use crate::scene::{Camera, Scene, Settings};
use anyhow::Context;
use egui::{hex_color, include_image, Align, ImageButton, Layout, RichText, Ui};
use egui_file::FileDialog;
use log::{info, warn};
use rust_i18n::t;
use std::{fs, path::Path};

pub struct YamlMenu {
    pub open_yaml_dialog: Option<FileDialog>,
    create_yaml_dialog: Option<FileDialog>,
}

impl YamlMenu {
    pub const fn new() -> Self {
        Self {
            open_yaml_dialog: None,
            create_yaml_dialog: None,
        }
    }

    pub fn show(&mut self, scene: &mut Option<Scene>, ui: &mut Ui) {
        // show open yaml dialog if present
        if let Some(d) = self.open_yaml_dialog.as_mut() {
            if d.show(ui.ctx()).selected() {
                if let Some(p) = d.path() {
                    info!("Loading scene from {}", p.display());
                    Scene::load(p)
                        .map_err(|e| {
                            warn!("{}", e);
                        })
                        .map(|s| {
                            scene.replace(s);
                        })
                        .ok();
                } else {
                    warn!("Open yaml dialog selected but returned no path");
                }

                self.open_yaml_dialog = None;
            }
        }

        // show create yaml dialog if present
        if let Some(d) = self.create_yaml_dialog.as_mut() {
            if d.show(ui.ctx()).selected() {
                match d.path() {
                    Some(p) => {
                        info!("Created new scene at {}", p.display());
                        scene.replace(Scene {
                            path: p.to_path_buf(),
                            objects: vec![],
                            lights: vec![],
                            camera: Camera::default(),
                            settings: Settings::default(),
                        });

                        Self::save_scene(scene.as_ref());
                    }
                    None => {
                        warn!("Create yaml dialog selected but returned no path");
                    }
                }

                self.create_yaml_dialog = None;
            }
        }

        ui.horizontal(|ui| {
            ui.heading(t!("yaml"));
            self.buttons(scene, ui);
        });

        ui.group(|ui| {
            ui.vertical_centered(|ui| match scene {
                Some(s) => {
                    ui.label(format!("{}:", t!("loaded_scene")));
                    ui.label(RichText::new(format!("{}", s.path.display())))
                }
                None => ui.label(t!("no_scene_loaded")),
            });
        });
    }

    pub fn load_scene(&mut self) {
        if !self
            .open_yaml_dialog
            .as_ref()
            .is_some_and(egui_file::FileDialog::visible)
        {
            let mut dialog = FileDialog::open_file(None).filename_filter(Box::new(|p| {
                Path::new(p)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml"))
            }));

            dialog.open();

            self.open_yaml_dialog = Some(dialog);
        }
    }

    pub fn create_scene(&mut self) {
        if !self
            .create_yaml_dialog
            .as_ref()
            .is_some_and(egui_file::FileDialog::visible)
        {
            let mut dialog = FileDialog::save_file(None).filename_filter(Box::new(|p| {
                Path::new(p)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("yaml"))
            }));

            dialog.open();
            self.create_yaml_dialog = Some(dialog);
        }
    }

    fn buttons(&mut self, scene: &mut Option<Scene>, ui: &mut Ui) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // TODO: make this implicit somehow
            let tint_color = if ui.visuals().dark_mode {
                hex_color!("#ffffff")
            } else {
                hex_color!("#000000")
            };

            // load button
            ui.add_sized(
                [20.0, 20.0],
                ImageButton::new(include_image!("../../res/icons/folder-open-solid.svg"))
                    .tint(tint_color),
            )
            .on_hover_text(t!("load_scene"))
            .clicked()
            .then(|| self.load_scene());

            // save button
            ui.add_enabled_ui(scene.is_some(), |ui| {
                ui.add_sized(
                    [20.0, 20.0],
                    ImageButton::new(include_image!("../../res/icons/floppy-disk-solid.svg"))
                        .tint(tint_color),
                )
                .on_hover_text(t!("save_scene"))
                .clicked()
                .then(|| Self::save_scene(scene.as_ref()));
            });

            // new button
            ui.add_sized(
                [20.0, 20.0],
                ImageButton::new(include_image!("../../res/icons/plus-solid.svg")).tint(tint_color),
            )
            .on_hover_text(t!("new_scene"))
            .clicked()
            .then(|| self.create_scene());

            // reload button
            ui.add_enabled_ui(scene.is_some(), |ui| {
                ui.add_sized(
                    [20.0, 20.0],
                    ImageButton::new(include_image!(
                        "../../res/icons/arrow-rotate-left-solid.svg"
                    ))
                    .tint(tint_color),
                )
                .on_hover_text(t!("reload_scene"))
                .clicked()
                .then(|| {
                    if let Some(path) = scene.as_ref().map(|s| s.path.clone()) {
                        match Scene::load(path.as_path()) {
                            Ok(s) => {
                                scene.replace(s);
                            }
                            Err(e) => warn!("{}", e),
                        }
                    }
                });
            });
        });
    }

    fn save_scene(scene: Option<&Scene>) {
        match scene {
            Some(scene) => {
                serde_yml::to_string(scene)
                    .context("Failed to serialize scene")
                    .and_then(|str| {
                        fs::write(scene.path.as_path(), str).context("Failed to save config")
                    })
                    .unwrap_or_else(|e| {
                        warn!("{}", e);
                    });
            }
            None => {
                warn!("save_scene called with no scene loaded");
            }
        }
    }
}
