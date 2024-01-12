use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use egui::{Color32, ColorImage, ImageData, TextureOptions};

use log::{debug, info};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};

use crate::raytracer::Raytracer;

impl super::App {
    #[allow(clippy::too_many_lines)]
    pub fn render(&mut self, ctx: egui::Context) {
        let render_size = self.render_size.as_size();
        self.render_texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: [render_size.0 as usize, render_size.1 as usize],
                pixels: vec![Color32::BLACK; (render_size.0 * render_size.1) as usize],
            })),
            TextureOptions::default(),
        );

        let texture = self.render_texture.clone();
        let raytracer = Raytracer::new(self.scene.clone(), 1e-5, 5);

        let block_size = [render_size.0 / 20, render_size.1 / 20];

        let rendering_progress = self.rendering_progress.clone();
        let rendering_time = self.rendering_time.clone();
        let rendering_cancel = self.rendering_cancel.clone();

        let image_buffer = self.render_image.clone();

        rendering_progress.store(0, Ordering::Relaxed);
        rendering_time.store(0, Ordering::Relaxed);

        self.rendering_thread = Some(std::thread::spawn(move || {
            let start = std::time::Instant::now();

            let blocks = AtomicUsize::new(0);
            (0..render_size.1 / block_size[1])
                .flat_map(|y_block| {
                    (0..render_size.0 / block_size[0]).map(move |x_block| (x_block, y_block))
                })
                .par_bridge()
                .take_any_while(|_| !rendering_cancel.load(Ordering::Relaxed))
                .map(|(x_block, y_block)| {
                    debug!(
                        "rendering block ({}, {}) of ({}, {}) ({:.2}%)",
                        x_block,
                        y_block,
                        render_size.0 / block_size[0],
                        render_size.1 / block_size[1],
                        (x_block + y_block * render_size.0 / block_size[0]) as f32
                            / (render_size.0 / block_size[0] * render_size.1 / block_size[1])
                                as f32
                            * 100.0
                    );

                    let pixels = (0..block_size[0] * block_size[1])
                        .into_par_iter()
                        .map(|i| {
                            let x = i % block_size[0] + x_block * block_size[0];
                            let y = i / block_size[0] + y_block * block_size[1];
                            raytracer.render((x, y), (render_size.0, render_size.1))
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
                            / (render_size.0 / block_size[0] * render_size.1 / block_size[1])
                                as f32
                            * f32::from(u16::MAX))
                        .round() as u16,
                        Ordering::Relaxed,
                    );

                    (pixels, x_block, y_block)
                })
                .take_any_while(|_| !rendering_cancel.load(Ordering::Relaxed))
                .for_each_with(texture, |texture, (pixels, x_block, y_block)| {
                    texture.set_partial(
                        [
                            (x_block * block_size[0]) as usize,
                            (y_block * block_size[1]) as usize,
                        ],
                        ImageData::Color(Arc::new(ColorImage {
                            size: [block_size[0] as usize, block_size[1] as usize],
                            pixels: pixels.clone(),
                        })),
                        TextureOptions::default(),
                    );
                    let mut image = image_buffer.lock();
                    for x in 0..block_size[0] {
                        for y in 0..block_size[1] {
                            image.put_pixel(
                                x_block * block_size[0] + x,
                                y_block * block_size[1] + y,
                                image::Rgb([
                                    pixels[(x + y * block_size[0]) as usize].r(),
                                    pixels[(x + y * block_size[0]) as usize].g(),
                                    pixels[(x + y * block_size[0]) as usize].b(),
                                ]),
                            );
                        }
                    }
                    ctx.request_repaint();
                });

            rendering_progress.store(u16::MAX, Ordering::Relaxed);
            rendering_time.store(start.elapsed().as_millis() as u32, Ordering::Relaxed);

            info!("rendering finished: {:?}", start.elapsed());
        }));
    }
}
