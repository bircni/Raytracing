mod preview;

use std::{borrow::BorrowMut, sync::Arc};

use self::preview::Preview;
use crate::{raytracer::Raytracer, scene::Scene, Color};

use eframe::CreationContext;
use egui::{
    load::SizedTexture, CentralPanel, Color32, ColorImage, ImageData, ImageSource, RichText, Sense,
    TextureHandle, TextureOptions,
};
use egui_file::FileDialog;
use nalgebra::Point3;
use rayon::prelude::{ParallelBridge, ParallelIterator};
use std::path::PathBuf;

pub struct App {
    current_tab: usize,
    scene: Scene,
    preview: Preview,
    render_texture: TextureHandle,
    rendering_thread: Option<std::thread::JoinHandle<()>>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

const RENDER_SIZE: [usize; 2] = [2000, 1000];

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        let preview = Preview::from_scene(cc.gl.clone().unwrap(), &scene)?;

        let render_texture = cc.egui_ctx.load_texture(
            "render",
            ImageData::Color(Arc::new(ColorImage {
                size: RENDER_SIZE,
                pixels: {
                    let mut pixels = Vec::<Color32>::with_capacity(RENDER_SIZE[0] * RENDER_SIZE[1]);
                    pixels.resize(RENDER_SIZE[0] * RENDER_SIZE[1], Color32::BLACK);
                    pixels
                },
            })),
            TextureOptions::default(),
        );

