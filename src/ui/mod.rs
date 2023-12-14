mod preview;
mod properties;
mod render;

use self::preview::Preview;

use crate::scene::Scene;
use anyhow::Context;
use eframe::CreationContext;
use egui::{
    mutex::Mutex, pos2, Align, CursorIcon, Frame, Layout, ProgressBar, Rect, Rounding, Stroke, Vec2,
};
use egui::{
    Button, CentralPanel, Color32, ColorImage, ImageData, Key, Sense, TextStyle, TextureHandle,
    TextureOptions, Ui,
};
use egui_file::FileDialog;
use image::{ImageBuffer, RgbImage};
use log::{info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

#[derive(PartialEq, Eq)]
enum RenderSize {
    FullHD,
    Wqhd,
    Uhd1,
    Uhd2,
    Custom([u32; 2]),
}

impl RenderSize {
    fn as_size(&self) -> (u32, u32) {
        match self {
            RenderSize::FullHD => (1920, 1080),
            RenderSize::Wqhd => (2560, 1440),
            RenderSize::Uhd1 => (3840, 2160),
            RenderSize::Uhd2 => (7680, 4320),
            &RenderSize::Custom([x, y]) => (x, y),
        }
    }
}

impl std::fmt::Display for RenderSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderSize::FullHD => write!(f, "Full HD"),
            RenderSize::Wqhd => write!(f, "2k"),
            RenderSize::Uhd1 => write!(f, "4k"),
            RenderSize::Uhd2 => write!(f, "8k"),
            RenderSize::Custom(_) => write!(f, "Custom"),
        }
    }
}

pub struct App {
    current_tab: usize,
    scene: Scene,
    render_texture: TextureHandle,
    rendering_thread: Option<std::thread::JoinHandle<()>>,
    rendering_cancel: Arc<AtomicBool>,
    render_image: Arc<Mutex<RgbImage>>,
    opened_file: Option<PathBuf>,
    open_file_dialog: Option<FileDialog>,
    save_image_dialog: Option<FileDialog>,
    render_size: RenderSize,
    rendering_progress: Arc<AtomicU16>,
    preview_zoom: f32,
    preview_position: Vec2,
    preview_activate_movement: bool,
    movement_speed: f32,
    look_sensitivity: f32,
    pause_delta: bool,
    pause_count: i32,
}

impl App {
    pub fn new(cc: &CreationContext, scene: Scene) -> anyhow::Result<Self> {
        egui_extras::install_image_loaders(&cc.egui_ctx);

        Preview::init(
            cc.wgpu_render_state
                .as_ref()
                .context("Failed to get wgpu context")?,
        );
        let render_size = RenderSize::FullHD;

        let (render_texture, image_buffer) = {
            let render_size = render_size.as_size();
            let texture = cc.egui_ctx.load_texture(
                "render",
                ImageData::Color(Arc::new(ColorImage {
                    size: [render_size.0 as usize, render_size.1 as usize],
                    pixels: vec![Color32::BLACK; (render_size.0 * render_size.1) as usize],
                })),
                TextureOptions::default(),
            );
            let image_buffer = Arc::new(Mutex::new(ImageBuffer::new(render_size.0, render_size.1)));
            (texture, image_buffer)
        };

        cc.egui_ctx.style_mut(|s| {
            s.text_styles.insert(
                TextStyle::Name("subheading".into()),
                TextStyle::Monospace.resolve(s),
            );
        });

        Ok(Self {
            current_tab: 0,
            scene,
            preview_zoom: 0.0,
            preview_position: Vec2::ZERO,
            render_texture,
            rendering_thread: None,
            opened_file: None,
            open_file_dialog: None,
            save_image_dialog: None,
            render_size,
            rendering_progress: Arc::new(AtomicU16::new(0)),
            rendering_cancel: Arc::new(AtomicBool::new(false)),
            render_image: image_buffer,
            preview_activate_movement: false,
            movement_speed: 0.1,
            look_sensitivity: 0.001,
            pause_delta: false,
            pause_count: 0,
        })
    }

    fn export_button(&mut self, ui: &mut Ui) {
        if ui
            .add_enabled(
                self.rendering_progress.load(Ordering::Relaxed) == u16::MAX,
                Button::new("Export"),
            )
            .clicked()
        {
            info!("Exporting image");
            self.save_image_dialog
                .get_or_insert_with(|| {
                    FileDialog::save_file(None)
                        .default_filename("render.png")
                        .filename_filter(Box::new(|name| {
                            [".png", ".jpg", ".jpeg"]
                                .into_iter()
                                .any(|ext| name.ends_with(ext))
                        }))
                })
                .open();
        }

        if let Some(dialog) = &mut self.save_image_dialog {
            if dialog.show(ui.ctx()).selected() {
                if let Some(file) = dialog.path() {
                    log::info!("Saving image to {:?}", file);
                    self.render_image.lock().save(file).unwrap_or_else(|e| {
                        warn!("Failed to save image: {}", e);
                    });
                }
            }
        }
    }

