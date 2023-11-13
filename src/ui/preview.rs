use std::sync::Arc;

use anyhow::Context;
use egui::{mutex::Mutex, PaintCallback, Painter, Rect, Shape};
use egui_glow::{glow::HasContext, CallbackFn};
use log::debug;
use nalgebra::{Isometry3, Matrix4, Perspective3};

use crate::scene::{light::Light, Scene};
use eframe::glow;

pub struct Preview {
    callback: Arc<CallbackFn>,
    view_matrix: Arc<Mutex<Matrix4<f32>>>,
    lights: Arc<Mutex<Vec<Light>>>,
}

macro_rules! gl_result {
    ($expr: expr, $err: expr) => {
        unsafe { $expr }
            .map_err(|e| anyhow::anyhow!(e))
            .context($err)
    };
}

macro_rules! gl {
    ($expr: expr) => {
        unsafe { $expr }
    };
}

impl Preview {
    pub fn from_scene(gl: Arc<glow::Context>, scene: &Scene) -> anyhow::Result<Self> {
        let program = compile_program(
            gl.clone(),
            include_str!("preview.vertex.glsl"),
            include_str!("preview.fragment.glsl"),
        )?;

        let view_matrix_uniform = gl!(gl.get_uniform_location(program, "view"))
            .context("Failed to get uniform location for view_matrix")?;

        let vertex_array = gl_result!(gl.create_vertex_array(), "Failed to create vertex array")?;
        gl!(gl.bind_vertex_array(Some(vertex_array)));

        // load vertex data
        let vertices: Vec<f32> = scene
            .objects
            .iter()
            .flat_map(|o| o.triangles.iter().map(|t| (t, o.transform)))
            .flat_map(|(t, m)| [t.a, t.b, t.c].map(move |p| m.transform_point(&p)))
            .map(|p| [p.x, p.y, p.z])
            .flatten()
            .collect();
        debug!("Loaded {} vertices", vertices.len());
        let vertex_buffer = gl_result!(gl.create_buffer(), "Failed to create vertex buffer")?;
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer)));
        gl!(gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(vertices.as_slice()),
            glow::STATIC_DRAW
        ));

        // position attribute
        gl!(gl.vertex_attrib_pointer_f32(
            0,
            3,
            glow::FLOAT,
            false,
            3 * std::mem::size_of::<f32>() as i32,
            0
        ));
        gl!(gl.enable_vertex_attrib_array(0));

        {
            // load normal data
            let normals: Vec<f32> = scene
                .objects
                .iter()
                .flat_map(|o| o.triangles.iter().map(|t| (t, o.transform)))
                .flat_map(|(t, m)| {
                    [t.a_normal, t.b_normal, t.c_normal].map(move |n| m.transform_vector(&n))
                })
                .map(|n| [n.x, n.y, n.z])
                .flatten()
                .collect();
            debug!("Loaded {} normals", normals.len());
            let normal_buffer = gl_result!(gl.create_buffer(), "Failed to create normal buffer")?;
            gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(normal_buffer)));
            gl!(gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(normals.as_slice()),
                glow::STATIC_DRAW
            ));

            // normal attribute
            gl!(gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                3 * std::mem::size_of::<f32>() as i32,
                0
            ));
            gl!(gl.enable_vertex_attrib_array(1));
        }

        {
            // load color data
            let colors: Vec<f32> = scene
                .objects
                .iter()
                .flat_map(|o| {
                    o.triangles
                        .iter()
                        .map(|t| t.material_index.map(|i| &o.materials[i]))
                        .flat_map(|m| [m, m, m])
                })
                .flat_map(|m| m.and_then(|m| m.kd).unwrap_or([1.0, 1.0, 1.0]))
                .collect();
            debug!("Loaded {} colors", colors.len());
            let color_buffer = gl_result!(gl.create_buffer(), "Failed to create color buffer")?;
            gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(color_buffer)));
            gl!(gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(colors.as_slice()),
                glow::STATIC_DRAW
            ));

            // color attribute
            gl!(gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                3 * std::mem::size_of::<f32>() as i32,
                0
            ));
            gl!(gl.enable_vertex_attrib_array(2));
        }

        // lights SSBO
        let lights_ssbo = gl_result!(gl.create_buffer(), "Failed to create lights SSBO")?;
        gl!(gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(lights_ssbo)));
        gl!(gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            scene
                .lights
                .iter()
                .flat_map(bytemuck::bytes_of)
                .copied()
                .collect::<Vec<u8>>()
                .as_slice(),
            glow::STATIC_DRAW
        ));
        gl!(gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(lights_ssbo)));

        // unbind
        gl!(gl.bind_vertex_array(None));
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, None));
        gl!(gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None));
        gl!(gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None));

        let view_matrix = Arc::new(Mutex::new(Matrix4::default()));

        let view_matrix_callback = view_matrix.clone();
        let lights = Arc::new(Mutex::new(scene.lights.clone()));
        Ok(Self {
            view_matrix,
            lights: lights.clone(),
            callback: Arc::new(CallbackFn::new(move |_, painter| unsafe {
                let gl = painter.gl().as_ref();

                gl.clear_color(0.0, 0.0, 0.0, 1.0);
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                gl.enable(glow::DEPTH_TEST);
                gl.disable(glow::CULL_FACE);

                // draw
                gl.use_program(Some(program));
                gl.bind_vertex_array(Some(vertex_array));
                gl.uniform_matrix_4_f32_slice(
                    Some(&view_matrix_uniform),
                    false,
                    view_matrix_callback.lock().as_slice(),
                );

                // bind lights ssbo
                gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(lights_ssbo));
                gl.buffer_data_u8_slice(
                    glow::SHADER_STORAGE_BUFFER,
                    lights
                        .lock()
                        .iter()
                        .flat_map(bytemuck::bytes_of)
                        .copied()
                        .collect::<Vec<u8>>()
                        .as_slice(),
                    glow::STATIC_DRAW,
                );
                gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(lights_ssbo));

                gl.draw_arrays(glow::TRIANGLES, 0, vertices.len() as i32 / 3);

                // reset
                gl.bind_vertex_array(None);
                gl.use_program(None);
                gl.bind_buffer(glow::ARRAY_BUFFER, None);
                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
                gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, None);

                // flush
                gl.flush();
            })),
        })
    }

    pub fn paint(&self, rect: Rect, painter: &Painter, scene: &Scene) {
        *self.view_matrix.lock() =
            Perspective3::new(rect.aspect_ratio(), scene.camera.fov, 0.1, 1000.0).to_homogeneous()
                * Isometry3::look_at_rh(
                    &scene.camera.position,
                    &scene.camera.look_at,
                    &scene.camera.up,
                )
                .to_homogeneous();

        *self.lights.lock() = scene.lights.clone();

        painter.add(Shape::Callback(PaintCallback {
            rect,
            callback: self.callback.clone(),
        }));
    }
}

