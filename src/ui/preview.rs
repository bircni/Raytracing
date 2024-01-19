use std::borrow::Cow;

use egui::{pos2, Align2, Color32, Frame, Key, Rect, Sense, Shape, Ui, Vec2};
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
use log::{debug, warn};
use nalgebra::{Isometry3, OPoint, Perspective3};

use crate::scene::{Scene, Skybox};

const MAX_LIGHTS: usize = 255;
const MAX_OBJECTS: usize = 255;

pub struct Preview {
    active_movement: bool,
    movement_speed: f32,
    look_sensitivity: f32,
    pause_delta: bool,
    pause_count: i32,
}

impl Preview {
    pub fn new() -> Self {
        Self {
            active_movement: false,
            movement_speed: 0.1,
            look_sensitivity: 0.001,
            pause_delta: false,
            pause_count: 0,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn init(render_state: &egui_wgpu::RenderState) {
        let device = &render_state.device;

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("preview vertex shader"),
            source: ShaderSource::Wgsl(Cow::from(include_str!("preview.wgsl"))),
        });

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

    fn change_preview_movement(&mut self, ui: &mut Ui, response: &egui::Response, activate: bool) {
        self.active_movement = activate;
        if activate {
            response.request_focus();
        }

        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::CursorVisible(!activate));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
    }

