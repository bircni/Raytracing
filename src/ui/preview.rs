#![allow(unsafe_code)]

use std::sync::Arc;

use anyhow::Context;
use egui::{PaintCallback, PaintCallbackInfo, Painter, Rect, Shape};
use egui_glow::{glow::HasContext, CallbackFn};
use log::debug;
use nalgebra::{Isometry3, Perspective3};

use crate::scene::Scene;
use eframe::glow;

#[derive(Debug)]
pub struct Preview {
    gl: Arc<glow::Context>,
    program: glow::NativeProgram,
    vertex_array: glow::NativeVertexArray,
    position_buffer: glow::NativeBuffer,
    normal_buffer: glow::NativeBuffer,
    color_buffer: glow::NativeBuffer,
    transform_index_buffer: glow::NativeBuffer,
    lights_ssbo: glow::NativeBuffer,
    transform_ssbo: glow::NativeBuffer,
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
    pub fn new(gl: Arc<glow::Context>) -> anyhow::Result<Self> {
        let program = compile_program(
            &gl,
            include_str!("preview.vertex.glsl"),
            include_str!("preview.fragment.glsl"),
        )?;

        let vertex_array = gl_result!(gl.create_vertex_array(), "Failed to create vertex array")?;
        gl!(gl.bind_vertex_array(Some(vertex_array)));

        let position_buffer = gl_result!(gl.create_buffer(), "Failed to create vertex buffer")?;
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(position_buffer)));
        gl!(gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 0, 0));
        gl!(gl.enable_vertex_attrib_array(0));

        let normal_buffer = gl_result!(gl.create_buffer(), "Failed to create normal buffer")?;
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(normal_buffer)));
        gl!(gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 0, 0));
        gl!(gl.enable_vertex_attrib_array(1));

        let color_buffer = gl_result!(gl.create_buffer(), "Failed to create color buffer")?;
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(color_buffer)));
        gl!(gl.vertex_attrib_pointer_f32(2, 3, glow::FLOAT, false, 0, 0));
        gl!(gl.enable_vertex_attrib_array(2));

        let transform_index_buffer = gl_result!(
            gl.create_buffer(),
            "Failed to create transform index buffer"
        )?;
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, Some(transform_index_buffer)));
        gl!(gl.vertex_attrib_pointer_i32(3, 1, glow::UNSIGNED_INT, 0, 0));

        let lights_ssbo = gl_result!(gl.create_buffer(), "Failed to create lights SSBO")?;

        let transform_ssbo = gl_result!(gl.create_buffer(), "Failed to create transform SSBO")?;

        gl!(gl.bind_vertex_array(None));
        gl!(gl.bind_buffer(glow::ARRAY_BUFFER, None));

        gl!(gl.enable(glow::DEBUG_OUTPUT));
        gl!(gl.debug_message_callback(gl_log));

        if gl!(gl.get_error()) != glow::NO_ERROR {
            return Err(anyhow::anyhow!("OpenGL error"));
        }

        Ok(Self {
            gl,
            program,
            vertex_array,
            position_buffer,
            normal_buffer,
            color_buffer,
            transform_index_buffer,
            lights_ssbo,
            transform_ssbo,
        })
    }

    fn callback(
        &self,
        triangle_count: usize,
    ) -> impl Fn(PaintCallbackInfo, &egui_glow::Painter) + Sync + Send + 'static {
        let program = self.program;
        let vertex_array = self.vertex_array;
        let lights_ssbo = self.lights_ssbo;
        let transform_ssbo = self.transform_ssbo;

        move |_info, painter| unsafe {
            let gl = painter.gl().as_ref();

            // setup
            gl.use_program(Some(program));
            gl.enable(glow::DEPTH_TEST);
            gl.disable(glow::CULL_FACE);

            // clear
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            // bind lights ssbo
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(lights_ssbo));
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, Some(lights_ssbo));

            // bind transform ssbo
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(transform_ssbo));
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, Some(transform_ssbo));

            // draw
            gl.bind_vertex_array(Some(vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, triangle_count as i32 * 3);

            // unbind
            gl.use_program(None);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
            gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None);
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 0, None);
            gl.bind_buffer_base(glow::SHADER_STORAGE_BUFFER, 1, None);

            // flush
            gl.flush();
        }
    }

    fn upload_scene(&self, rect: Rect, scene: &Scene) -> anyhow::Result<usize> {
        gl!(self.gl.use_program(Some(self.program)));

        let view_matrix_uniform = gl!(self.gl.get_uniform_location(self.program, "view"))
            .context("Failed to get uniform location for view_matrix")?;

        // set view matrix
        gl!(self.gl.uniform_matrix_4_f32_slice(
            Some(&view_matrix_uniform),
            false,
            (Perspective3::new(rect.aspect_ratio(), scene.camera.fov, 0.1, 1000.0)
                .to_homogeneous()
                * Isometry3::look_at_rh(
                    &scene.camera.position,
                    &scene.camera.look_at,
                    &scene.camera.up,
                )
                .to_homogeneous())
            .as_slice()
        ));

        // upload positions
        let positions = scene
            .objects
            .iter()
            .flat_map(|o| o.triangles.iter())
            .flat_map(|t| [t.a, t.b, t.c])
            .flat_map(|v| [v.x, v.y, v.z])
            .collect::<Vec<f32>>();
        gl!(self
            .gl
            .bind_buffer(glow::ARRAY_BUFFER, Some(self.position_buffer)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(positions.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} positions", positions.len() / 3);

        // upload normals
        let normals = scene
            .objects
            .iter()
            .flat_map(|o| o.triangles.iter())
            .flat_map(|t| [t.a_normal, t.b_normal, t.c_normal])
            .flat_map(|v| [v.x, v.y, v.z])
            .collect::<Vec<f32>>();
        gl!(self
            .gl
            .bind_buffer(glow::ARRAY_BUFFER, Some(self.normal_buffer)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(normals.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} normals", normals.len() / 3);

        // upload colors
        let colors = scene
            .objects
            .iter()
            .flat_map(|o| o.triangles.iter().map(move |t| (o, t)))
            .map(|(o, t)| {
                t.material_index
                    .and_then(|i| o.materials.get(i))
                    .and_then(|m| m.kd)
                    .unwrap_or([1.0, 1.0, 1.0])
            })
            .flat_map(|c| [c, c, c])
            .flatten()
            .collect::<Vec<f32>>();
        gl!(self
            .gl
            .bind_buffer(glow::ARRAY_BUFFER, Some(self.color_buffer)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(colors.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} colors", colors.len() / 3);

        // upload transform indices
        let transform_indices = scene
            .objects
            .iter()
            .enumerate()
            .flat_map(|(i, o)| std::iter::repeat(i as u32).take(o.triangles.len() * 3))
            .collect::<Vec<u32>>();
        gl!(self
            .gl
            .bind_buffer(glow::ARRAY_BUFFER, Some(self.transform_index_buffer)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            bytemuck::cast_slice(transform_indices.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} transform indices", transform_indices.len());

        // upload lights
        gl!(self
            .gl
            .bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.lights_ssbo)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            bytemuck::cast_slice(scene.lights.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} lights", scene.lights.len());

        // upload transforms
        let transforms = scene
            .objects
            .iter()
            .flat_map(|o| o.transform.to_homogeneous().as_slice().to_vec())
            .collect::<Vec<f32>>();
        gl!(self
            .gl
            .bind_buffer(glow::SHADER_STORAGE_BUFFER, Some(self.transform_ssbo)));
        gl!(self.gl.buffer_data_u8_slice(
            glow::SHADER_STORAGE_BUFFER,
            bytemuck::cast_slice(transforms.as_slice()),
            glow::STATIC_DRAW
        ));
        debug!("Uploaded {} transforms", transforms.len() / 16);

        gl!(self.gl.bind_buffer(glow::ARRAY_BUFFER, None));
        gl!(self.gl.bind_buffer(glow::SHADER_STORAGE_BUFFER, None));
        gl!(self.gl.use_program(None));

        Ok(scene.objects.iter().map(|o| o.triangles.len()).sum())
    }

    pub fn paint(&self, rect: Rect, painter: &Painter, scene: &Scene) -> anyhow::Result<()> {
        let triangle_count = self.upload_scene(rect, scene)?;
        debug!("Uploaded {} triangles", triangle_count);

        painter.add(Shape::Callback(PaintCallback {
            rect,
            callback: Arc::new(CallbackFn::new(self.callback(triangle_count))),
        }));

        Ok(())
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.program);
            self.gl.delete_vertex_array(self.vertex_array);
            self.gl.delete_buffer(self.position_buffer);
            self.gl.delete_buffer(self.normal_buffer);
            self.gl.delete_buffer(self.color_buffer);
            self.gl.delete_buffer(self.transform_index_buffer);
            self.gl.delete_buffer(self.lights_ssbo);
            self.gl.delete_buffer(self.transform_ssbo);
        }
    }
}

fn compile_program(
    gl: &glow::Context,
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

fn gl_log(source: u32, type_: u32, id: u32, severity: u32, msg: &str) {
    log::log!(
        match severity {
            glow::DEBUG_SEVERITY_HIGH => log::Level::Error,
            glow::DEBUG_SEVERITY_MEDIUM => log::Level::Warn,
            glow::DEBUG_SEVERITY_LOW => log::Level::Info,
            glow::DEBUG_SEVERITY_NOTIFICATION => log::Level::Debug,
            _ => log::Level::Trace,
        },
        "[OpenGL] {} {} {} {}",
        match source {
            glow::DEBUG_SOURCE_API => "API",
            glow::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
            glow::DEBUG_SOURCE_SHADER_COMPILER => "Shader Compiler",
            glow::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
            glow::DEBUG_SOURCE_APPLICATION => "Application",
            glow::DEBUG_SOURCE_OTHER => "Other",
            _ => "Unknown",
        },
        match type_ {
            glow::DEBUG_TYPE_ERROR => "Error",
            glow::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated Behavior",
            glow::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
            glow::DEBUG_TYPE_PORTABILITY => "Portability",
            glow::DEBUG_TYPE_PERFORMANCE => "Performance",
            glow::DEBUG_TYPE_MARKER => "Marker",
            glow::DEBUG_TYPE_PUSH_GROUP => "Push Group",
            glow::DEBUG_TYPE_POP_GROUP => "Pop Group",
            glow::DEBUG_TYPE_OTHER => "Other",
            _ => "Unknown",
        },
        id,
        msg
    );
}