        Ok(Self {
            current_tab: 0,
            scene,
            preview,
            render_texture,
            rendering_thread: None,
            opened_file: None,
            open_file_dialog: None,
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
                            ui.heading("Properties");
                            //Camera Group
                            ui.add_space(5.0);
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("Camera").font(egui::FontId {
                                        size: (16.0),
                                        family: (egui::FontFamily::Proportional),
                                    }));
                                });

                                ui.separator();

                                ui.vertical(|ui| {
                                    ui.label("Position:");

                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.position.x)
                                                .speed(0.1)
                                                .prefix("x: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.position.y)
                                                .speed(0.1)
                                                .prefix("y: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.position.z)
                                                .speed(0.1)
                                                .prefix("z: "),
                                        );
                                    });

                                    ui.label("Look at:");

                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.look_at.x)
                                                .speed(0.1)
                                                .prefix("x: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.look_at.y)
                                                .speed(0.1)
                                                .prefix("y: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut self.scene.camera.look_at.z)
                                                .speed(0.1)
                                                .prefix("z: "),
                                        );
                                    });

                                    ui.label("Field of View:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.scene.camera.fov,
                                            0.0..=std::f32::consts::PI,
                                        )
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
                                    ui.label(RichText::new("Lights").font(egui::FontId {
                                        size: (16.0),
                                        family: (egui::FontFamily::Proportional),
                                    }));
                                });

                                for (n, light) in self.scene.lights.iter_mut().enumerate() {
                                    ui.separator();
                                    ui.label(RichText::new(format!("Light {n}")).font(
                                        egui::FontId {
                                            size: (14.0),
                                            family: (egui::FontFamily::Monospace),
                                        },
                                    ));

                                    ui.label("Position:");
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::DragValue::new(&mut light.position.x)
                                                .speed(0.1)
                                                .prefix("x: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut light.position.y)
                                                .speed(0.1)
                                                .prefix("y: "),
                                        );
                                        ui.add(
                                            egui::DragValue::new(&mut light.position.z)
                                                .speed(0.1)
                                                .prefix("z: "),
                                        );
                                    });

                                    ui.label("Intensity:");
                                    ui.add(
                                        egui::Slider::new(&mut light.intensity, 0.0..=100.0)
                                            .clamp_to_range(true),
                                    );

                                    ui.label("Color:");
                                    egui::color_picker::color_edit_button_rgb(
                                        ui,
                                        &mut light.color.as_mut(),
                                    );
                                }
                            });
                            ui.add_space(10.0);

                            //File Group
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(RichText::new("File").font(egui::FontId {
                                        size: (16.0),
                                        family: (egui::FontFamily::Proportional),
                                    }));
                                });
                                ui.separator();
                                if (ui.button("Open")).clicked() {
                                    let mut dialog =
                                        FileDialog::open_file(self.opened_file.clone());
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

                            //Objects Group
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "Objects [{}]",
                                            self.scene.objects.len()
                                        ))
                                        .font(
                                            egui::FontId {
                                                size: (16.0),
                                                family: (egui::FontFamily::Proportional),
                                            },
                                        ),
                                    );
                                });

                                ui.separator();

                                for o in self.scene.objects.iter_mut() {
                                    ui.label(format!(
                                        "+ Object at {} with {} triangles",
                                        o.transform.transform_point(&Point3::origin()),
                                        o.triangles.len()
                                    ));
                                }
                            });
                            ui.add_space(10.0);

                            //Render Button
                            let ctx = ctx.clone();
                            let mut texture = self.render_texture.clone();
                            let raytracer =
                                Raytracer::new(self.scene.clone(), Color::new(0.1, 0.1, 0.1));

                            ui.vertical_centered(|ui| {
                                ui.add_sized(
                                    [120., 40.],
                                    egui::Button::new(RichText::new("Render").font(egui::FontId {
                                        size: (16.0),
                                        family: (egui::FontFamily::Proportional),
                                    })),
                                )
                                .clicked()
                                .then(|| {
                                    self.render_texture.set(
                                        ImageData::Color(Arc::new(ColorImage {
                                            size: RENDER_SIZE,
                                            pixels: {
                                                let mut pixels = Vec::<Color32>::with_capacity(
                                                    RENDER_SIZE[0] * RENDER_SIZE[1],
                                                );
                                                pixels.resize(
                                                    RENDER_SIZE[0] * RENDER_SIZE[1],
                                                    Color32::BLACK,
                                                );
                                                pixels
                                            },
                                        })),
                                        TextureOptions::default(),
                                    );

                                    self.rendering_thread = Some(std::thread::spawn(move || {
                                        let block_size = 10;
                                        for y_block in 0..RENDER_SIZE[1] / block_size {
                                            for x_block in 0..RENDER_SIZE[0] / block_size {
                                                println!(
                                                    "Rendering block ({}, {})",
                                                    x_block, y_block
                                                );
                                                let pixels = (0..block_size)
                                                    .flat_map(|y| {
                                                        (0..block_size).map(move |x| (x, y))
                                                    })
                                                    .par_bridge()
                                                    .map(|(x, y)| {
                                                        let color = raytracer.render(
                                                            (
                                                                x + (x_block * block_size),
                                                                y + (y_block * block_size),
                                                            ),
                                                            (RENDER_SIZE[0], RENDER_SIZE[1]),
                                                        );
                                                        Color32::from_rgb(
                                                            (color.x * 255.0) as u8,
                                                            (color.y * 255.0) as u8,
                                                            (color.z * 255.0) as u8,
                                                        )
                                                    })
                                                    .collect::<Vec<_>>();

                                                texture.borrow_mut().set_partial(
                                                    [x_block * block_size, y_block * block_size],
                                                    ImageData::Color(Arc::new(ColorImage {
                                                        size: [block_size, block_size],
                                                        pixels,
                                                    })),
                                                    TextureOptions::default(),
                                                );

                                                ctx.request_repaint();
                                            }
                                        }
                                    }));
                                    self.current_tab = 1;
                                });
                            });
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
                    self.rendering_thread
                        .as_ref()
                        .map(|t| t.is_finished())
                        .unwrap_or(false)
                        .then(|| {
                            self.rendering_thread = None;
                        });

                    egui::ScrollArea::new([true, true]).show(ui, |ui| {
                        ui.image(ImageSource::Texture(SizedTexture::from_handle(
                            &self.render_texture,
                        )));
                    });
                }
                n => panic!("Unknown tab: {}", n),
            }
        });
    }
}
