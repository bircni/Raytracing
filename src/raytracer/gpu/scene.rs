//! Build GPU scene buffers: world-space triangles, global BVH, diffuse atlas, sky texture.

use bvh::bvh::{Bvh, BvhNode};
use image::{Rgb, RgbImage};
use nalgebra::Vector3;

use crate::scene::{Color, Material, Scene, Skybox, Triangle};

use super::types::{GpuBvhNode, GpuLight, GpuMaterial, GpuTriangle};

pub struct GpuSceneData {
    pub triangles: Vec<GpuTriangle>,
    pub nodes: Vec<GpuBvhNode>,
    pub materials: Vec<GpuMaterial>,
    pub lights: Vec<GpuLight>,
    pub atlas: RgbImage,
    pub sky: Option<RgbImage>,
    pub sky_color: Color,
}

const fn aabb_corners(
    aabb: &bvh::aabb::Aabb<f32, 3>,
) -> (nalgebra::Point3<f32>, nalgebra::Point3<f32>) {
    (aabb.min, aabb.max)
}

fn classify_child(nodes: &[BvhNode<f32, 3>], child_index: usize) -> (u32, u32) {
    match &nodes[child_index] {
        BvhNode::Leaf { shape_index, .. } => (*shape_index as u32, 1u32),
        BvhNode::Node { .. } => (child_index as u32, 0u32),
    }
}

fn pack_bvh_nodes(nodes: &[BvhNode<f32, 3>]) -> Vec<GpuBvhNode> {
    nodes
        .iter()
        .map(|node| match node {
            BvhNode::Leaf { shape_index, .. } => GpuBvhNode {
                child_l_min: [0.0; 4],
                child_l_max: [0.0; 4],
                child_r_min: [0.0; 4],
                child_r_max: [0.0; 4],
                child_l_index: 0,
                child_r_index: 0,
                child_l_is_leaf: 0,
                child_r_is_leaf: 0,
                kind: 1,
                tri_index: *shape_index as u32,
                _pad: [0; 2],
            },
            BvhNode::Node {
                child_l_aabb,
                child_l_index,
                child_r_aabb,
                child_r_index,
                ..
            } => {
                let (l_min, l_max) = aabb_corners(child_l_aabb);
                let (r_min, r_max) = aabb_corners(child_r_aabb);
                let (l_idx, l_leaf) = classify_child(nodes, *child_l_index);
                let (r_idx, r_leaf) = classify_child(nodes, *child_r_index);
                GpuBvhNode {
                    child_l_min: [l_min.x, l_min.y, l_min.z, 0.0],
                    child_l_max: [l_max.x, l_max.y, l_max.z, 0.0],
                    child_r_min: [r_min.x, r_min.y, r_min.z, 0.0],
                    child_r_max: [r_max.x, r_max.y, r_max.z, 0.0],
                    child_l_index: l_idx,
                    child_r_index: r_idx,
                    child_l_is_leaf: l_leaf,
                    child_r_is_leaf: r_leaf,
                    kind: 0,
                    tri_index: 0,
                    _pad: [0; 2],
                }
            }
        })
        .collect()
}

struct AtlasEntry {
    min: [f32; 2],
    max: [f32; 2],
}

fn build_atlas(materials: &[&Material]) -> (RgbImage, Vec<AtlasEntry>) {
    let n = materials.len();
    let mut entries: Vec<AtlasEntry> = (0..n)
        .map(|_| AtlasEntry {
            min: [0.0, 0.0],
            max: [1.0, 1.0],
        })
        .collect();

    let mut max_w = 1u32;
    let mut sum_h = 0u32;
    for m in materials {
        if let Some(img) = m.diffuse_texture.as_ref() {
            max_w = max_w.max(img.width());
            sum_h = sum_h.saturating_add(img.height());
        }
    }
    if sum_h == 0 {
        sum_h = 1;
    }
    let atlas_w = max_w.max(1);
    let atlas_h = sum_h.max(1);
    let mut atlas = RgbImage::from_pixel(atlas_w, atlas_h, Rgb([255, 255, 255]));

    let mut y = 0u32;
    for (i, m) in materials.iter().enumerate() {
        if let Some(img) = m.diffuse_texture.as_ref() {
            let w = img.width();
            let h = img.height();
            for yy in 0..h {
                for xx in 0..w {
                    atlas.put_pixel(xx, y + yy, *img.get_pixel(xx, yy));
                }
            }
            let aw = atlas_w as f32;
            let ah = atlas_h as f32;
            entries[i] = AtlasEntry {
                min: [0.0, y as f32 / ah],
                max: [w as f32 / aw, (y + h) as f32 / ah],
            };
            y += h;
        }
    }

    (atlas, entries)
}

