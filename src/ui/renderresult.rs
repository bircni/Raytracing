use egui::{pos2, Color32, CursorIcon, Frame, Rect, Rounding, Sense, Stroke, Ui, Vec2};

use super::{preview::Preview, render::Render};

pub struct RenderResult {}

impl RenderResult {
    pub fn render_result(ui: &mut Ui, render: &Render, preview: &mut Preview) {
        Frame::canvas(ui.style()).outer_margin(10.0).show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::drag());

            let response = response.on_hover_and_drag_cursor(CursorIcon::Grab);

            preview.zoom += ui.input(|i| i.scroll_delta.y);
            preview.zoom = preview.zoom.clamp(
                -response.rect.width().min(response.rect.height()) / 4.0,
                std::f32::INFINITY,
            );
            preview.position += response.drag_delta();

            response.double_clicked().then(|| {
                preview.zoom = 0.0;
                preview.position = Vec2::ZERO;
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

            let render_aspect = render.rsize.as_size().0 as f32 / render.rsize.as_size().1 as f32;
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
                rect.translate(preview.position)
                    .expand2(Vec2::new(preview.zoom * render_aspect, preview.zoom)),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        });
    }
}
