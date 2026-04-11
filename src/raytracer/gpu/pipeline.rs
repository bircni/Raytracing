//! Compute pipeline and per-block render for the GPU raytracer.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, AtomicU32, Ordering},
    },
    thread,
    time::Instant,
};

use anyhow::Context;
use bytemuck::cast_slice;
use egui::{Color32, ColorImage, ImageData, TextureHandle, TextureOptions, mutex::Mutex};
use image::RgbImage;
use log::{info, warn};
use nalgebra::Rotation3;
use wgpu::util::DeviceExt;

use crate::scene::{Scene, Skybox};

use super::scene::{GpuSceneData, build_gpu_scene};
use super::types::{GpuGlobals, GpuLight};

const MAX_LIGHTS: usize = 64;

/// Caps samples/pixel on the GPU path.
///
/// The render thread shares the same wgpu queue as egui. Very high sample counts keep the GPU busy
/// and can exhaust queue staging memory — egui then fails `write_buffer_with` (panic in
/// `egui_wgpu::Renderer::update`). Large resolutions also hit TDR / VRAM limits. Use the CPU path
/// for high sample counts or full-quality AA.
fn gpu_max_samples_for_resolution(width: u32, height: u32) -> u32 {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels <= 2_073_600 {
        // up to ~1080p — keep conservative; 128×AA here starves egui staging
        32
    } else if pixels <= 8_294_400 {
        // up to ~4K
        16
    } else if pixels <= 16_588_800 {
        12
    } else {
        8
    }
}

