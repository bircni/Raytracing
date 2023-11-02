use std::ops::RangeInclusive;

use egui::{CentralPanel, RichText, Slider};

pub struct App {
    current_tab: usize,
    rays_per_pixel: i32,
    picked_path: Option<String>,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: 0,
            rays_per_pixel: 0,
            picked_path: None,
        }
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
                    ui.centered_and_justified(|ui| {
                        ui.heading("Preview will be here");
                    });

                    egui::SidePanel::right("panel").show(ctx, |ui| {
                        ui.heading(RichText::new("Properties").size(35.0));
                        

                        ui.separator();
                        ui.add_space(10.0);

                        ui.label(RichText::new("Rays per pixel:").size(20.0));
                        ui.add_space(10.0);

                        ui.add(Slider::new(
                            &mut self.rays_per_pixel,
                            RangeInclusive::new(0, 100),
                        ));

                        ui.add_space(10.0);

                        ui.label(RichText::new("FEATURE: pick file").size(20.0));
                        ui.add_space(10.0);

                        if ui.button(RichText::new("Open file").size(20.0)).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Object", &["obj", "mtl"])
                                .pick_file()
                            {
                                self.picked_path = Some(path.display().to_string());
                                println!("picked path: {:?}", self.picked_path)
                            }
                        }

                        if self.picked_path.is_some() {
                            ui.label("Picked path:");
                            ui.label(RichText::new(self.picked_path.as_ref().unwrap()).size(20.0));
                            
                        }

                        ui.separator();
                        ui.add_space(10.0);

                        ui.vertical_centered(|ui| {
                            ui.button(RichText::new("Render").size(30.0))
                                .clicked()
                                .then(|| self.current_tab = 1);
                        });
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