    pub fn show(&mut self, ui: &mut Ui, scene: &mut Scene) {
        ui.vertical(|ui| {
        let available_size = ui.available_size();
        let aspect_ratio = scene.camera.resolution.0 as f32 / scene.camera.resolution.1 as f32;

        // compute largest rectangle with aspect_ratio that fits in available_size
        let (width, height) = if available_size.x / available_size.y > aspect_ratio {
            (available_size.y * aspect_ratio, available_size.y)
        } else {
            (available_size.x, available_size.x / aspect_ratio)
        };

        Frame::canvas(ui.style())
            .outer_margin(10.0)
            .inner_margin(0.0)
            .fill(match scene.settings.skybox {
                    Skybox::Image { ..} => Color32::GRAY,
                    Skybox::Color(c) => Color32::from_rgb(
                        (c.x * 255.0) as u8,
                        (c.y * 255.0) as u8,
                        (c.z * 255.0) as u8,
                    )
                })
            .show(ui, |ui| {
                let (response, painter) =
                    ui.allocate_painter(Vec2 { x: width -20.0, y: height -20.0 }, Sense::click_and_drag());
                painter.add(Preview::paint(response.rect, scene));
                if response.hover_pos().is_some() && !self.active_movement {
                    egui::show_tooltip(ui.ctx(), egui::Id::new("preview_tooltip"), |ui| {
                        ui.label("Click to change camera position");
                    });
                }

                if response.clicked() {
                    self.change_preview_movement(ui, &response, true);
                }

                if self.active_movement {
                    painter.debug_text(
                        pos2(response.rect.left(), response.rect.top()),
                        Align2::LEFT_TOP,
                        Color32::WHITE,
                        format!("WASD to move camera\nQE to change movement speed {:?}\nYC to change look sensitivity {:?}\nF to reset look_to point facing [0, 0, 0]\nESC to exit movement mode", self.movement_speed, self.look_sensitivity),
                    );
                    ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));

                self.move_camera(ui, &response, scene);
            }
            if !response.has_focus() && self.active_movement {
                // exit movement mode when tabbed out
                self.change_preview_movement(ui, &response, false);
            }
            })});
    }

    fn move_camera(&mut self, ui: &mut Ui, response: &egui::Response, scene: &mut Scene) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.active_movement {
            // exit movement mode using ESC
            self.change_preview_movement(ui, response, false);
        }
        if response.hover_pos().is_none() {
            // move mouse to center
            self.pause_delta = true;
            let center = response.rect.center();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorPosition(center));
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CursorGrab(
                    egui::CursorGrab::Confined,
                ));
        }
        let delta = ui.input(|i| i.pointer.delta()); // rotate look_at point around camera position using mouse
        let direction = (scene.camera.look_at - scene.camera.position).normalize();
        let right = direction.cross(&scene.camera.up).normalize();
        let up = right.cross(&direction).normalize();
        if self.pause_delta {
            self.pause_count += 1;
            if self.pause_count > 5 {
                self.pause_delta = false;
                self.pause_count = 0;
            }
        }

        if !self.pause_delta {
            // move look_at point in a sphere around camera with constant distance 1 using mouse
            let new_point =
                scene.camera.position + direction + right * delta.x * self.look_sensitivity
                    - up * delta.y * self.look_sensitivity;
            scene.camera.look_at =
                scene.camera.position + (new_point - scene.camera.position).normalize();
        }

        scene.camera.fov = (scene.camera.fov - (ui.input(|i| i.scroll_delta.y) * 0.001))
            .clamp(0.0_f32.to_radians(), 180.0_f32.to_radians());

        // movement using keyboard
        ui.input(|i| {
            // reset look_at point facing [0, 0, 0]
            i.key_down(Key::F).then(|| {
                scene.camera.look_at = OPoint::origin();
            });
            // lower sensitivity and clamp so it cant go negative
            i.key_pressed(Key::Y).then(|| {
                self.look_sensitivity = (self.look_sensitivity - 0.0001_f32).max(0.0);
                warn!("Look sensitivity: {}", self.look_sensitivity);
            });
            // higher sensitivity and clamp so it cant go too high
            i.key_pressed(Key::C).then(|| {
                self.look_sensitivity = (self.look_sensitivity + 0.0001_f32).min(0.5);
                warn!("Look sensitivity: {}", self.look_sensitivity);
            });
            // lower movement speed and clamp so it cant go negative
            i.key_down(Key::Q).then(|| {
                self.movement_speed = (self.movement_speed - 0.005_f32).max(0.0);
                warn!("Movement speed: {}", self.movement_speed);
            });
            // higher movement speed and clamp so it cant go too high
            i.key_down(Key::E).then(|| {
                self.movement_speed = (self.movement_speed + 0.005_f32).min(1.0);
                warn!("Movement speed: {}", self.movement_speed);
            });
            // look up
            i.key_down(Key::ArrowUp).then(|| {
                scene.camera.look_at += up * self.look_sensitivity;
            });
            // look down
            // calculate up vector of camera
            i.key_down(Key::ArrowDown).then(|| {
                scene.camera.look_at -= up * self.look_sensitivity;
            });
            // look left
            i.key_down(Key::ArrowLeft).then(|| {
                scene.camera.look_at -= right * self.look_sensitivity;
            });
            // look right
            i.key_down(Key::ArrowRight).then(|| {
                scene.camera.look_at += right * self.look_sensitivity;
            });
            // move camera forward
            i.key_down(Key::W).then(|| {
                scene.camera.position += direction * self.movement_speed;
                scene.camera.look_at += direction * self.movement_speed;
            });
            // move camera backward
            i.key_down(Key::S).then(|| {
                scene.camera.position -= direction * self.movement_speed;
                scene.camera.look_at -= direction * self.movement_speed;
            });
            // move camera left
            i.key_down(Key::A).then(|| {
                scene.camera.position -= right * self.movement_speed;
                scene.camera.look_at -= right * self.movement_speed;
            });
            // move camera right
            i.key_down(Key::D).then(|| {
                scene.camera.position += right * self.movement_speed;
                scene.camera.look_at += right * self.movement_speed;
            });
            // move camera up
            i.key_down(Key::Space).then(|| {
                scene.camera.position += scene.camera.up * self.movement_speed;
                scene.camera.look_at += scene.camera.up * self.movement_speed;
            });
            // move camera down
            i.modifiers.shift.then(|| {
                scene.camera.position -= scene.camera.up * self.movement_speed;
                scene.camera.look_at -= scene.camera.up * self.movement_speed;
            });
        });
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
