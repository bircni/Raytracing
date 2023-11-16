use std::{
    borrow::BorrowMut,
    sync::{atomic::Ordering, Arc},
};

use egui::{Color32, ColorImage, ImageData, TextureOptions};
use log::info;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{raytracer::Raytracer, Color};

impl super::App {
    pub fn render(&mut self, ctx: egui::Context) {
        self.render_texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: self.render_size,
                pixels: vec![Color32::BLACK; self.render_size[0] * self.render_size[1]],
            })),
            TextureOptions::default(),
        );

        let mut texture = self.render_texture.clone();
        let raytracer = Raytracer::new(self.scene.clone(), Color::new(0.1, 0.1, 0.1), 1e-6);

        let render_size = self.render_size;
        let block_size = [render_size[0] / 10, render_size[1] / 10];
        let rendering_progress = self.rendering_progress.clone();

        rendering_progress.store(0, Ordering::Relaxed);

        self.rendering_thread = Some(std::thread::spawn(move || {
            let start = std::time::Instant::now();

            for y_block in 0..render_size[1] / block_size[1] {
                for x_block in 0..render_size[0] / block_size[0] {
                    info!(
                        "rendering block ({}, {}) of ({}, {}) ({:.2}%)",
                        x_block,
                        y_block,
                        render_size[0] / block_size[0],
                        render_size[1] / block_size[1],
                        (x_block + y_block * render_size[0] / block_size[0]) as f32
                            / (render_size[0] / block_size[0] * render_size[1] / block_size[1])
                                as f32
                            * 100.0
                    );

                    let pixels = (0..block_size[0] * block_size[1])
                        .into_par_iter()
                        .map(|i| {
                            let x = i % block_size[0] + x_block * block_size[0];
                            let y = i / block_size[0] + y_block * block_size[1];
                            raytracer.render((x, y), (render_size[0], render_size[1]))
                        })
                        .map(|c| {
                            Color32::from_rgb(
                                (c.x * 255.0) as u8,
                                (c.y * 255.0) as u8,
                                (c.z * 255.0) as u8,
                            )
                        })
                        .collect::<Vec<_>>();

                    texture.borrow_mut().set_partial(
                        [x_block * block_size[0], y_block * block_size[1]],
                        ImageData::Color(Arc::new(ColorImage {
                            size: block_size,
                            pixels,
                        })),
                        TextureOptions::default(),
                    );

                    rendering_progress.store(
                        (((x_block + y_block * render_size[0] / block_size[0]) as f32
                            / (render_size[0] / block_size[0] * render_size[1] / block_size[1])
                                as f32)
                            * u16::MAX as f32)
                            .round() as u16,
                        Ordering::Relaxed,
                    );

                    ctx.request_repaint();
                }
            }

            rendering_progress.store(u16::MAX, Ordering::Relaxed);

            info!("rendering finished: {:?}", start.elapsed());
        }));
    }
}
