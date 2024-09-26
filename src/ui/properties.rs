use crate::{
    raytracer::render::Render,
    scene::{Color, Light, Object, Skybox},
    Scene,
};
use anyhow::Context;
use egui::{
    color_picker, hex_color, include_image, Align, Button, CollapsingHeader, DragValue, FontFamily,
    ImageButton, Layout, RichText, Slider, SliderClamping, Ui,
};
use egui_file::FileDialog;
use log::warn;
use nalgebra::{coordinates::XYZ, Scale3, Translation3, UnitQuaternion};
use rust_i18n::t;
use std::{f32::consts, path::Path};

fn xyz_drag_value(ui: &mut Ui, value: &mut XYZ<f32>) {
    ui.horizontal(|ui| {
        ui.add(DragValue::new(&mut value.x).speed(0.1).prefix("x: "));
        ui.add(DragValue::new(&mut value.y).speed(0.1).prefix("y: "));
        ui.add(DragValue::new(&mut value.z).speed(0.1).prefix("z: "));
    });
}

pub struct Properties {
    /// Dialog to select a skybox image
    skybox_dialog: Option<FileDialog>,
    /// Dialog to add a new object
    object_dialog: Option<FileDialog>,
}

impl Properties {
    pub const fn new() -> Self {
        Self {
            skybox_dialog: None,
            object_dialog: None,
        }
    }

    pub fn show(&mut self, scene: &mut Scene, ui: &mut Ui, render: &Render) {
        ui.horizontal(|ui| {
            ui.heading(t!("properties"));
        });

        Self::camera_settings(scene, ui);

        ui.add_space(5.0);

        self.scene_settings(scene, ui, render);

        ui.add_space(5.0);

        Self::lights(ui, scene);

        ui.add_space(5.0);

        self.objects(ui, scene);
    }

    pub fn camera_settings(scene: &mut Scene, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(t!("camera")).size(16.0));
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.label(format!("{}:", t!("position")));

                xyz_drag_value(ui, &mut scene.camera.position);

                ui.label(format!("{}:", t!("look_at")));

                xyz_drag_value(ui, &mut scene.camera.look_at);

                ui.label(format!("{}:", t!("fov")));

