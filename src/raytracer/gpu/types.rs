//! GPU buffer layouts matching `raytrace.wgsl`.

use bytemuck::{Pod, Zeroable};

/// Uniform block (must match `raytrace.wgsl` `GpuGlobals` exactly).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuGlobals {
    pub cam_pos: [f32; 4],
    pub inv_rot_c0: [f32; 4],
    pub inv_rot_c1: [f32; 4],
    pub inv_rot_c2: [f32; 4],
    pub fov_tan_half: f32,
    pub width: f32,
    pub height: f32,
    pub aspect: f32,
    pub block_origin_x: u32,
    pub block_origin_y: u32,
    pub block_size_x: u32,
    pub block_size_y: u32,
    pub full_width: u32,
    pub full_height: u32,
    pub tri_count: u32,
    pub node_count: u32,
    pub light_count: u32,
    pub samples: u32,
    pub sqrt_samples: u32,
    pub antialias: u32,
    pub max_depth: u32,
    pub _pad0: u32,
    pub _pad_pre_ambient: [u32; 2],
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub sky_color: [f32; 3],
    pub sky_has_tex: u32,
    pub sky_width: u32,
    pub sky_height: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub material_count: u32,
    pub delta: f32,
    /// WGSL uniform: `vec3` after `f32` is aligned to 16 bytes.
    pub _pad_after_delta: [u32; 2],
    pub _pad_tail: [f32; 3],
    /// WGSL uniform: `vec4` after `vec3` needs 4 bytes padding before next `vec4` block.
    pub _pad_before_uniform_end: u32,
    pub _pad_uniform_end: [u32; 4],
    pub _pad_uniform_end_b: [u32; 4],
    pub _pad_uniform_end_c: [u32; 4],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuTriangle {
    pub a: [f32; 4],
    pub b: [f32; 4],
    pub c: [f32; 4],
    pub na: [f32; 4],
    pub nb: [f32; 4],
    pub nc: [f32; 4],
    pub uv_a: [f32; 2],
    pub uv_b: [f32; 2],
    pub uv_c: [f32; 2],
    pub material_index: u32,
    pub _pad: u32,
}

/// Mirrors one `bvh::BvhNode`: inner nodes use child AABBs; leaf nodes store `tri_index`.
#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuBvhNode {
    pub child_l_min: [f32; 4],
    pub child_l_max: [f32; 4],
    pub child_r_min: [f32; 4],
    pub child_r_max: [f32; 4],
    pub child_l_index: u32,
    pub child_r_index: u32,
    pub child_l_is_leaf: u32,
    pub child_r_is_leaf: u32,
    pub kind: u32,
    pub tri_index: u32,
    pub _pad: [u32; 2],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuMaterial {
    pub diffuse_color: [f32; 3],
    pub specular_exponent: f32,
    pub specular_color: [f32; 3],
    pub dissolve: f32,
    pub illum: i32,
    pub has_texture: u32,
    pub _pad_m: u32,
    /// WGSL aligns `vec2<f32>` to 8; Rust `[f32; 2]` does not — pad so UV bounds match the shader.
    pub _pad_before_uv: u32,
    pub texture_uv_min: [f32; 2],
    pub texture_uv_max: [f32; 2],
    pub _pad_end: u32,
    /// Pad to WGSL struct size / `array<GpuMaterial>` stride (multiple of 16).
    pub _pad_trailing: [u32; 3],
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GpuLight {
    pub position: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 3],
    pub intensity: f32,
}
