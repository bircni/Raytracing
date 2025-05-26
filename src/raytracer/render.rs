use crate::{raytracer::Raytracer, scene::Scene};
use egui::{Color32, ColorImage, ImageData, TextureHandle, TextureOptions, mutex::Mutex};
use image::RgbImage;
use log::{debug, info};
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicUsize, Ordering},
    },
    thread,
    time::Instant,
};

pub struct Render {
    pub texture: TextureHandle,
    /// Progress of the rendering in the range [0, `u16::MAX`]
    pub progress: Arc<AtomicU16>,
    pub thread: Option<thread::JoinHandle<()>>,
    /// Cancel the rendering if true
    pub cancel: Arc<AtomicBool>,
    pub image: Arc<Mutex<RgbImage>>,
    /// Write the rendering time in milliseconds
    pub time: Arc<AtomicU32>,
}

impl Render {
    pub fn new(texture: TextureHandle, image: Arc<Mutex<RgbImage>>) -> Self {
        Self {
            texture,
            progress: Arc::new(AtomicU16::new(0)),
            thread: None,
            cancel: Arc::new(AtomicBool::new(false)),
            image,
            time: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn render(&mut self, ctx: egui::Context, scene: &Scene) {
        let rsize = scene.camera.resolution;
        info!("Rendering scene with resolution {rsize:?}");

        // resize texture and image buffer to match the new resolution
        self.texture.set(
            ImageData::Color(Arc::new(ColorImage {
                size: [rsize.0 as usize, rsize.1 as usize],
                pixels: vec![Color32::BLACK; (rsize.0 * rsize.1) as usize],
            })),
            TextureOptions::default(),
        );
        *self.image.lock() = RgbImage::new(rsize.0, rsize.1);

        // reset progress and time
        self.progress.store(0, Ordering::Relaxed);
        self.time.store(0, Ordering::Relaxed);

        let args = RenderingThread {
            cancel: Arc::<AtomicBool>::clone(&self.cancel),
            ctx,
            scene: scene.clone(),
            progress: Arc::<AtomicU16>::clone(&self.progress),
            texture: self.texture.clone(),
            image: Arc::<Mutex<RgbImage>>::clone(&self.image),
            time: Arc::<AtomicU32>::clone(&self.time),
        };

        // spawn rendering thread
        self.thread = Some(thread::spawn(move || {
            args.run();
        }));
    }
}

struct RenderingThread {
    ctx: egui::Context,
    scene: Scene,
    /// egui Texture (GPU exclusive)
    texture: TextureHandle,
    /// image data (CPU exclusive)
    image: Arc<Mutex<RgbImage>>,
    /// Cancel the rendering if true
    cancel: Arc<AtomicBool>,
    /// Progress of the rendering in the range [0, `u16::MAX`]
    progress: Arc<AtomicU16>,
    /// Write the rendering time in milliseconds
    time: Arc<AtomicU32>,
}

impl RenderingThread {
    #[expect(
        clippy::significant_drop_tightening,
        reason = "no need to drop the texture"
    )]
    /// main rendering thread
    fn run(self) {
        let start = Instant::now();

        let (width, height) = self.image.lock().dimensions();

        // TODO: make block size adaptive to the resolution
        // this will currently cause unrendered pixels if
        // the resolution is not a multiple of 20
        let block_size = [width / 20, height / 20];
        let anti_aliasing = self.scene.settings.anti_aliasing;
        let raytracer = Raytracer::new(self.scene, 1e-5, 5);

        let blocks_rendered = AtomicUsize::new(0);

        (0..height / block_size[1])
            .flat_map(|y_block| (0..width / block_size[0]).map(move |x_block| (x_block, y_block)))
            // parallelize iterator over blocks
            .par_bridge()
            .take_any_while(|_| !self.cancel.load(Ordering::Relaxed))
            .map(|(x_block, y_block)| {
                debug!(
                    "rendering block ({}, {}) of ({}, {}) ({:.2}%)",
                    x_block,
                    y_block,
                    width / block_size[0],
                    height / block_size[1],
                    (x_block + y_block * width / block_size[0]) as f32
                        / (width / block_size[0] * height / block_size[1]) as f32
                        * 100.0
                );

                let pixels = (0..block_size[0] * block_size[1])
                    // parallelize over pixels
                    .into_par_iter()
                    .map(|i| {
                        let x = i % block_size[0] + x_block * block_size[0];
                        let y = i / block_size[0] + y_block * block_size[1];
                        raytracer.render((x, y), (width, height), anti_aliasing)
                    })
                    .map(|c| {
                        Color32::from_rgb(
                            (c.x * 255.0) as u8,
                            (c.y * 255.0) as u8,
                            (c.z * 255.0) as u8,
                        )
                    })
                    .collect::<Vec<_>>();

                self.progress.store(
                    ((blocks_rendered.fetch_add(1, Ordering::Relaxed) as f32)
                        / (width / block_size[0] * height / block_size[1]) as f32
                        * f32::from(u16::MAX))
                    .round() as u16,
                    Ordering::Relaxed,
                );

                (pixels, x_block, y_block)
            })
            // take while not cancelled
            .take_any_while(|_| !self.cancel.load(Ordering::Relaxed))
            .for_each_with(self.texture, |texture, (pixels, x_block, y_block)| {
                // copy pixels to texture
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

                // copy pixels to image
                let mut image = self.image.lock();
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

                self.ctx.request_repaint();
            });

        self.progress.store(u16::MAX, Ordering::Relaxed);
        self.time
            .store(start.elapsed().as_millis() as u32, Ordering::Relaxed);

        info!("rendering finished: {:?}", start.elapsed());
    }
}
