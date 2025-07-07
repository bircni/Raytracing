use crate::{raytracer::render::Render, scene::Scene};
use egui::{Color32, CornerRadius, CursorIcon, Frame, Rect, Sense, Stroke, Ui, Vec2, pos2};

pub struct RenderResult {
    // zoom factor where 0 is no zoom
    zoom: f32,
    position: Vec2,
}

impl RenderResult {
    pub const fn new() -> Self {
        Self {
            zoom: 0.0,
            position: Vec2::ZERO,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, render: &Render) {
        Frame::canvas(ui.style()).outer_margin(10.0).show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

            let response = response.on_hover_and_drag_cursor(CursorIcon::Grab);

            // Check if the dialog is being hovered over or active
            if response.has_focus() || response.hovered() {
                self.zoom += ui.input(|i| i.raw_scroll_delta.y);
                self.zoom = self.zoom.clamp(
                    -response.rect.width().min(response.rect.height()) / 4.0,
                    f32::INFINITY,
                );
                self.position += response.drag_delta();
            }

            response.double_clicked().then(|| {
                self.zoom = 0.0;
                self.position = Vec2::ZERO;
            });

            // paint gray grid
            let cell_size = 25.0;
            for y in 0..=response.rect.height() as usize / cell_size as usize {
                for x in 0..=response.rect.width() as usize / cell_size as usize {
                    painter.rect(
                        Rect::from_min_size(
                            pos2(
                                (x as f32).mul_add(cell_size, response.rect.left()),
                                (y as f32).mul_add(cell_size, response.rect.top()),
                            ),
                            Vec2::splat(cell_size),
                        ),
                        CornerRadius::default(),
                        if (x + y).is_multiple_of(2) {
                            Color32::GRAY
                        } else {
                            Color32::DARK_GRAY
                        },
                        Stroke::NONE,
                        egui::StrokeKind::Outside,
                    );
                }
            }

            let render_aspect = scene.camera.resolution.0 as f32 / scene.camera.resolution.1 as f32;
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
                render.texture.id(),
                rect.translate(self.position)
                    .expand2(Vec2::new(self.zoom * render_aspect, self.zoom)),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        });
    }
}
