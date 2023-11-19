use egui::{
    color_picker, Button, Context, DragValue, FontFamily, RichText, ScrollArea, SidePanel, Slider,
    Ui,
};
use egui_file::FileDialog;

impl super::App {
    pub fn properties(&mut self, ctx: &Context, ui: &mut Ui) {
        SidePanel::right("panel")
            .show_separator_line(true)
            .show_inside(ui, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.heading("Properties");

                    ui.add_space(5.0);

                    //Camera Group
                    ui.group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Camera").size(16.0));
                        });

                        ui.separator();

                        ui.vertical(|ui| {
                            ui.label("Position:");

                            ui.horizontal(|ui| {
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.position.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.position.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.position.z)
                                        .speed(0.1)
                                        .prefix("z: "),
                                );
                            });

                            ui.label("Look at:");

                            ui.horizontal(|ui| {
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.look_at.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.look_at.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                                ui.add(
                                    DragValue::new(&mut self.scene.camera.look_at.z)
                                        .speed(0.1)
                                        .prefix("z: "),
                                );
                            });

                            ui.label("Field of View:");
                            ui.add(
                                Slider::new(&mut self.scene.camera.fov, 0.0..=std::f32::consts::PI)
                                    .step_by(0.01)
                                    .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                    .clamp_to_range(true),
                            );
                        });
                    });

                    ui.add_space(10.0);

                    //Lighting Group
                    ui.group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(format!("Lights ({})", self.scene.lights.len()))
                                    .size(16.0),
                            );
                        });

                        for (n, light) in self.scene.lights.iter_mut().enumerate() {
                            ui.separator();
                            ui.label(
                                RichText::new(format!("Light {n}"))
                                    .size(14.0)
                                    .family(FontFamily::Monospace),
                            );

                            ui.label("Position:");
                            ui.horizontal(|ui| {
                                ui.add(
                                    DragValue::new(&mut light.position.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    DragValue::new(&mut light.position.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                                ui.add(
                                    DragValue::new(&mut light.position.z)
                                        .speed(0.1)
                                        .prefix("z: "),
                                );
                            });

                            ui.label("Intensity:");
                            ui.add(
                                Slider::new(&mut light.intensity, 0.0..=100.0).clamp_to_range(true),
                            );

                            ui.label("Color:");
                            color_picker::color_edit_button_rgb(ui, light.color.as_mut());
                        }
                    });

                    ui.add_space(10.0);

                    //Objects Group
                    ui.group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new(format!("Objects ({})", self.scene.objects.len()))
                                    .size(16.0),
                            );
                        });

                        for o in self.scene.objects.iter_mut() {
                            ui.separator();

                            ui.label(
                                RichText::new(format!(
                                    "Object with {} triangles",
                                    o.triangles.len()
                                ))
                                .size(14.0)
                                .family(FontFamily::Monospace),
                            );

                            ui.label("Position");
                            ui.horizontal(|ui| {
                                ui.add(
                                    DragValue::new(&mut o.transform.isometry.translation.x)
                                        .speed(0.1)
                                        .prefix("x: "),
                                );
                                ui.add(
                                    DragValue::new(&mut o.transform.isometry.translation.y)
                                        .speed(0.1)
                                        .prefix("y: "),
                                );
                                ui.add(
                                    DragValue::new(&mut o.transform.isometry.translation.z)
                                        .speed(0.1)
                                        .prefix("z: "),
                                );
                            });

                            ui.label("Rotation");
                            ui.horizontal(|ui| {
                                let (mut x, mut y, mut z) =
                                    o.transform.isometry.rotation.euler_angles();

                                ui.add(
                                    DragValue::new(&mut x)
                                        .speed(0.01)
                                        .clamp_range(
                                            -std::f32::consts::FRAC_PI_2
                                                ..=std::f32::consts::FRAC_PI_2,
                                        )
                                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                        .prefix("x: "),
                                )
                                .changed()
                                .then(|| {
                                    o.transform.isometry.rotation =
                                        nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                                });
                                ui.add(
                                    DragValue::new(&mut y)
                                        .speed(0.01)
                                        .clamp_range(
                                            -std::f32::consts::FRAC_PI_2
                                                ..=std::f32::consts::FRAC_PI_2,
                                        )
                                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                        .prefix("y: "),
                                )
                                .changed()
                                .then(|| {
                                    o.transform.isometry.rotation =
                                        nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                                });
                                ui.add(
                                    DragValue::new(&mut z)
                                        .speed(0.01)
                                        .clamp_range(
                                            -std::f32::consts::FRAC_PI_2
                                                ..=std::f32::consts::FRAC_PI_2,
                                        )
                                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                                        .prefix("z: "),
                                )
                                .changed()
                                .then(|| {
                                    o.transform.isometry.rotation =
                                        nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                                });
                            });
                        }
                    });

                    ui.add_space(10.0);

                    //File Group
                    ui.group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("File").size(16.0));
                        });
                        ui.separator();
                        if (ui.button("Open")).clicked() {
                            let mut dialog = FileDialog::open_file(self.opened_file.clone());
                            //dialog.filter(Box::new(|path| path.ends_with(".obj"))).open();
                            dialog.open();
                            self.open_file_dialog = Some(dialog);
                        }
                        if let Some(dialog) = &mut self.open_file_dialog {
                            if dialog.show(ctx).selected() {
                                if let Some(file) = dialog.path() {
                                    self.opened_file = Some(file.to_path_buf());
                                }
                            }
                        }
                        if self.opened_file.is_some() {
                            ui.label("Opened file:");
                            ui.label(self.opened_file.as_ref().unwrap().to_str().unwrap());
                        }
                    });

                    ui.add_space(10.0);

                    // Render Button
                    ui.vertical_centered(|ui| {
                        ui.add_enabled_ui(self.rendering_thread.is_none(), |ui| {
                            ui.add_sized(
                                // TODO: wegmachen
                                [120., 40.],
                                Button::new(RichText::new("Render").size(16.0)),
                            )
                            .clicked()
                            .then(|| {
                                self.render(ctx.clone());
                                self.current_tab = 1;
                            });
                        })
                    });
                })
            });
    }
}
