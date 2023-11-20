#![allow(unsafe_code)]

use std::{borrow::Cow, sync::Arc};

use egui::{mutex::Mutex, Rect, Shape};
use egui_wgpu::{
    wgpu::{
        self,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages,
        ColorTargetState, ColorWrites, FragmentState, MultisampleState, PipelineLayoutDescriptor,
        PrimitiveState, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
        ShaderSource, ShaderStages, VertexAttribute, VertexBufferLayout, VertexFormat, VertexState,
        VertexStepMode,
    },
    Callback, CallbackTrait,
};
use log::debug;
use nalgebra::{Isometry3, Perspective3};

use crate::scene::Scene;

pub struct Preview {}

impl Preview {
    pub fn new(render_state: &egui_wgpu::RenderState, scene: &Scene) -> anyhow::Result<Self> {
        let device = &render_state.device;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview vertex shader"),
            source: ShaderSource::Wgsl(Cow::from(include_str!("preview.wgsl"))),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("preview bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    // 3x f32 for position, 3x f32 for color
                    array_stride: std::mem::size_of::<f32>() as u64 * (3 + 3),
                    step_mode: VertexStepMode::Vertex,

                    attributes: &[
                        // position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // color
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: std::mem::size_of::<f32>() as u64 * 3,
                            shader_location: 1,
                        },
                    ],
                }],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: render_state.target_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("preview uniform buffer"),
            contents: bytemuck::cast_slice(
                (Perspective3::new(16.0 / 9.0, scene.camera.fov, 0.1, 1000.0).to_homogeneous()
                    * Isometry3::look_at_rh(
                        &scene.camera.position,
                        &scene.camera.look_at,
                        &scene.camera.up,
                    )
                    .to_homogeneous())
                .as_slice(),
            ),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("preview bind group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("preview vertex buffer"),
            contents: bytemuck::cast_slice(
                scene
                    .objects
                    .iter()
                    .flat_map(|o| o.triangles.iter().map(move |t| (o, t)))
                    .map(|(o, t)| (t.material_index.and_then(|i| o.materials.get(i)), t))
                    .flat_map(|(m, t)| -> [[f32; 3]; 6] {
                        let color = m.as_ref().and_then(|m| m.kd).unwrap_or([0.0; 3]);
                        [t.a.into(), color, t.b.into(), color, t.c.into(), color]
                    })
                    .flatten()
                    .collect::<Vec<f32>>()
                    .as_slice(),
            ),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        let resources = PreviewResources {
            bind_group,
            pipeline,
            vertex_buffer,
            uniform_buffer,
        };

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        Ok(Self {})
    }

    pub fn paint(&self, rect: Rect, scene: &Scene) -> egui::Shape {
        Shape::Callback(Callback::new_paint_callback(
            rect,
            PreviewRenderer {
                aspect_ratio: rect.aspect_ratio(),
                scene: scene.clone(),
            },
        ))
    }
}

struct PreviewResources {
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
}

struct PreviewRenderer {
    scene: Scene,
    aspect_ratio: f32,
}

struct VertexCount(usize);

impl CallbackTrait for PreviewRenderer {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        debug!("Preparing preview renderer");

        let resources = callback_resources
            .get::<PreviewResources>()
            .expect("Failed to get preview resources");

        queue.write_buffer(
            &resources.uniform_buffer,
            0,
            bytemuck::cast_slice(
                (Perspective3::new(self.aspect_ratio, self.scene.camera.fov, 0.1, 1000.0)
                    .to_homogeneous()
                    * Isometry3::look_at_rh(
                        &self.scene.camera.position,
                        &self.scene.camera.look_at,
                        &self.scene.camera.up,
                    )
                    .to_homogeneous())
                .as_slice(),
            ),
        );

        callback_resources.insert(VertexCount(
            self.scene
                .objects
                .iter()
                .map(|o| o.triangles.len())
                .sum::<usize>()
                * 3,
        ));

        Vec::new()
    }

    fn paint<'a>(
        &'a self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'a>,
        callback_resources: &'a egui_wgpu::CallbackResources,
    ) {
        debug!("Painting preview renderer");

        let resources = callback_resources
            .get::<PreviewResources>()
            .expect("Failed to get preview resources");

        let vertex_count = callback_resources
            .get::<VertexCount>()
            .expect("Failed to get vertex count")
            .0;

        render_pass.set_pipeline(&resources.pipeline);
        render_pass.set_bind_group(0, &resources.bind_group, &[]);
        render_pass.set_vertex_buffer(0, resources.vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count as u32, 0..1);
    }
}

impl Drop for Preview {
    fn drop(&mut self) {}
}
