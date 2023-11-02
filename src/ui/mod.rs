mod preview;

use self::preview::Preview;
use crate::scene::Scene;

use eframe::CreationContext;
use egui::{CentralPanel, Sense};
use nalgebra::Point3;

pub struct App {
    current_tab: usize,
    scene: Scene,
    preview: Preview,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        let preview = Preview::from_scene(cc.gl.clone().unwrap(), &scene)?;

        Ok(Self {
            current_tab: 0,
            scene,
            preview,
        })
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.current_tab == 0, "Preview")
                    .clicked()
                {
                    self.current_tab = 0;
                }

                if ui
                    .selectable_label(self.current_tab == 1, "Render")
                    .clicked()
                {
                    self.current_tab = 1;
                }
            });

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                0 => {
                    egui::SidePanel::right("panel")
                        .show_separator_line(true)
                        .show_inside(ui, |ui| {
                            ui.group(|ui| {
                                ui.heading("Properties");

                                ui.vertical_centered(|ui| {
                                    ui.label("Camera");
                                });

                                ui.separator();

                                ui.horizontal(|ui| {
                                    ui.label("Position");
                                    ui.add(
                                        egui::DragValue::new(&mut self.scene.camera.position.x)
                                            .prefix("x: ")
                                            .speed(0.01),
                                    );
                                    ui.add(
                                        egui::DragValue::new(&mut self.scene.camera.position.z)
                                            .prefix("y: ")
                                            .speed(0.01),
                                    );
                                    ui.add(
                                        egui::DragValue::new(&mut self.scene.camera.position.y)
                                            .prefix("z: ")
                                            .speed(0.01),
                                    );
                                });

                                ui.vertical_centered(|ui| {
                                    ui.label("Objects");
                                });

                                ui.separator();

                                for o in self.scene.objects.iter() {
                                    ui.label(format!(
                                        "- Object at {}, with {} triangles",
                                        o.transform.transform_point(&Point3::origin()),
                                        o.triangles.len()
                                    ));
                                }
                            })
                        });

                    egui::Frame::canvas(ui.style())
                        .outer_margin(10.0)
                        .show(ui, |ui| {
                            let (response, painter) =
                                ui.allocate_painter(ui.available_size(), Sense::drag());
                            self.preview.paint(response.rect, &painter, &self.scene);
                        });
                }

                1 => {
                    ui.centered_and_justified(|ui| {
                        ui.heading("Render will be here");
                    });
                }
                n => panic!("Unknown tab: {}", n),
            }
        });
    }
}
