use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use egui::{
    color_picker, hex_color, include_image, Align, Button, Color32, ColorImage, DragValue,
    FontFamily, ImageButton, ImageData, Layout, RichText, Slider, TextureOptions, Ui,
};
use egui_file::FileDialog;
use image::RgbImage;
use log::warn;
use nalgebra::{coordinates::XYZ, Scale3, Translation3, UnitQuaternion};

use crate::{
    scene::{Light, Object, Skybox},
    Color, Scene,
};

use super::render::Render;

fn xyz_drag_value(ui: &mut Ui, value: &mut XYZ<f32>) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut value.x).speed(0.1).prefix("x: "));
        ui.add(DragValue::new(&mut value.y).speed(0.1).prefix("y: "));
        ui.add(DragValue::new(&mut value.z).speed(0.1).prefix("z: "));
    });
}

pub struct Properties {
    skybox_file_dialog: Option<FileDialog>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
}

impl Properties {
    pub fn new() -> Self {
        Self {
            skybox_file_dialog: None,
            opened_file: None,
            open_file_dialog: None,
        }
    }
    pub fn show(&mut self, scene: &mut Scene, ui: &mut Ui, render: &mut Render) {
        ui.horizontal(|ui| {
            ui.heading("Properties");
        });

        Self::camera_settings(scene, ui);

        self.scene_settings(scene, ui, render);

        Self::lights(ui, scene);

        self.objects(ui, scene);
    }

    pub fn camera_settings(scene: &mut Scene, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Camera").size(16.0));
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label("Position:");

                xyz_drag_value(ui, &mut scene.camera.position);

                ui.label("Look at:");

                xyz_drag_value(ui, &mut scene.camera.look_at);

                ui.label("Field of View:");

                ui.add(
                    Slider::new(&mut scene.camera.fov, 0.0..=std::f32::consts::PI)
                        .step_by(0.01)
                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                        .clamp_to_range(true),
                );
            });
        });
    }

    fn scene_settings(&mut self, scene: &mut Scene, ui: &mut Ui, render: &mut Render) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("Scene Settings").size(16.0));
            });

            ui.separator();

            Self::render_options(ui, render, scene);

            self.skybox_options(ui, scene);

            ui.label("Ambient Color:");
            color_picker::color_edit_button_rgb(ui, scene.settings.ambient_color.as_mut());

            ui.label("Ambient Intensitiy:");
            ui.add(
                Slider::new(&mut scene.settings.ambient_intensity, 0.0..=1.0).clamp_to_range(true),
            );
        });
    }

    fn render_options(ui: &mut Ui, render: &mut Render, scene: &mut Scene) {
        ui.label("Render Size:");
        ui.vertical(|ui| {
            ui.add_enabled_ui(render.thread.is_none(), |ui| {
                ui.vertical(|ui| {
                    let text = Self::format_render_size(scene.camera.resolution);
                    egui::ComboBox::from_id_source(0)
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            (ui.selectable_value(
                                &mut scene.camera.resolution,
                                (1920, 1080),
                                "Full HD",
                            )
                            .changed()
                                || ui
                                    .selectable_value(
                                        &mut scene.camera.resolution,
                                        (2560, 1440),
                                        "2k",
                                    )
                                    .changed()
                                || ui
                                    .selectable_value(
                                        &mut scene.camera.resolution,
                                        (3840, 2160),
                                        "4k",
                                    )
                                    .changed()
                                || ui
                                    .selectable_value(
                                        &mut scene.camera.resolution,
                                        (7680, 4320),
                                        "8k",
                                    )
                                    .changed())
                            .then(|| {
                                Self::change_render_size(scene, render);
                            });
                        });
                    ui.horizontal(|ui| {
                        let (x, y) = &mut scene.camera.resolution;
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
                            Self::change_render_size(scene, render);
                        });
                    });
                });
            });
        });
    }

    fn skybox_options(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.label("Skybox:");

        if let Some(dialog) = &mut self.skybox_file_dialog {
            if dialog.show(ui.ctx()).selected() {
                match (|| {
                    let path = dialog.path().ok_or(anyhow::anyhow!("No path selected"))?;

                    let image = image::open(path)
                        .context("Failed to open image")?
                        .into_rgb8();

                    Ok::<_, anyhow::Error>(Skybox::Image {
                        path: path.to_path_buf(),
                        image,
                    })
                })() {
                    Ok(skybox) => {
                        scene.settings.skybox = skybox;
                    }
                    Err(e) => {
                        warn!("Failed to load skybox: {}", e);
                    }
                }

                self.skybox_file_dialog = None;
            }
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.radio(matches!(scene.settings.skybox, Skybox::Color(_)), "Color")
                    .clicked()
                    .then(|| {
                        scene.settings.skybox = Skybox::Color(Color::default());
                    });

                ui.radio(
                    matches!(scene.settings.skybox, Skybox::Image { .. }),
                    "Image",
                )
                .clicked()
                .then(|| {
                    let mut dialog = FileDialog::open_file(None).filename_filter(Box::new(|p| {
                        Path::new(p)
                            .extension()
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("exr"))
                    }));

                    dialog.open();

                    self.skybox_file_dialog = Some(dialog);
                });
            });

            match &mut scene.settings.skybox {
                Skybox::Image { path, .. } => {
                    ui.label(path.display().to_string());
                }
                Skybox::Color(c) => {
                    ui.color_edit_button_rgb(c.as_mut());
                }
            }
        });
    }

    fn lights(ui: &mut Ui, scene: &mut Scene) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("Lights ({})", scene.lights.len())).size(16.0))
            });

            scene
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
                    scene.lights.remove(n);
                });

            ui.separator();
            ui.vertical_centered(|ui| {
                ui.add(Button::new(RichText::new("+ Add Light")).frame(false))
                    .clicked()
                    .then(|| {
                        scene.lights.push(Light {
                            position: nalgebra::Point3::new(5.0, 2.0, 2.0),
                            intensity: 3.0,
                            color: nalgebra::Vector3::new(1.0, 1.0, 1.0),
                        });
                    });
            });
        });
    }

    fn objects(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(format!("Objects ({})", scene.objects.len())).size(16.0));
            });

            let mut objects_to_remove = Vec::new();

            for (n, o) in scene.objects.iter_mut().enumerate() {
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

                xyz_drag_value(ui, &mut o.translation);

                ui.label("Rotation");

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

                ui.label("Scale");

                xyz_drag_value(ui, &mut o.scale);
            }

            for o in objects_to_remove {
                scene.objects.remove(o);
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
                            match Object::from_obj(
                                file,
                                Translation3::identity(),
                                UnitQuaternion::identity(),
                                Scale3::identity(),
                            ) {
                                Ok(object) => {
                                    scene.objects.push(object);
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
    fn change_render_size(scene: &mut Scene, render: &mut Render) {
        let (x, y) = scene.camera.resolution;
        *render.image_buffer.lock() = RgbImage::new(x, y);
        scene.camera.resolution = (x, y);
        render.texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: [x as usize, y as usize],
                pixels: vec![Color32::BLACK; (x * y) as usize],
            })),
            TextureOptions::default(),
        );
    }

    fn format_render_size(size: (u32, u32)) -> &'static str {
        match size {
            (1920, 1080) => "FullHD",
            (2560, 1440) => "2k",
            (3840, 2160) => "4k",
            (7680, 4320) => "8k",
            _ => "Custom",
        }
    }
}