fn pack_material(m: &Material, entry: &AtlasEntry) -> GpuMaterial {
    GpuMaterial {
        diffuse_color: m
            .diffuse_color
            .map_or([0.9, 0.9, 0.9], |c| [c.x, c.y, c.z]),
        specular_exponent: m.specular_exponent.unwrap_or(1.0),
        specular_color: m
            .specular_color
            .map_or([0.9, 0.9, 0.9], |c| [c.x, c.y, c.z]),
        dissolve: m.dissolve.unwrap_or(1.0),
        illum: m.illumination_model.as_i32(),
        has_texture: u32::from(m.diffuse_texture.is_some()),
        _pad_m: 0,
        _pad_before_uv: 0,
        texture_uv_min: entry.min,
        texture_uv_max: entry.max,
        _pad_end: 0,
        _pad_trailing: [0; 3],
    }
}

const fn default_gpu_material() -> GpuMaterial {
    GpuMaterial {
        diffuse_color: [0.9, 0.9, 0.9],
        specular_exponent: 1.0,
        specular_color: [0.9, 0.9, 0.9],
        dissolve: 1.0,
        illum: 0,
        has_texture: 0,
        _pad_m: 0,
        _pad_before_uv: 0,
        texture_uv_min: [0.0, 0.0],
        texture_uv_max: [1.0, 1.0],
        _pad_end: 0,
        _pad_trailing: [0; 3],
    }
}

fn to_gpu_triangle(tri: &Triangle, material_index: u32) -> GpuTriangle {
    GpuTriangle {
        a: [tri.a.x, tri.a.y, tri.a.z, 0.0],
        b: [tri.b.x, tri.b.y, tri.b.z, 0.0],
        c: [tri.c.x, tri.c.y, tri.c.z, 0.0],
        na: [tri.a_normal.x, tri.a_normal.y, tri.a_normal.z, 0.0],
        nb: [tri.b_normal.x, tri.b_normal.y, tri.b_normal.z, 0.0],
        nc: [tri.c_normal.x, tri.c_normal.y, tri.c_normal.z, 0.0],
        uv_a: [tri.a_uv.x, tri.a_uv.y],
        uv_b: [tri.b_uv.x, tri.b_uv.y],
        uv_c: [tri.c_uv.x, tri.c_uv.y],
        material_index,
        _pad: 0,
    }
}

/// Build packed scene for the GPU path. Returns `None` if there are no triangles.
pub fn build_gpu_scene(scene: &Scene) -> Option<GpuSceneData> {
    let flat: Vec<&Material> = scene
        .objects
        .iter()
        .flat_map(|o| o.materials.iter())
        .collect();

    let (atlas, entries) = build_atlas(&flat);

    let mut gpu_materials: Vec<GpuMaterial> = vec![default_gpu_material()];
    for (m, e) in flat.iter().zip(entries.iter()) {
        gpu_materials.push(pack_material(m, e));
    }

    let mut world_tris: Vec<Triangle> = Vec::new();
    let mut mat_base: usize = 1;
    for obj in &scene.objects {
        let t = obj.transform();
        for tri in &obj.triangles {
            let gidx = tri.material_index.map_or(0, |i| mat_base + i);
            let wt = Triangle::new(
                t.transform_point(&tri.a),
                t.transform_point(&tri.b),
                t.transform_point(&tri.c),
                t.transform_vector(&tri.a_normal)
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_else(Vector3::x),
                t.transform_vector(&tri.b_normal)
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_else(Vector3::x),
                t.transform_vector(&tri.c_normal)
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_else(Vector3::x),
                tri.a_uv,
                tri.b_uv,
                tri.c_uv,
                Some(gidx),
            );
            world_tris.push(wt);
        }
        mat_base += obj.materials.len();
    }

    if world_tris.is_empty() {
        return None;
    }

    let mut slice = world_tris;
    let bvh = Bvh::build(&mut slice);
    let nodes = pack_bvh_nodes(&bvh.nodes);
    let gpu_tris: Vec<GpuTriangle> = slice
        .iter()
        .map(|t| {
            let mid = t.material_index.map_or(0, |i| i as u32);
            to_gpu_triangle(t, mid)
        })
        .collect();

    let lights: Vec<GpuLight> = scene
        .lights
        .iter()
        .map(|l| GpuLight {
            position: [l.position.x, l.position.y, l.position.z],
            _pad: 0.0,
            color: [l.color.x, l.color.y, l.color.z],
            intensity: l.intensity,
        })
        .collect();

    let (sky, sky_color) = match &scene.settings.skybox {
        Skybox::Image { image, .. } => (Some(image.clone()), Color::new(0.0, 0.0, 0.0)),
        Skybox::Color(c) => (None, *c),
    };

    Some(GpuSceneData {
        triangles: gpu_tris,
        nodes,
        materials: gpu_materials,
        lights,
        atlas,
        sky,
        sky_color,
    })
}
