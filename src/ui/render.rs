use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use egui::{Color32, ColorImage, ImageData, TextureOptions};

use log::{debug, info};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};

use crate::raytracer::Raytracer;

impl super::App {
    pub fn render(&mut self, ctx: egui::Context) {
        self.render_texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: self.render_size,
                pixels: vec![Color32::BLACK; self.render_size[0] * self.render_size[1]],
            })),
            TextureOptions::default(),
        );

        let texture = self.render_texture.clone();
        let raytracer = Raytracer::new(self.scene.clone(), 1e-5);

        let render_size = self.render_size;
        let block_size = [render_size[0] / 10, render_size[1] / 10];
        let rendering_progress = self.rendering_progress.clone();
        let image_buffer = self.image_buffer.clone();

        rendering_progress.store(0, Ordering::Relaxed);

        self.rendering_thread = Some(std::thread::spawn(move || {
            let start = std::time::Instant::now();

            let blocks = AtomicUsize::new(0);
            (0..render_size[1] / block_size[1])
                .flat_map(|y_block| {
                    (0..render_size[0] / block_size[0]).map(move |x_block| (x_block, y_block))
                })
                .par_bridge()
                .map(|(x_block, y_block)| {
                    debug!(
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

                    rendering_progress.store(
                        ((blocks.fetch_add(1, Ordering::Relaxed) as f32)
                            / (render_size[0] / block_size[0] * render_size[1] / block_size[1])
                                as f32
                            * u16::MAX as f32)
                            .round() as u16,
                        Ordering::Relaxed,
                    );

                    (pixels, x_block, y_block)
                })
                .for_each_with(texture, |texture, (pixels, x_block, y_block)| {
                    texture.set_partial(
                        [x_block * block_size[0], y_block * block_size[1]],
                        ImageData::Color(Arc::new(ColorImage {
                            size: block_size,
                            pixels: pixels.clone(),
                        })),
                        TextureOptions::default(),
                    );
                    let mut bufferlock = image_buffer.lock().unwrap();
                    for x in 0..block_size[0] {
                        for y in 0..block_size[1] {
                            bufferlock.put_pixel(
                                (x_block * block_size[0] + x) as u32,
                                (y_block * block_size[1] + y) as u32,
                                image::Rgb([
                                    pixels[x + y * block_size[0]].r(),
                                    pixels[x + y * block_size[0]].g(),
                                    pixels[x + y * block_size[0]].b(),
                                ]),
                            );
                        }
                    }
                    ctx.request_repaint();
                });

            rendering_progress.store(u16::MAX, Ordering::Relaxed);

            info!("rendering finished: {:?}", start.elapsed());
        }));
    }
}
