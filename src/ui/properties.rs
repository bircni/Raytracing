use egui::{
    color_picker, Align, Button, Context, DragValue, FontFamily, Layout, RichText, ScrollArea,
    SidePanel, Slider, Ui,
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

                        let mut lights_to_remove = Vec::new();

                        for (n, light) in self.scene.lights.iter_mut().enumerate() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("Light {n}"))
                                        .size(14.0)
                                        .family(FontFamily::Monospace),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add_sized(
                                            [20., 20.],
                                            Button::new(
                                                RichText::new("x")
                                                    .size(14.0)
                                                    .family(FontFamily::Monospace),
                                            )
                                            .frame(false)
                                            .small(),
                                        )
                                        .clicked()
                                    {
                                        lights_to_remove.push(n);
                                    }
                                });
                            });

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

                        for n in lights_to_remove {
                            self.scene.lights.remove(n);
                            log::info!("Removed light {}", n);
                        }

                        ui.separator();
                        ui.vertical_centered(|ui| {
                            ui.add(Button::new(RichText::new("+ Add Light")).frame(false))
                                .clicked()
                                .then(|| {
                                    self.scene.lights.push(Default::default());
                                    log::info!("Added light");
                                });
                        });
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

                        let mut objects_to_remove = Vec::new();

                        for (n, o) in self.scene.objects.iter_mut().enumerate() {
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label(
                                    //"Object with {} triangles"
                                    RichText::new(format!("Object ({} ▲)", o.triangles.len()))
                                        .size(14.0)
                                        .family(FontFamily::Monospace),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui
                                        .add_sized(
                                            [20., 20.],
                                            Button::new(
                                                RichText::new("x")
                                                    .size(14.0)
                                                    .family(FontFamily::Monospace),
                                            )
                                            .frame(false)
                                            .small(),
                                        )
                                        .clicked()
                                    {
                                        objects_to_remove.push(n);
                                    }
                                });
                            });

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

                        for o in objects_to_remove {
                            self.scene.objects.remove(o);
                            log::info!("Removed object");
                        }

                        ui.separator();
                        ui.vertical_centered(|ui| {
                            if ui
                                .add(Button::new(RichText::new("+ Add Object")).frame(false))
                                .clicked()
                            {
                                let dialog = FileDialog::open_file(self.opened_file.clone());
                                //dialog.filter(Box::new(|path| path.ends_with(".obj"))).open();
                                let mut dialog = dialog.filter(Box::new(|path| {
                                    path.extension().is_some_and(|ext| ext == "obj")
                                }));
                                dialog.open();
                                self.open_file_dialog = Some(dialog);
                                log::info!("Added object");
                            }
                            if let Some(dialog) = &mut self.open_file_dialog {
                                if dialog.show(ctx).selected() {
                                    if let Some(file) = dialog.path() {
                                        self.opened_file = Some(file.to_path_buf());
                                    }
                                }
                            }
                            if self.opened_file.is_some() {
                                ui.separator();
                                ui.label("Opened object:");
                                ui.label(self.opened_file.as_ref().unwrap().to_str().unwrap());
                            }
                        });
                    });
                })
            });
    }
}