fn compile_program(
    gl: Arc<glow::Context>,
    vertex_shader_src: &str,
    fragment_shader_src: &str,
) -> anyhow::Result<glow::Program> {
    // create shader
    let vertex_shader = gl_result!(
        gl.create_shader(glow::VERTEX_SHADER),
        "Failed to create vertex shader"
    )?;
    gl!(gl.shader_source(vertex_shader, vertex_shader_src));
    gl!(gl.compile_shader(vertex_shader));
    if !gl!(gl.get_shader_compile_status(vertex_shader)) {
        let log = gl!(gl.get_shader_info_log(vertex_shader));
        return Err(anyhow::anyhow!("Failed to compile vertex shader: {}", log));
    }

    let fragment_shader = gl_result!(
        gl.create_shader(glow::FRAGMENT_SHADER),
        "Failed to create fragment shader"
    )?;
    gl!(gl.shader_source(fragment_shader, fragment_shader_src));
    gl!(gl.compile_shader(fragment_shader));
    if !gl!(gl.get_shader_compile_status(fragment_shader)) {
        let log = gl!(gl.get_shader_info_log(fragment_shader));
        return Err(anyhow::anyhow!(
            "Failed to compile fragment shader: {}",
            log
        ));
    }

    // create program
    let program = gl_result!(gl.create_program(), "Failed to create program")?;
    gl!(gl.attach_shader(program, vertex_shader));
    gl!(gl.attach_shader(program, fragment_shader));
    gl!(gl.link_program(program));
    if !gl!(gl.get_program_link_status(program)) {
        let log = gl!(gl.get_program_info_log(program));
        return Err(anyhow::anyhow!("Failed to link program: {}", log));
    }

    // delete shaders
    gl!(gl.delete_shader(vertex_shader));
    gl!(gl.delete_shader(fragment_shader));

    Ok(program)
}
