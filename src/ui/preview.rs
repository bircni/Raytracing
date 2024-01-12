use std::borrow::Cow;

use egui::{Rect, Shape};
use egui_wgpu::{
    wgpu::{
        self,
        util::{BufferInitDescriptor, DeviceExt},
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferDescriptor,
        BufferUsages, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState,
        DepthStencilState, FragmentState, FrontFace, MultisampleState, PipelineLayoutDescriptor,
        PolygonMode, PrimitiveState, PrimitiveTopology, RenderPipeline, RenderPipelineDescriptor,
        ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilState, TextureFormat,
        VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
    },
    Callback, CallbackTrait,
};
use log::debug;
use nalgebra::{Isometry3, Perspective3};

use crate::scene::Scene;

const MAX_LIGHTS: usize = 255;
const MAX_OBJECTS: usize = 255;

pub struct Preview {}

// implement preview, define methods and variables
impl Preview {
    #![allow(clippy::too_many_lines)]
    pub fn init(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview vertex shader"),
            source: ShaderSource::Wgsl(Cow::from(include_str!("preview.wgsl"))),
        });

        // use wgpu to create a bind group layout for the preview render pipeline
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("preview bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("preview pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // define the render pipeline for the preview
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("preview pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[VertexBufferLayout {
                    // 3x f32 for position, 3x f32 for normal, 3x f32 for color, 1x u32 for transform index
                    array_stride: std::mem::size_of::<f32>() as u64 * (3 + 3 + 3 + 1),
                    step_mode: VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 0,
                        },
                        // normal
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: std::mem::size_of::<f32>() as u64 * 3,
                            shader_location: 1,
                        },
                        // color
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: std::mem::size_of::<f32>() as u64 * 6,
                            shader_location: 2,
                        },
                        // transform index
                        VertexAttribute {
                            format: VertexFormat::Uint32,
                            offset: std::mem::size_of::<f32>() as u64 * 9,
                            shader_location: 3,
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
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });

        // create various buffers for the preview
        let uniform_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("preview uniform buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            size: std::mem::size_of::<ShaderUniforms>() as u64,
            mapped_at_creation: false,
        });

        let lights_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("preview lights buffer"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            size: std::mem::size_of::<ShaderLight>() as u64 * MAX_LIGHTS as u64,
            mapped_at_creation: false,
        });

        let transforms_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("preview transforms buffer"),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            size: std::mem::size_of::<[[f32; 4]; 4]>() as u64 * MAX_OBJECTS as u64,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("preview bind group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: lights_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: transforms_buffer.as_entire_binding(),
                },
            ],
        });

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("preview vertex buffer"),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            size: 0,
            mapped_at_creation: false,
        });

        let resources = PreviewResources {
            bind_group,
            pipeline,
            vertex_buffer,
            uniform_buffer,
            lights_buffer,
            transforms_buffer,
        };

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(resources);

        render_state
            .renderer
            .write()
            .callback_resources
            .insert(VertexCount(0));
    }

    pub fn paint(rect: Rect, scene: &Scene) -> egui::Shape {
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
    lights_buffer: Buffer,
    transforms_buffer: Buffer,
}

struct PreviewRenderer {
    scene: Scene,
    aspect_ratio: f32,
}

struct VertexCount(usize);

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderUniforms {
    view: [[f32; 4]; 4],
    lights_count: u32,
    _pad: [u32; 3],
    ambient_color: [f32; 3],
    ambient_intensity: f32,
}

#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct ShaderLight {
    position: [f32; 3],
    _pad: [f32; 1],
    color: [f32; 3],
    intensity: f32,
}

// implement callback trait for preview renderer to prepare and paint the preview in close to real time
#[allow(clippy::expect_used)]
impl CallbackTrait for PreviewRenderer {
    #[allow(clippy::too_many_lines)]
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        debug!("Preparing preview renderer");

        let vertex_count = callback_resources
            .get::<VertexCount>()
            .expect("Failed to get vertex count");

        let vertices = self
            .scene
            .objects
            .iter()
            .map(|o| o.triangles.len())
            .sum::<usize>()
            * 3;

        if vertex_count.0 != vertices {
            debug!("New vertex buffer from {} to {}", vertex_count.0, vertices);

            let resources = callback_resources
                .get_mut::<PreviewResources>()
                .expect("Failed to get preview resources");

            resources.vertex_buffer.destroy();

            resources.vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("preview vertex buffer"),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                contents: self
                    .scene
                    .objects
                    .iter()
                    .enumerate()
                    .flat_map(|(i, o)| o.triangles.iter().map(move |t| (i, o, t)))
                    .map(|(i, o, t)| (i, t.material_index.and_then(|i| o.materials.get(i)), t))
                    .flat_map(|(i, m, t)| {
                        let color = m
                            .as_ref()
                            .and_then(|m| m.diffuse_color)
                            .map_or([0.9; 3], std::convert::Into::into);
                        [
                            bytemuck::bytes_of(&[t.a.into(), t.a_normal.into(), color]),
                            bytemuck::bytes_of(&(i as u32)),
                            bytemuck::bytes_of(&[t.b.into(), t.b_normal.into(), color]),
                            bytemuck::bytes_of(&(i as u32)),
                            bytemuck::bytes_of(&[t.c.into(), t.c_normal.into(), color]),
                            bytemuck::bytes_of(&(i as u32)),
                        ]
                        .into_iter()
                        .flatten()
                        .copied()
                        .collect::<Vec<u8>>()
                    })
                    .collect::<Vec<u8>>()
                    .as_slice(),
            });

            callback_resources.insert(VertexCount(vertices));
        }

        let resources = callback_resources
            .get::<PreviewResources>()
            .expect("Failed to get preview resources");

        queue.write_buffer(
            &resources.uniform_buffer,
            0,
            bytemuck::cast_slice(&[ShaderUniforms {
                view: (Perspective3::new(self.aspect_ratio, self.scene.camera.fov, 0.1, 1000.0)
                    .to_homogeneous()
                    * Isometry3::look_at_rh(
                        &self.scene.camera.position,
                        &self.scene.camera.look_at,
                        &self.scene.camera.up,
                    )
                    .to_homogeneous())
                .into(),
                lights_count: self.scene.lights.len() as u32,
                ambient_color: self.scene.settings.ambient_color.into(),
                ambient_intensity: self.scene.settings.ambient_intensity,
                ..Default::default()
            }]),
        );

        queue.write_buffer(
            &resources.lights_buffer,
            0,
            self.scene
                .lights
                .iter()
                .map(|l| ShaderLight {
                    position: l.position.into(),
                    color: l.color.into(),
                    intensity: l.intensity,
                    ..Default::default()
                })
                .chain(std::iter::repeat(ShaderLight::default()))
                .take(MAX_LIGHTS)
                .flat_map(|x| bytemuck::bytes_of(&x).to_vec())
                .collect::<Vec<u8>>()
                .as_slice(),
        );

        queue.write_buffer(
            &resources.transforms_buffer,
            0,
            self.scene
                .objects
                .iter()
                .map(|o| o.transform().to_homogeneous())
                .chain(std::iter::repeat(Isometry3::identity().to_homogeneous()))
                .take(MAX_OBJECTS)
                .flat_map(|m| bytemuck::cast_slice(m.as_slice()).to_vec())
                .collect::<Vec<u8>>()
                .as_slice(),
        );

        vec![]
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
