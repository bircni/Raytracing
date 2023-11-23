use egui::{
    color_picker, hex_color, include_image, Align, Button, DragValue, FontFamily, ImageButton,
    Layout, RichText, ScrollArea, SidePanel, Slider, TextStyle, Ui,
};
use egui_file::FileDialog;
use log::warn;
use nalgebra::{coordinates::XYZ, Similarity3};

use crate::scene::{Light, Object};

use super::App;

fn xyz_drag_value(ui: &mut Ui, value: &mut XYZ<f32>) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut value.x).speed(0.1).prefix("x: "));
        ui.add(DragValue::new(&mut value.y).speed(0.1).prefix("y: "));
        ui.add(DragValue::new(&mut value.z).speed(0.1).prefix("z: "));
    });
}

impl App {
    pub fn properties(&mut self, ui: &mut Ui) {
        SidePanel::right("panel")
            .show_separator_line(true)
            .show_inside(ui, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Properties");

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_sized(
                                [20.0, 20.0],
                                ImageButton::new(include_image!(
                                    "../../res/icons/floppy-disk-solid.svg"
                                ))
                                .tint(hex_color!("#ffffff")),
                            )
                            .clicked()
                            .then(|| {
                                self.save_scene();
                            });
                        });
                    });

                    ui.add_space(5.0);

                    self.camera_settings(ui);

                    ui.add_space(10.0);

                    self.scene_settings(ui);

                    ui.add_space(10.0);

                    self.lights(ui);

                    ui.add_space(10.0);

                    self.objects(ui);
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
                xyz_drag_value(ui, &mut self.scene.camera.position);

                ui.label("Look at:");
                xyz_drag_value(ui, &mut self.scene.camera.look_at);

                ui.label("Field of View:");
                ui.add(
                    Slider::new(&mut self.scene.camera.fov, 0.0..=std::f32::consts::PI)
                        .step_by(0.01)
                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                        .clamp_to_range(true),
                );
            });
        });
    }

    fn scene_settings(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Scene Settings").size(16.0));
            });

            ui.separator();

            ui.label("Rendering Size:");
            ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut self.render_size[0]).speed(1.0).prefix("w: "));
                    ui.add(DragValue::new(&mut self.render_size[1]).speed(1.0).prefix("h: "));
            });

            ui.label("Background Color:");
            color_picker::color_edit_button_rgb(ui, self.scene.settings.background_color.as_mut());

            ui.label("Ambient Color:");
            color_picker::color_edit_button_rgb(ui, self.scene.settings.ambient_color.as_mut());

            ui.label("Ambient Intensitiy:");
            ui.add(
                Slider::new(&mut self.scene.settings.ambient_intensity, 0.0..=1.0)
                    .clamp_to_range(true),
            );

            ui.separator();
        });
    }

    fn lights(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(format!("Lights ({})", self.scene.lights.len()))
                        .text_style(TextStyle::Name("subheading".into())),
                )
            });

            self.scene
                .lights
                .iter_mut()
                .enumerate()
                .filter_map(|(n, light)| {
                    let mut remove = false;

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Light {n}")).size(14.0));
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
                    xyz_drag_value(ui, &mut light.position);

                    ui.label("Intensity:");
                    ui.add(Slider::new(&mut light.intensity, 0.0..=100.0).clamp_to_range(true));

                    ui.label("Color:");
                    color_picker::color_edit_button_rgb(ui, light.color.as_mut());

                    remove.then_some(n)
                })
                .collect::<Vec<_>>()
                .into_iter()
                .for_each(|n| {
                    self.scene.lights.remove(n);
                });

            ui.separator();
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

            for (n, o) in self.scene.objects.iter_mut().enumerate() {
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("Object {} ({} ▲)", n, o.triangles.len()))
                            .size(14.0)
                            .family(FontFamily::Monospace),
                    );
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
                xyz_drag_value(ui, &mut o.transform.isometry.translation);

                ui.label("Rotation");
                ui.horizontal(|ui| {
                    let (mut x, mut y, mut z) = o.transform.isometry.rotation.euler_angles();

                    [&mut x, &mut y, &mut z]
                        .iter_mut()
                        .any(|angle| {
                            ui.add(
                                DragValue::new(*angle)
                                    .speed(0.01)
                                    .clamp_range(
                                        -std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2,
                                    )
                                    .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                    .prefix("x: "),
                            )
                            .changed()
                        })
                        .then(|| {
                            o.transform.isometry.rotation =
                                nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                        })
                });
            }

            for o in objects_to_remove {
                self.scene.objects.remove(o);
            }

            ui.separator();
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
                            match Object::from_obj(file, Similarity3::identity()) {
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
}