                ui.add(
                    Slider::new(&mut scene.camera.fov, 0.0..=consts::PI)
                        .step_by(0.01)
                        .custom_formatter(|x, _| format!("{:.2}°", x.to_degrees()))
                        .clamping(SliderClamping::Always),
                );
            });
        });
    }

    fn scene_settings(&mut self, scene: &mut Scene, ui: &mut Ui, render: &Render) {
        ui.vertical(|ui| {
            ui.group(|ui| {
                CollapsingHeader::new(RichText::new(t!("scene_settings")).size(16.0))
                    .default_open(true)
                    .show_unindented(ui, |ui| {
                        ui.separator();

                        Self::render_options(ui, render, scene);

                        self.skybox_options(ui, scene);

                        Self::ambient_options(ui, scene);
                    });
            });
        });
    }

    fn ambient_options(ui: &mut Ui, scene: &mut Scene) {
        ui.label(format!("{}:", t!("ambient_color")));
        color_picker::color_edit_button_rgb(ui, scene.settings.ambient_color.as_mut());

        ui.label(format!("{}:", t!("ambient_intensity")));
        ui.add(
            Slider::new(&mut scene.settings.ambient_intensity, 0.0..=1.0)
                .clamping(SliderClamping::Always),
        );
    }

    fn render_options(ui: &mut Ui, render: &Render, scene: &mut Scene) {
        ui.label(format!("{}:", t!("render_size")));
        ui.vertical(|ui| {
            ui.add_enabled_ui(render.thread.is_none(), |ui| {
                ui.vertical(|ui| {
                    let text = Self::format_render_size(scene.camera.resolution);
                    egui::ComboBox::from_id_salt(0)
                        .selected_text(text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut scene.camera.resolution, (1280, 720), "HD");
                            ui.selectable_value(
                                &mut scene.camera.resolution,
                                (1920, 1080),
                                "Full HD",
                            );
                            ui.selectable_value(&mut scene.camera.resolution, (2560, 1440), "2k");
                            ui.selectable_value(&mut scene.camera.resolution, (3840, 2160), "4k");
                            ui.selectable_value(&mut scene.camera.resolution, (7680, 4320), "8k");
                        });
                    ui.horizontal(|ui| {
                        let (x, y) = &mut scene.camera.resolution;
                        ui.add(DragValue::new(x).speed(1.0).range(10..=8192).prefix("w: "));
                        ui.add(DragValue::new(y).speed(1.0).range(10..=8192).prefix("h: "));
                    });
                    ui.checkbox(&mut scene.settings.anti_aliasing, "Anti-Aliasing");
                    if scene.settings.anti_aliasing {
                        ui.label("Samples per pixel:");
                        ui.add(
                            Slider::new(&mut scene.settings.samples, 1..=128)
                                .clamping(SliderClamping::Always),
                        );
                    }
                });
            });
        });
    }

    #[allow(clippy::blocks_in_conditions)]
    fn skybox_options(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.label(format!("{}:", t!("background")));

        if let Some(dialog) = &mut self.skybox_dialog {
            if dialog.show(ui.ctx()).selected() {
                match (|| {
                    let path = dialog
                        .path()
                        .ok_or_else(|| anyhow::anyhow!("No path selected"))?;

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

                self.skybox_dialog = None;
            }
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.radio(
                    matches!(scene.settings.skybox, Skybox::Color(_)),
                    t!("color"),
                )
                .clicked()
                .then(|| {
                    scene.settings.skybox = Skybox::Color(Color::default());
                });

                ui.radio(
                    matches!(scene.settings.skybox, Skybox::Image { .. }),
                    t!("skybox"),
                )
                .clicked()
                .then(|| self.load_skybox_img());
            });

            match &mut scene.settings.skybox {
                Skybox::Image { path, .. } => {
                    ui.button(t!("reload_skybox"))
                        .clicked()
                        .then(|| self.load_skybox_img());
                    ui.label(path.display().to_string());
                }
                Skybox::Color(c) => {
                    ui.color_edit_button_rgb(c.as_mut());
                }
            }
        });
    }

    fn load_skybox_img(&mut self) {
        let mut dialog = FileDialog::open_file(None).filename_filter(Box::new(|p| {
            Path::new(p)
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("exr"))
        }));

        dialog.open();

        self.skybox_dialog = Some(dialog);
    }

    fn lights(ui: &mut Ui, scene: &mut Scene) {
        ui.vertical(|ui| {
            ui.group(|ui| {
                CollapsingHeader::new(
                    RichText::new(format!("{} ({})", t!("lights"), scene.lights.len())).size(16.0),
                )
                .default_open(true)
                .show_unindented(ui, |ui| {
                    scene
                        .lights
                        .iter_mut()
                        .enumerate()
                        .filter_map(|(n, light)| {
                            let mut remove = false;

                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{} {n}", t!("light")))
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

                            ui.label(format!("{}:", t!("position")));

                            xyz_drag_value(ui, &mut light.position);

                            ui.label(format!("{}:", t!("intensity")));

                            ui.add(
                                Slider::new(&mut light.intensity, 0.0..=100.0)
                                    .clamping(SliderClamping::Always),
                            );

                            ui.label(format!("{}:", t!("color")));

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
                        ui.add(Button::new(RichText::new(t!("add_light"))).frame(false))
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
            });
        });
    }

    fn objects(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.vertical(|ui| {
            ui.group(|ui| {
                CollapsingHeader::new(
                    RichText::new(format!("{} ({})", t!("objects"), scene.objects.len()))
                        .size(16.0),
                )
                .default_open(true)
                .show_unindented(ui, |ui| {
                    let mut objects_to_remove = Vec::new();

                    for (n, o) in scene.objects.iter_mut().enumerate() {
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} ({} ▲)", o.name, o.triangles.len()))
                                    .size(14.0)
                                    .family(FontFamily::Monospace),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add_sized(
                                        [20.0, 20.0],
                                        ImageButton::new(include_image!(
                                            "../../res/icons/trash-solid.svg"
                                        ))
                                        .tint(hex_color!("#cc0000")),
                                    )
                                    .clicked()
                                {
                                    objects_to_remove.push(n);
                                }
                            });
                        });

                        ui.label(format!("{}:", t!("position")));

                        xyz_drag_value(ui, &mut o.translation);

                        ui.label(format!("{}:", t!("rotation")));

                        ui.horizontal(|ui| {
                            let (mut x, mut y, mut z) = o.rotation.euler_angles();

                            [("x", &mut x), ("y", &mut y), ("z", &mut z)]
                                .iter_mut()
                                .any(|(prefix, angle)| {
                                    ui.add(
                                        DragValue::new(*angle)
                                            .speed(0.01)
                                            .custom_formatter(|f, _| {
                                                format!("{:.1}°", f.to_degrees())
                                            })
                                            .prefix(format!("{prefix}: ")),
                                    )
                                    .changed()
                                })
                                .then(|| {
                                    o.rotation =
                                        nalgebra::UnitQuaternion::from_euler_angles(x, y, z);
                                })
                        });

                        ui.label(format!("{}:", t!("scale")));

                        xyz_drag_value(ui, &mut o.scale);
                    }

                    for o in objects_to_remove {
                        scene.objects.remove(o);
                    }

                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if ui
                            .add(Button::new(RichText::new(t!("add_object"))).frame(false))
                            .clicked()
                        {
                            let mut dialog =
                                FileDialog::open_file(None).show_files_filter(Box::new(|path| {
                                    path.extension()
                                        .is_some_and(|ext| ext.eq_ignore_ascii_case("obj"))
                                }));
                            dialog.open();
                            self.object_dialog = Some(dialog);
                        }

                        if let Some(dialog) = &mut self.object_dialog {
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
            });
        });
    }

    const fn format_render_size(size: (u32, u32)) -> &'static str {
        match size {
            (1280, 720) => "HD",
            (1920, 1080) => "FullHD",
            (2560, 1440) => "2k",
            (3840, 2160) => "4k",
            (7680, 4320) => "8k",
            _ => "Custom",
        }
    }
}
