use std::ops::RangeInclusive;

use egui::{CentralPanel, RichText, Slider};

pub struct App {
    current_tab: usize,
}

impl App {
    pub fn new() -> Self {
        Self { current_tab: 0 }
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
                        ui.heading("Properties");

                        ui.separator();

                        ui.add(Slider::new(&mut 0, RangeInclusive::new(0, 100)));

                        ui.separator();

                        ui.vertical_centered(|ui| {
                            ui.button(RichText::new("Render").size(20.0))
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
