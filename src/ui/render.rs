use std::sync::{
    atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering},
    Arc,
};

use egui::{mutex::Mutex, Color32, ColorImage, ImageData, TextureHandle, TextureOptions};

use image::RgbImage;
use log::{debug, info};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};

use crate::{raytracer::Raytracer, scene::Scene};

pub struct Render {
    pub texture: TextureHandle,
    pub progress: Arc<AtomicU16>,
    pub thread: Option<std::thread::JoinHandle<()>>,
    pub cancel: Arc<AtomicBool>,
    pub rimage: Arc<Mutex<RgbImage>>,
    pub time: Arc<AtomicU32>,
}

impl Render {
    pub fn new(texture: TextureHandle, rimage: Arc<Mutex<RgbImage>>) -> Self {
        Self {
            texture,
            progress: Arc::new(AtomicU16::new(0)),
            thread: None,
            cancel: Arc::new(AtomicBool::new(false)),
            rimage,
            time: Arc::new(AtomicU32::new(0)),
        }
    }
    pub fn render(&mut self, ctx: egui::Context, scene: &Scene) {
        let rsize = scene.camera.resolution;
        self.texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: [rsize.0 as usize, rsize.1 as usize],
                pixels: vec![Color32::BLACK; (rsize.0 * rsize.1) as usize],
            })),
            TextureOptions::default(),
        );

        self.progress.store(0, Ordering::Relaxed);
        self.time.store(0, Ordering::Relaxed);

        let args = ThreadArgs {
            rsize,
            rendering_cancel: self.cancel.clone(),
            ctx,
            scene: scene.clone(),
            rendering_progress: self.progress.clone(),
            texture: self.texture.clone(),
            image_buffer: self.rimage.clone(),
            rendering_time: self.time.clone(),
        };

        self.thread = Some(std::thread::spawn(move || {
            rendering_thread(args);
        }));
    }
}

struct ThreadArgs {
    ctx: egui::Context,
    scene: Scene,
    texture: TextureHandle,
    image_buffer: Arc<Mutex<RgbImage>>,
    rsize: (u32, u32),
    rendering_cancel: Arc<AtomicBool>,
    rendering_progress: Arc<AtomicU16>,
    rendering_time: Arc<AtomicU32>,
}

fn rendering_thread(
    ThreadArgs {
        rsize,
        rendering_cancel,
        ctx,
        scene,
        rendering_progress,
        texture,
        image_buffer,
        rendering_time,
    }: ThreadArgs,
) {
    let start = std::time::Instant::now();
    let block_size = [rsize.0 / 20, rsize.1 / 20];
    let raytracer = Raytracer::new(scene, 1e-5, 5);
    let blocks = AtomicUsize::new(0);
    (0..rsize.1 / block_size[1])
        .flat_map(|y_block| (0..rsize.0 / block_size[0]).map(move |x_block| (x_block, y_block)))
        .par_bridge()
        .take_any_while(|_| !rendering_cancel.load(Ordering::Relaxed))
        .map(|(x_block, y_block)| {
            debug!(
                "rendering block ({}, {}) of ({}, {}) ({:.2}%)",
                x_block,
                y_block,
                rsize.0 / block_size[0],
                rsize.1 / block_size[1],
                (x_block + y_block * rsize.0 / block_size[0]) as f32
                    / (rsize.0 / block_size[0] * rsize.1 / block_size[1]) as f32
                    * 100.0
            );

            let pixels = (0..block_size[0] * block_size[1])
                .into_par_iter()
                .map(|i| {
                    let x = i % block_size[0] + x_block * block_size[0];
                    let y = i / block_size[0] + y_block * block_size[1];
                    raytracer.render((x, y), (rsize.0, rsize.1))
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
                    / (rsize.0 / block_size[0] * rsize.1 / block_size[1]) as f32
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
}
