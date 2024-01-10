use std::sync::Arc;

use egui::{
    color_picker, hex_color, include_image, Align, Button, Color32, ColorImage, DragValue,
    FontFamily, ImageButton, ImageData, Layout, RichText, ScrollArea, SidePanel, Slider,
    TextureOptions, Ui,
};
use egui_file::FileDialog;
use image::RgbImage;
use log::warn;
use nalgebra::{coordinates::XYZ, Scale3, Translation3, UnitQuaternion};

use crate::{
    raytracer::Skybox,
    scene::{Light, Object},
};

use super::{App, RenderSize};

pub const SMALL_SPACE: f32 = 2.5;
pub const MEDIUM_SPACE: f32 = 5.0;
pub const LARGE_SPACE: f32 = 10.0;

fn xyz_drag_value(ui: &mut Ui, value: &mut XYZ<f32>) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut value.x).speed(0.1).prefix("x: "));
        ui.add(DragValue::new(&mut value.y).speed(0.1).prefix("y: "));
        ui.add(DragValue::new(&mut value.z).speed(0.1).prefix("z: "));
    });
}

impl App {
    pub fn info_bar(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                if let Some(text) = &self.info_text {
                    ui.label(text);
                } else {
                    ui.label(String::new());
                }
            });
        });
    }

    #[allow(clippy::out_of_bounds_indexing)]
    pub fn properties(&mut self, ui: &mut Ui) {
        SidePanel::right("panel")
            .show_separator_line(true)
            .show_inside(ui, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Properties");

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let tint_color = if ui.visuals().dark_mode {
                                hex_color!("#ffffff")
                            } else {
                                hex_color!("#000000")
                            };
                            ui.add_sized(
                                [20.0, 20.0],
                                ImageButton::new(include_image!(
                                    "../../res/icons/floppy-disk-solid.svg"
                                ))
                                .tint(tint_color),
                            )
                            .on_hover_text("Save Scene")
                            .clicked()
                            .then(|| {
                                self.save_scene();
                            });
                            ui.add_sized(
                                [20.0, 20.0],
                                ImageButton::new(include_image!(
                                    "../../res/icons/download-solid.svg"
                                ))
                                .tint(tint_color),
                            )
                            .on_hover_text("Download all Skyboxes")
                            .clicked()
                            .then(|| {
                                if let Some(skybox_path) = self.scene.path.parent() {
                                    self.download_skyboxes(skybox_path.join("skybox"));
                                } else {
                                    warn!("Failed to download skyboxes: scene path is not set");
                                }
                            });
                        });
                    });

                    ui.add_space(MEDIUM_SPACE);

                    self.camera_settings(ui);

                    ui.add_space(LARGE_SPACE);

                    self.scene_settings(ui);

                    ui.add_space(LARGE_SPACE);

                    self.lights(ui);

                    ui.add_space(LARGE_SPACE);

                    self.objects(ui);

                    ui.add_space(SMALL_SPACE);
                });
            });
    }

    fn camera_settings(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Camera").size(16.0));
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label("Position:");
                ui.add_space(SMALL_SPACE);
                xyz_drag_value(ui, &mut self.scene.camera.position);

                ui.add_space(MEDIUM_SPACE);

                ui.label("Look at:");
                ui.add_space(SMALL_SPACE);
                xyz_drag_value(ui, &mut self.scene.camera.look_at);

                ui.add_space(MEDIUM_SPACE);

                ui.label("Field of View:");
                ui.add_space(SMALL_SPACE);
                ui.add(
                    Slider::new(&mut self.scene.camera.fov, 0.0..=std::f32::consts::PI)
                        .step_by(0.01)
                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                        .clamp_to_range(true),
                );
                ui.add_space(SMALL_SPACE);
            });
        });
    }

    fn scene_settings(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Scene Settings").size(16.0));
            });

            ui.separator();

            self.render_options(ui);

            ui.add_space(MEDIUM_SPACE);

            self.skybox_options(ui);

            ui.add_space(MEDIUM_SPACE);

            ui.add_enabled_ui(self.scene.settings.skybox.is_none(), |ui| {
                ui.label("Background Color:");
                ui.add_space(SMALL_SPACE);
                color_picker::color_edit_button_rgb(
                    ui,
                    self.scene.settings.background_color.as_mut(),
                );
            });

            ui.add_space(MEDIUM_SPACE);

            ui.label("Ambient Color:");
            ui.add_space(SMALL_SPACE);
            color_picker::color_edit_button_rgb(ui, self.scene.settings.ambient_color.as_mut());

            ui.add_space(MEDIUM_SPACE);

            ui.label("Ambient Intensitiy:");
            ui.add_space(SMALL_SPACE);
            ui.add(
                Slider::new(&mut self.scene.settings.ambient_intensity, 0.0..=1.0)
                    .clamp_to_range(true),
            );
            ui.add_space(SMALL_SPACE);
        });
    }

    fn render_options(&mut self, ui: &mut Ui) {
        ui.label("Render Size:");
        ui.add_space(SMALL_SPACE);
        ui.vertical(|ui| {
            let mut render_size = self.render_size.as_size();
            ui.add_enabled_ui(self.rendering_thread.is_none(), |ui| {
                ui.vertical(|ui| {
                    egui::ComboBox::from_id_source(0)
                        .selected_text(format!("{}", self.render_size))
                        .show_ui(ui, |ui| {
                            (ui.selectable_value(
                                &mut self.render_size,
                                RenderSize::FullHD,
                                format!("{}", RenderSize::FullHD),
                            )
                            .changed()
                                | ui.selectable_value(
                                    &mut self.render_size,
                                    RenderSize::Wqhd,
                                    format!("{}", RenderSize::Wqhd),
                                )
                                .changed()
                                || ui
                                    .selectable_value(
                                        &mut self.render_size,
                                        RenderSize::Uhd1,
                                        format!("{}", RenderSize::Uhd1),
                                    )
                                    .changed()
                                || ui
                                    .selectable_value(
                                        &mut self.render_size,
                                        RenderSize::Uhd2,
                                        format!("{}", RenderSize::Uhd2),
                                    )
                                    .changed()
                                || ui
                                    .selectable_value(
                                        &mut self.render_size,
                                        RenderSize::Custom([render_size.0, render_size.1]),
                                        format!("{}", RenderSize::Custom([0, 0])),
                                    )
                                    .changed())
                            .then(|| {
                                self.change_render_size();
                            });
                        });
                    ui.add_space(MEDIUM_SPACE);
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(
                            self.rendering_thread.is_none()
                                && matches!(self.render_size, RenderSize::Custom(_)),
                            |ui| {
                                let (x, y) = match &mut self.render_size {
                                    RenderSize::Custom([x, y]) => (x, y),
                                    _ => (&mut render_size.0, &mut render_size.1),
                                };
                                (ui.add(
                                    DragValue::new(x)
                                        .speed(1.0)
                                        .clamp_range(10..=8192)
                                        .prefix("w: "),
                                )
                                .changed()
                                    || ui
                                        .add(
                                            DragValue::new(y)
                                                .speed(1.0)
                                                .clamp_range(10..=8192)
                                                .prefix("h: "),
                                        )
                                        .changed())
                                .then(|| {
                                    self.change_render_size();
                                });
                            },
                        );
                    });
                });
            });
        });
    }

    fn skybox_options(&mut self, ui: &mut Ui) {
        ui.label("Skybox:");
        ui.add_space(SMALL_SPACE);
        let mut skybox = self.scene.settings.skybox;
        ui.vertical(|ui| {
            egui::ComboBox::from_id_source(1)
                .selected_text(format!("{}", skybox.unwrap_or(Skybox::None)))
                .show_ui(ui, |ui| {
                    (ui.selectable_value(&mut skybox, None, format!("{}", Skybox::None))
                        .changed()
                        | ui.selectable_value(
                            &mut skybox,
                            Some(Skybox::ScythianTombs2),
                            format!("{}", Skybox::ScythianTombs2),
                        )
                        .changed()
                        | ui.selectable_value(
                            &mut skybox,
                            Some(Skybox::RainforestTrail),
                            format!("{}", Skybox::RainforestTrail),
                        )
                        .changed()
                        | ui.selectable_value(
                            &mut skybox,
                            Some(Skybox::StudioSmall08),
                            format!("{}", Skybox::StudioSmall08),
                        )
                        .changed()
                        | ui.selectable_value(
                            &mut skybox,
                            Some(Skybox::Kloppenheim02),
                            format!("{}", Skybox::Kloppenheim02),
                        )
                        .changed()
                        | ui.selectable_value(
                            &mut skybox,
                            Some(Skybox::CircusArena),
                            format!("{}", Skybox::CircusArena),
                        )
                        .changed())
                    .then(|| {
                        self.scene.settings.skybox = skybox;
                    });
                });
        });
    }

    #[allow(clippy::out_of_bounds_indexing)]
    fn lights(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("Lights ({})", self.scene.lights.len())).size(16.0))
            });

            self.scene
                .lights
                .iter_mut()
                .enumerate()
                .filter_map(|(n, light)| {
                    let mut remove = false;

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Light {n}"))
                                .size(14.0)
                                .family(FontFamily::Monospace),
                        );
                        ui.add_space(SMALL_SPACE);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            remove = ui
                                .add_sized(
                                    [20.0, 20.0],
                                    ImageButton::new(include_image!(
                                        "../../res/icons/trash-solid.svg"
                                    ))
                                    .tint(hex_color!("#cc0000")),
                                )
                                .clicked();
                        });
                    });

                    ui.label("Position:");
                    ui.add_space(SMALL_SPACE);
                    xyz_drag_value(ui, &mut light.position);

                    ui.add_space(MEDIUM_SPACE);

                    ui.label("Intensity:");
                    ui.add_space(SMALL_SPACE);
                    ui.add(Slider::new(&mut light.intensity, 0.0..=100.0).clamp_to_range(true));

                    ui.add_space(MEDIUM_SPACE);

                    ui.label("Color:");
                    ui.add_space(SMALL_SPACE);
                    color_picker::color_edit_button_rgb(ui, light.color.as_mut());

                    ui.add_space(MEDIUM_SPACE);

                    remove.then_some(n)
                })
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|n| {
                    self.scene.lights.remove(n);
                });

            ui.separator();
            ui.add_space(SMALL_SPACE);
            ui.vertical_centered(|ui| {
                ui.add(Button::new(RichText::new("+ Add Light")).frame(false))
                    .clicked()
                    .then(|| {
                        self.scene.lights.push(Light {
                            position: nalgebra::Point3::new(5.0, 2.0, 2.0),
                            intensity: 3.0,
                            color: nalgebra::Vector3::new(1.0, 1.0, 1.0),
                        });
                    });
            });
        });
    }

    fn objects(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("Objects ({})", self.scene.objects.len())).size(16.0),
                );
            });

            let mut objects_to_remove = Vec::new();

            #[allow(clippy::out_of_bounds_indexing)]
            for (n, o) in self.scene.objects.iter_mut().enumerate() {
                ui.separator();

                ui.add_space(SMALL_SPACE);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Object {} ({} ▲)", n, o.triangles.len()))
                            .size(14.0)
                            .family(FontFamily::Monospace),
                    );
                    ui.add_space(SMALL_SPACE);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_sized(
                                [20.0, 20.0],
                                ImageButton::new(include_image!("../../res/icons/trash-solid.svg"))
                                    .tint(hex_color!("#cc0000")),
                            )
                            .clicked()
                        {
                            objects_to_remove.push(n);
                        }
                    });
                });

                ui.label("Position");
                ui.add_space(SMALL_SPACE);
                xyz_drag_value(ui, &mut o.translation);
                ui.add_space(MEDIUM_SPACE);

                ui.label("Rotation");
                ui.add_space(SMALL_SPACE);
                ui.horizontal(|ui| {
                    let (mut x, mut y, mut z) = o.rotation.euler_angles();

                    [&mut x, &mut y, &mut z]
                        .iter_mut()
                        .any(|angle| {
                            ui.add(
                                DragValue::new(*angle)
                                    .speed(0.01)
                                    .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                    .prefix("x: "),
                            )
                            .changed()
                        })
                        .then(|| {
                            o.rotation = nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                        })
                });

                ui.add_space(MEDIUM_SPACE);

                ui.label("Scale");
                ui.add_space(SMALL_SPACE);
                xyz_drag_value(ui, &mut o.scale);
                ui.add_space(MEDIUM_SPACE);
            }

            for o in objects_to_remove {
                self.scene.objects.remove(o);
            }

            ui.separator();
            ui.add_space(SMALL_SPACE);
            ui.vertical_centered(|ui| {
                if ui
                    .add(Button::new(RichText::new("+ Add Object")).frame(false))
                    .clicked()
                {
                    let mut dialog = FileDialog::open_file(self.opened_file.clone())
                        .show_files_filter(Box::new(|path| {
                            path.extension().is_some_and(|ext| ext == "obj")
                        }));
                    dialog.open();
                    self.open_file_dialog = Some(dialog);
                }

                if let Some(dialog) = &mut self.open_file_dialog {
                    if dialog.show(ui.ctx()).selected() {
                        if let Some(file) = dialog.path() {
                            match Object::from_obj(
                                file,
                                Translation3::identity(),
                                UnitQuaternion::identity(),
                                Scale3::identity(),
                            ) {
                                Ok(object) => {
                                    self.scene.objects.push(object);
                                }
                                Err(e) => warn!("Failed to load object: {}", e),
                            }
                        }
                    }
                }
            });
        });
    }

    /// Change the render size
    fn change_render_size(&mut self) {
        let (x, y) = self.render_size.as_size();
        *self.render_image.lock() = RgbImage::new(x, y);

        self.render_texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: [x as usize, y as usize],
                pixels: vec![Color32::BLACK; (x * y) as usize],
            })),
            TextureOptions::default(),
        );
    }
}
