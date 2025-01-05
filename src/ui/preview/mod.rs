use self::gpu::WgpuPainter;
use crate::scene::{Object, Scene, Skybox};
use egui::{
    mutex::RwLock, pos2, Align, Align2, Color32, Context, CursorGrab, DroppedFile, Event, Frame,
    Id, Key, LayerId, Layout, Order, Pos2, Rect, RichText, Sense, Shape, TextStyle, Ui, Vec2,
    ViewportCommand,
};
use egui_wgpu::Callback;
use log::warn;
use nalgebra::{OPoint, Scale3, Translation3, UnitQuaternion};
use rust_i18n::t;
use std::{path::PathBuf, sync::Arc};

pub mod gpu;

#[derive(Clone)]
pub struct Preview {
    // whether the preview is in movement mode
    active: bool,
    speed: f32,
    sensitivity: f32,
    gpu: WgpuPainter,
    dropped_files: Vec<DroppedFile>,
}

impl Preview {
    pub const fn new(scene: Arc<RwLock<Option<Scene>>>) -> Self {
        Self {
            active: false,
            speed: 0.1,
            sensitivity: 0.001,
            gpu: gpu::WgpuPainter::new(scene),
            dropped_files: Vec::new(),
        }
    }

    fn change_preview_movement(&mut self, ui: &Ui, response: &egui::Response, active: bool) {
        self.active = active;

        if active {
            response.request_focus();
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorGrab(CursorGrab::Locked));
        } else {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::CursorGrab(CursorGrab::None));
        }

        ui.ctx()
            .send_viewport_cmd(ViewportCommand::CursorVisible(!active));
    }

    pub fn show(&mut self, ui: &mut Ui, scene: &mut Option<Scene>) {
        Self::show_hover_overlay(ui.ctx(), scene.as_ref(), ui.available_rect_before_wrap());
        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                //self.dropped_files = i.raw.dropped_files.clone();
                self.dropped_files.clone_from(&i.raw.dropped_files);
                if let Some(path) = self.dropped_files.first().and_then(|p| p.path.as_ref()) {
                    Self::handle_file(path, scene);
                }
                self.dropped_files.clear();
            }
        });
        let Some(scene) = scene else {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading(t!("no_scene_loaded"));
                        ui.label(RichText::new(t!("drop_yaml")));
                    });
                });
            });
            return;
        };
        ui.vertical(|ui| {
            let available_size = ui.available_size();
            let aspect_ratio = scene.camera.resolution.0 as f32 / scene.camera.resolution.1 as f32;

            // compute largest rectangle with aspect_ratio that fits in available_size
            let (width, height) = if available_size.x / available_size.y > aspect_ratio {
                (available_size.y * aspect_ratio, available_size.y)
            } else {
                (available_size.x, available_size.x / aspect_ratio)
            };

            Frame::canvas(ui.style())
                .outer_margin(10.0)
                .inner_margin(0.0)
                .fill(match scene.settings.skybox {
                    Skybox::Image { .. } => Color32::GRAY,
                    Skybox::Color(c) => Color32::from_rgb(
                        (c.x * 255.0) as u8,
                        (c.y * 255.0) as u8,
                        (c.z * 255.0) as u8,
                    ),
                })
                .show(ui, |ui| {
                    let (response, painter) = ui.allocate_painter(
                        Vec2 {
                            x: width - 20.0,
                            y: height - 20.0,
                        },
                        Sense::click_and_drag(),
                    );
                    painter.add(Shape::Callback(Callback::new_paint_callback(
                        response.rect,
                        self.gpu.clone(),
                    )));

                    if response.hover_pos().is_some() && !self.active {
                        egui::show_tooltip(
                            ui.ctx(),
                            ui.layer_id(),
                            egui::Id::new("preview_tooltip"),
                            |ui| {
                                ui.label(t!("change_camera_pos"));
                            },
                        );
                    }

                    if response.clicked() {
                        self.change_preview_movement(ui, &response, true);
                    }

                    if self.active {
                        // TODO: do not use debug_text
                        painter.debug_text(
                            pos2(response.rect.left(), response.rect.top()),
                            Align2::LEFT_TOP,
                            Color32::WHITE,
                            format!(
                                "{}\n{} {:.2}\n{} {:.4}\n{}\n{}",
                                t!("wasd"),
                                t!("qe"),
                                self.speed,
                                t!("yc"),
                                self.sensitivity,
                                t!("f"),
                                t!("esc")
                            ),
                        );

                        self.move_camera(ui, &response, scene);
                    }

                    if !response.has_focus() && self.active {
                        // exit movement mode when tabbed out
                        self.change_preview_movement(ui, &response, false);
                    }
                })
        });
    }

    fn handle_file(path: &PathBuf, scene: &mut Option<Scene>) {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("yaml" | "yml") => {
                Scene::load(path).map_or_else(
                    |e| {
                        warn!("Failed to load scene: {}", e);
                    },
                    |s| {
                        scene.replace(s);
                    },
                );
            }
            Some("obj") => {
                if let Some(scene) = scene.as_mut() {
                    match Object::from_obj(
                        path,
                        Translation3::identity(),
                        UnitQuaternion::identity(),
                        Scale3::identity(),
                    ) {
                        Ok(object) => scene.objects.push(object),
                        Err(e) => warn!("Failed to load object: {}", e),
                    }
                }
            }
            _ => {}
        }
    }

    pub fn show_hover_overlay(ctx: &Context, scene: Option<&Scene>, rect: Rect) {
        //TODO: show only when hovering over preview
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            if let Some(hovered) = ctx.input(|i| i.raw.hovered_files.clone()).first() {
                let extension = hovered
                    .path
                    .as_ref()
                    .and_then(|p| p.extension())
                    .and_then(|ext| ext.to_str());
                painter.rect_filled(rect, 0.0, Color32::from_black_alpha(192));
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    match extension {
                        Some("yaml" | "yml") => t!("hov_yaml"),
                        Some("obj") if scene.is_some() => t!("hov_obj"),
                        _ => t!("hov_unknown"),
                    },
                    TextStyle::Heading.resolve(&ctx.style()),
                    Color32::WHITE,
                );
            }
        }
    }

    fn move_camera(&mut self, ui: &Ui, response: &egui::Response, scene: &mut Scene) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.active {
            // exit movement mode using ESC
            self.change_preview_movement(ui, response, false);
        }

        // compute mouse movement
        let delta = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    &Event::PointerMoved(pos) => Some(response.rect.center() - pos),
                    _ => None,
                })
                .fold(Pos2::ZERO, |acc, x| acc + x)
        });

        let direction = (scene.camera.look_at - scene.camera.position).normalize();
        let right = direction.cross(&scene.camera.up).normalize();
        let up = right.cross(&direction).normalize();

        // move mouse to center
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::CursorPosition(
                response.rect.center(),
            ));

        // move look_at point in a sphere around camera with constant distance 1 using mouse
        let new_point = scene.camera.position + direction - (right * delta.x * self.sensitivity)
            + (up * delta.y * self.sensitivity);
        scene.camera.look_at =
            scene.camera.position + (new_point - scene.camera.position).normalize();

        scene.camera.fov = ui
            .input(|i| i.raw_scroll_delta.y)
            .mul_add(-0.001, scene.camera.fov)
            .clamp(0.0_f32.to_radians(), 180.0_f32.to_radians());

        // compute movement
        ui.input(|i| {
            i.key_down(Key::F).then(|| {
                scene.camera.look_at = OPoint::origin();
            });

            i.key_pressed(Key::Y).then(|| {
                self.sensitivity = (self.sensitivity - 0.0001_f32).max(0.0);
                warn!("Look sensitivity: {}", self.sensitivity);
            });
            i.key_pressed(Key::C).then(|| {
                self.sensitivity = (self.sensitivity + 0.0001_f32).min(0.5);
                warn!("Look sensitivity: {}", self.sensitivity);
            });
            i.key_down(Key::Q).then(|| {
                self.speed = (self.speed - 0.005_f32).max(0.0);
                warn!("Movement speed: {}", self.speed);
            });
            i.key_down(Key::E).then(|| {
                self.speed = (self.speed + 0.005_f32).min(1.0);
                warn!("Movement speed: {}", self.speed);
            });

            i.key_down(Key::W).then(|| {
                scene.camera.position += direction * self.speed;
                scene.camera.look_at += direction * self.speed;
            });
            i.key_down(Key::S).then(|| {
                scene.camera.position -= direction * self.speed;
                scene.camera.look_at -= direction * self.speed;
            });
            i.key_down(Key::A).then(|| {
                scene.camera.position -= right * self.speed;
                scene.camera.look_at -= right * self.speed;
            });
            i.key_down(Key::D).then(|| {
                scene.camera.position += right * self.speed;
                scene.camera.look_at += right * self.speed;
            });
            i.key_down(Key::Space).then(|| {
                scene.camera.position += scene.camera.up * self.speed;
                scene.camera.look_at += scene.camera.up * self.speed;
            });
            i.modifiers.shift.then(|| {
                scene.camera.position -= scene.camera.up * self.speed;
                scene.camera.look_at -= scene.camera.up * self.speed;
            });
        });
    }
}
