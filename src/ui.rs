use egui::{CentralPanel, RichText, Slider};
use egui_file::FileDialog;
use std::{ops::RangeInclusive, path::PathBuf};

pub struct App {
    current_tab: usize,
    rays_per_pixel: i32,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: 0,
            rays_per_pixel: 0,
            opened_file: None,
            open_file_dialog: None,
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