    fn render_button(&mut self, ui: &mut Ui) {
        if self.rendering_thread.is_some() {
            ui.button("Cancel").clicked().then(|| {
                self.rendering_cancel.store(true, Ordering::Relaxed);
            });
        } else {
            ui.add_enabled_ui(self.rendering_thread.is_none(), |ui| {
                ui.button("Render").clicked().then(|| {
                    self.render(ui.ctx().clone());
                    self.current_tab = 1;
                })
            });
        }
    }

    fn preview(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style())
            .outer_margin(10.0)
            .fill(Color32::from_rgb(
                (self.scene.settings.background_color[0] * 255.0) as u8,
                (self.scene.settings.background_color[1] * 255.0) as u8,
                (self.scene.settings.background_color[2] * 255.0) as u8,
            ))
            .show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
                painter.add(Preview::paint(response.rect, &self.scene));
                if response.clicked() {
                    self.preview_activate_movement = true;
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CursorGrab(
                            egui::CursorGrab::Confined,
                        ));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
                }
                if self.preview_activate_movement {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
                    self.move_camera(ui, &response);
                }
            });
    }

    fn move_camera(&mut self, ui: &mut Ui, response: &egui::Response) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.preview_activate_movement {
            // exit movement mode using ESC
            self.preview_activate_movement = false;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
        }
        if response.hover_pos().is_none() {
            // move mouse to center
            self.pause_delta = true;
            let center = response.rect.center();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorPosition(center));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(
                    egui::CursorGrab::Confined,
                ));
        }
        let delta = ui.input(|i| i.pointer.delta()); // rotate look_at point around camera position using mouse
        let direction = (self.scene.camera.look_at - self.scene.camera.position).normalize();
        let right = direction.cross(&self.scene.camera.up).normalize();
        let up = right.cross(&direction).normalize();
        if self.pause_delta {
            self.pause_count += 1;
            if self.pause_count > 5 {
                self.pause_delta = false;
                self.pause_count = 0;
            }
        }
        if !self.pause_delta {
            // move look_at point in a sphere around camera with constant distance 1 using mouse
            let new_point =
                self.scene.camera.position + direction + right * delta.x * self.look_sensitivity
                    - up * delta.y * self.look_sensitivity;
            self.scene.camera.look_at =
                self.scene.camera.position + (new_point - self.scene.camera.position).normalize();
        }
        self.scene.camera.fov = (self.scene.camera.fov - (ui.input(|i| i.scroll_delta.y) * 0.001))
            .clamp(0.0_f32.to_radians(), 180.0_f32.to_radians());
        // movement using keyboard
        if ui.input(|i| i.key_pressed(egui::Key::Y)) {
            // lower sensitivity and clamp so it cant go negative
            self.look_sensitivity = (self.look_sensitivity - 0.0001_f32).max(0.0);
            warn!("Look sensitivity: {}", self.look_sensitivity);
        }
        if ui.input(|i| i.key_pressed(egui::Key::C)) {
            // higher sensitivity and clamp so it cant go too high
            self.look_sensitivity = (self.look_sensitivity + 0.0001_f32).min(0.5);
            warn!("Look sensitivity: {}", self.look_sensitivity);
        }
        if ui.input(|i| i.key_pressed(egui::Key::Q)) {
            // lower movement speed and clamp so it cant go negative
            self.movement_speed = (self.movement_speed - 0.005_f32).max(0.0);
            warn!("Movement speed: {}", self.movement_speed);
        }
        if ui.input(|i| i.key_pressed(egui::Key::E)) {
            // higher movement speed and clamp so it cant go too high
            self.movement_speed = (self.movement_speed + 0.005_f32).min(1.0);
            warn!("Movement speed: {}", self.movement_speed);
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowUp)) {
            // look up
            self.scene.camera.look_at += up * self.look_sensitivity;
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowDown)) {
            // look down
            // calculate up vector of camera
            self.scene.camera.look_at -= up * self.look_sensitivity;
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowLeft)) {
            // look left
            self.scene.camera.look_at -= right * self.look_sensitivity;
        }
        if ui.input(|i| i.key_down(egui::Key::ArrowRight)) {
            // look right
            self.scene.camera.look_at += right * self.look_sensitivity;
        }
        if ui.input(|i| i.key_down(egui::Key::W)) {
            // move camera forward
            self.scene.camera.position += direction * self.movement_speed;
            self.scene.camera.look_at += direction * self.movement_speed;
        }
        if ui.input(|i| i.key_down(egui::Key::S)) {
            // move camera backward
            self.scene.camera.position -= direction * self.movement_speed;
            self.scene.camera.look_at -= direction * self.movement_speed;
        }
        if ui.input(|i| i.key_down(egui::Key::A)) {
            // move camera left
            self.scene.camera.position -= right * self.movement_speed;
            self.scene.camera.look_at -= right * self.movement_speed;
        }
        if ui.input(|i| i.key_down(egui::Key::D)) {
            // move camera right
            self.scene.camera.position += right * self.movement_speed;
            self.scene.camera.look_at += right * self.movement_speed;
        }
        if ui.input(|i| i.key_down(egui::Key::Space)) {
            // move camera up
            self.scene.camera.position += self.scene.camera.up * self.movement_speed;
            self.scene.camera.look_at += self.scene.camera.up * self.movement_speed;
        }
        if ui.input(|i| i.modifiers.shift) {
            // move camera down
            self.scene.camera.position -= self.scene.camera.up * self.movement_speed;
            self.scene.camera.look_at -= self.scene.camera.up * self.movement_speed;
        }
    }

    fn render_result(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style()).outer_margin(10.0).show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

            let response = response.on_hover_and_drag_cursor(CursorIcon::Grab);

            self.preview_zoom += ui.input(|i| i.scroll_delta.y);
            self.preview_zoom = self.preview_zoom.clamp(
                -response.rect.width().min(response.rect.height()) / 4.0,
                std::f32::INFINITY,
            );
            self.preview_position += response.drag_delta();

            response.double_clicked().then(|| {
                self.preview_zoom = 0.0;
                self.preview_position = Vec2::ZERO;
            });

            // paint gray grid
            let cell_size = 25.0;
            for y in 0..=response.rect.height() as usize / cell_size as usize {
                for x in 0..=response.rect.width() as usize / cell_size as usize {
                    painter.rect(
                        Rect::from_min_size(
                            pos2(
                                response.rect.left() + x as f32 * cell_size,
                                response.rect.top() + y as f32 * cell_size,
                            ),
                            Vec2::splat(cell_size),
                        ),
                        Rounding::default(),
                        if (x + y) % 2 == 0 {
                            Color32::GRAY
                        } else {
                            Color32::DARK_GRAY
                        },
                        Stroke::NONE,
                    );
                }
            }

            let render_aspect =
                self.render_size.as_size().0 as f32 / self.render_size.as_size().1 as f32;
            let rect = Rect::from_min_size(
                response.rect.min,
                // keep aspect ratio
                Vec2::new(
                    response
                        .rect
                        .width()
                        .min(response.rect.height() * render_aspect),
                    response
                        .rect
                        .height()
                        .min(response.rect.width() / render_aspect),
                ),
            );

            // center rect
            let rect = Rect::from_min_size(
                rect.min + (response.rect.size() - rect.size()) / 2.0,
                rect.size(),
            );

            painter.image(
                self.render_texture.id(),
                rect.translate(self.preview_position).expand2(Vec2::new(
                    self.preview_zoom * render_aspect,
                    self.preview_zoom,
                )),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        });
    }

    fn save_scene(&mut self) {
        serde_yaml::to_string(&self.scene)
            .context("Failed to serialize scene")
            .and_then(|str| std::fs::write("res/config.yaml", str).context("Failed to save config"))
            .unwrap_or_else(|e| {
                warn!("Failed to save config: {}", e);
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rendering_thread
            .as_ref()
            .is_some_and(JoinHandle::is_finished)
            .then(|| {
                self.rendering_thread = None;
                self.rendering_cancel.store(false, Ordering::Relaxed);
            });

        ctx.input(|input| input.key_pressed(Key::S) && input.modifiers.ctrl)
            .then(|| {
                self.save_scene();
            });

        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_label(self.current_tab == 0, "Preview")
                    .clicked()
                    .then(|| {
                        self.current_tab = 0;
                    });

                ui.selectable_label(self.current_tab == 1, "Render")
                    .clicked()
                    .then(|| {
                        self.current_tab = 1;
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    self.export_button(ui);
                    self.render_button(ui);

                    ui.add(
                        ProgressBar::new(
                            f32::from(self.rendering_progress.load(Ordering::Relaxed))
                                / f32::from(u16::MAX),
                        )
                        .desired_width(ui.available_width() / 3.0)
                        .show_percentage()
                        .fill(Color32::DARK_BLUE),
                    );

                    ui.label("Rendering progress");
                });
            });

            ui.vertical_centered(|ui| {
                ui.separator();
            });

            match self.current_tab {
                0 => {
                    self.properties(ui);

                    self.preview(ui);
                }

                1 => {
                    self.render_result(ui);
                }
                n => unreachable!("Invalid tab index {}", n),
            }
        });
    }
}