pub struct GpuRaytrace {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuRaytrace {
    #[expect(
        clippy::too_many_lines,
        reason = "wgpu bind group layout setup is inherently long"
    )]
    pub fn new(render_state: &egui_wgpu::RenderState) -> anyhow::Result<Self> {
        let device = render_state.device.clone();
        let queue = render_state.queue.clone();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raytrace"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "raytrace.wgsl"
            ))),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raytrace bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<GpuGlobals>() as u64)
                                .context("GpuGlobals size")?,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("raytrace pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raytrace compute"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
        })
    }

    /// Renders the scene to the given texture and CPU image buffer (same contract as CPU path).
    #[expect(
        clippy::too_many_arguments,
        reason = "matches render thread parameters"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "block loop matches CPU render thread structure"
    )]
    pub fn render(
        &self,
        scene: &Scene,
        mut texture: TextureHandle,
        image: &Arc<Mutex<RgbImage>>,
        ctx: &egui::Context,
        cancel: &Arc<AtomicBool>,
        progress: &Arc<AtomicU16>,
        time: &Arc<AtomicU32>,
    ) -> anyhow::Result<()> {
        let start = Instant::now();
        let Some(data) = build_gpu_scene(scene) else {
            anyhow::bail!("Scene has no geometry for GPU render");
        };

        let GpuSceneData {
            triangles,
            nodes,
            materials,
            lights,
            atlas,
            sky,
            sky_color,
        } = data;

        let tri_count = triangles.len() as u32;
        let node_count = nodes.len() as u32;
        let material_count = materials.len() as u32;

        let (width, height) = scene.camera.resolution;
        let block_size = [width / 20, height / 20];
        let block_w = block_size[0].max(1);
        let block_h = block_size[1].max(1);

        let tri_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gpu triangles"),
                contents: cast_slice(&triangles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let node_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gpu bvh"),
                contents: cast_slice(&nodes),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let mat_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gpu materials"),
                contents: cast_slice(&materials),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let mut light_data: Vec<GpuLight> = lights.into_iter().take(MAX_LIGHTS).collect();
        light_data.resize(
            MAX_LIGHTS,
            GpuLight {
                position: [0.0; 3],
                _pad: 0.0,
                color: [0.0; 3],
                intensity: 0.0,
            },
        );
        let light_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("gpu lights"),
                contents: cast_slice(&light_data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let atlas_rgba = rgb_to_rgba(&atlas);
        let atlas_tex = self.texture_rgba(
            atlas.width(),
            atlas.height(),
            &atlas_rgba,
            Some("diffuse atlas"),
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let (_sky_tex, sky_view) = sky.as_ref().map_or_else(
            || {
                let t = self.texture_rgba(1, 1, &[255, 255, 255, 255], Some("sky fallback"));
                let v = t.create_view(&wgpu::TextureViewDescriptor::default());
                (t, v)
            },
            |sky_img| {
                let rgba = rgb_to_rgba(sky_img);
                let t = self.texture_rgba(sky_img.width(), sky_img.height(), &rgba, Some("sky"));
                let v = t.create_view(&wgpu::TextureViewDescriptor::default());
                (t, v)
            },
        );

        let block_elements = (block_w * block_h) as usize;
        let block_bytes = (block_elements * 16) as u64;
        let block_out = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("block out"),
            size: block_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging readback"),
            size: block_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu globals"),
            size: std::mem::size_of::<GpuGlobals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raytrace bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tri_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: node_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: mat_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: light_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&sky_view),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: block_out.as_entire_binding(),
                },
            ],
        });

        let blocks_x = width.div_ceil(block_w);
        let blocks_y = height.div_ceil(block_h);
        let total_blocks = (blocks_x * blocks_y) as f32;

        let rot = Rotation3::look_at_rh(
            &(scene.camera.look_at - scene.camera.position),
            &scene.camera.up,
        );
        let inv = rot.inverse();
        let m = inv.matrix();
        let c0 = m.column(0);
        let c1 = m.column(1);
        let c2 = m.column(2);
        let inv_rot_c0 = [c0.x, c0.y, c0.z, 0.0];
        let inv_rot_c1 = [c1.x, c1.y, c1.z, 0.0];
        let inv_rot_c2 = [c2.x, c2.y, c2.z, 0.0];

        let fov_tan_half = (scene.camera.fov * 0.5).tan();
        let aspect = width as f32 / height as f32;

        let aa = scene.settings.anti_aliasing;
        let requested = scene.settings.samples.max(1);
        let heuristic_cap = gpu_max_samples_for_resolution(width, height);
        let cap = scene
            .settings
            .gpu_max_samples
            .unwrap_or(heuristic_cap)
            .max(1);
        let samples = requested.min(cap);
        if requested > cap {
            warn!(
                "GPU raytrace: anti-alias samples {requested} exceed cap {cap} (heuristic default {heuristic_cap} unless `gpu_max_samples` is set in scene settings); using {samples}. Use CPU render for more."
            );
        }
        let sqrt_samples = (samples as f32).sqrt().floor() as u32;
        let sqrt_samples = sqrt_samples.max(1);

        let mut block_index = 0u32;
        for y_block in 0..blocks_y {
            for x_block in 0..blocks_x {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(());
                }

                let ox = x_block * block_w;
                let oy = y_block * block_h;
                let pix_width = block_w.min(width.saturating_sub(ox)).max(1);
                let pix_height = block_h.min(height.saturating_sub(oy)).max(1);

                let globals = GpuGlobals {
                    cam_pos: [
                        scene.camera.position.x,
                        scene.camera.position.y,
                        scene.camera.position.z,
                        0.0,
                    ],
                    inv_rot_c0,
                    inv_rot_c1,
                    inv_rot_c2,
                    fov_tan_half,
                    width: pix_width as f32,
                    height: pix_height as f32,
                    aspect,
                    block_origin_x: ox,
                    block_origin_y: oy,
                    block_size_x: pix_width,
                    block_size_y: pix_height,
                    full_width: width,
                    full_height: height,
                    tri_count,
                    node_count,
                    light_count: scene.lights.len().min(MAX_LIGHTS) as u32,
                    samples,
                    sqrt_samples,
                    antialias: u32::from(aa),
                    max_depth: scene.settings.max_bounces,
                    _pad0: 0,
                    _pad_pre_ambient: [0, 0],
                    ambient_color: [
                        scene.settings.ambient_color.x,
                        scene.settings.ambient_color.y,
                        scene.settings.ambient_color.z,
                    ],
                    ambient_intensity: scene.settings.ambient_intensity,
                    sky_color: [sky_color.x, sky_color.y, sky_color.z],
                    sky_has_tex: u32::from(matches!(scene.settings.skybox, Skybox::Image { .. })),
                    sky_width: sky.as_ref().map_or(1, RgbImage::width),
                    sky_height: sky.as_ref().map_or(1, RgbImage::height),
                    atlas_width: atlas.width(),
                    atlas_height: atlas.height(),
                    material_count,
                    delta: 1e-5,
                    _pad_after_delta: [0; 2],
                    _pad_tail: [0.0; 3],
                    _pad_before_uniform_end: 0,
                    _pad_uniform_end: [0; 4],
                    _pad_uniform_end_b: [0; 4],
                    _pad_uniform_end_c: [0; 4],
                };

                self.queue
                    .write_buffer(&uniform_buf, 0, cast_slice(&[globals]));

                let wg_x = pix_width.div_ceil(16);
                let wg_y = pix_height.div_ceil(16);

                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("raytrace encode"),
                        });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("raytrace pass"),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.dispatch_workgroups(wg_x, wg_y, 1);
                }
                let this_pixels = (pix_width * pix_height) as usize;
                let this_copy_bytes = (this_pixels * 16) as u64;
                encoder.copy_buffer_to_buffer(&block_out, 0, &staging, 0, this_copy_bytes);
                self.queue.submit(std::iter::once(encoder.finish()));
                let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

                staging
                    .slice(0..this_copy_bytes)
                    .map_async(wgpu::MapMode::Read, |_| {});
                let _ = self.device.poll(wgpu::PollType::wait_indefinitely());

                let view = staging.slice(0..this_copy_bytes).get_mapped_range();
                let mapped: &[u8] = &view;
                let floats: &[[f32; 4]] = bytemuck::cast_slice(mapped);
                let mut pixels = Vec::with_capacity(this_pixels);
                for p in floats.iter().take(this_pixels) {
                    let r = (p[0].clamp(0.0, 1.0) * 255.0) as u8;
                    let g = (p[1].clamp(0.0, 1.0) * 255.0) as u8;
                    let b = (p[2].clamp(0.0, 1.0) * 255.0) as u8;
                    pixels.push(Color32::from_rgb(r, g, b));
                }
                let mut rgb_rows = Vec::with_capacity(this_pixels);
                for p in floats.iter().take(this_pixels) {
                    rgb_rows.push([
                        (p[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (p[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (p[2].clamp(0.0, 1.0) * 255.0) as u8,
                    ]);
                }
                drop(view);
                staging.unmap();

                // Let the UI thread run a frame before we queue more GPU work + egui uploads.
                thread::yield_now();

                texture.set_partial(
                    [ox as usize, oy as usize],
                    ImageData::Color(Arc::new(ColorImage::new(
                        [pix_width as usize, pix_height as usize],
                        pixels,
                    ))),
                    TextureOptions::default(),
                );

                {
                    let mut img = image.lock();
                    for x in 0..pix_width {
                        for y in 0..pix_height {
                            let pi = (x + y * pix_width) as usize;
                            if let Some(px) = rgb_rows.get(pi) {
                                img.put_pixel(ox + x, oy + y, image::Rgb(*px));
                            }
                        }
                    }
                }

                block_index += 1;
                progress.store(
                    ((block_index as f32 / total_blocks) * f32::from(u16::MAX)).round() as u16,
                    Ordering::Relaxed,
                );
                ctx.request_repaint();
            }
        }

        progress.store(u16::MAX, Ordering::Relaxed);
        time.store(start.elapsed().as_millis() as u32, Ordering::Relaxed);
        info!("GPU rendering finished: {:?}", start.elapsed());
        Ok(())
    }

    fn texture_rgba(
        &self,
        width: u32,
        height: u32,
        rgba: &[u8],
        label: Option<&'static str>,
    ) -> wgpu::Texture {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        tex
    }
}

fn rgb_to_rgba(img: &RgbImage) -> Vec<u8> {
    let mut v = Vec::with_capacity(img.width() as usize * img.height() as usize * 4);
    for p in img.pixels() {
        v.push(p[0]);
        v.push(p[1]);
        v.push(p[2]);
        v.push(255);
    }
    v
}
