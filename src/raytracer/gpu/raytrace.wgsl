// Whitted-style raytracing (matches CPU path: direct lighting, transparency, reflections).

struct GpuGlobals {
    cam_pos: vec4<f32>,
    inv_rot_c0: vec4<f32>,
    inv_rot_c1: vec4<f32>,
    inv_rot_c2: vec4<f32>,
    fov_tan_half: f32,
    width: f32,
    height: f32,
    aspect: f32,
    block_origin_x: u32,
    block_origin_y: u32,
    block_size_x: u32,
    block_size_y: u32,
    full_width: u32,
    full_height: u32,
    tri_count: u32,
    node_count: u32,
    light_count: u32,
    samples: u32,
    sqrt_samples: u32,
    antialias: u32,
    max_depth: u32,
    _pad0: u32,
    _pad_pre_ambient: vec2<u32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    sky_color: vec3<f32>,
    sky_has_tex: u32,
    sky_width: u32,
    sky_height: u32,
    atlas_width: u32,
    atlas_height: u32,
    material_count: u32,
    delta: f32,
    _pad_tail: vec3<f32>,
    // Uniform address space: arrays of u32 must use vec4 stride; use explicit vec4 padding.
    _pad_uniform_end: vec4<u32>,
    _pad_uniform_end_b: vec4<u32>,
    _pad_uniform_end_c: vec4<u32>,
}

struct GpuTriangle {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    na: vec4<f32>,
    nb: vec4<f32>,
    nc: vec4<f32>,
    uv_a: vec2<f32>,
    uv_b: vec2<f32>,
    uv_c: vec2<f32>,
    material_index: u32,
    _pad: u32,
}

struct GpuBvhNode {
    child_l_min: vec4<f32>,
    child_l_max: vec4<f32>,
    child_r_min: vec4<f32>,
    child_r_max: vec4<f32>,
    child_l_index: u32,
    child_r_index: u32,
    child_l_is_leaf: u32,
    child_r_is_leaf: u32,
    kind: u32,
    tri_index: u32,
    _pad: vec2<u32>,
}

struct GpuMaterial {
    diffuse_color: vec3<f32>,
    specular_exponent: f32,
    specular_color: vec3<f32>,
    dissolve: f32,
    illum: i32,
    has_texture: u32,
    _pad_m: u32,
    _pad_before_uv: u32,
    texture_uv_min: vec2<f32>,
    texture_uv_max: vec2<f32>,
    _pad_end: u32,
    _pad_trailing0: u32,
    _pad_trailing1: u32,
    _pad_trailing2: u32,
}

struct GpuLight {
    position: vec3<f32>,
    _pad: f32,
    color: vec3<f32>,
    intensity: f32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct Hit {
    t: f32,
    tri_index: u32,
    u: f32,
    v: f32,
    w: f32,
}

const MAX_LIGHTS: u32 = 64u;

@group(0) @binding(0)
var<uniform> globals: GpuGlobals;
@group(0) @binding(1)
var<storage, read> triangles: array<GpuTriangle>;
@group(0) @binding(2)
var<storage, read> nodes: array<GpuBvhNode>;
@group(0) @binding(3)
var<storage, read> materials: array<GpuMaterial>;
@group(0) @binding(4)
var<storage, read> lights: array<GpuLight>;
@group(0) @binding(5)
var diffuse_atlas: texture_2d<f32>;
@group(0) @binding(6)
var sky_tex: texture_2d<f32>;
@group(0) @binding(7)
var<storage, read_write> block_pixels: array<vec4<f32>>;

/// `inv_rot_c*` are columns of `look_at_rh(...).inverse()` (camera → world), matching
/// `Camera::ray` / `Rotation3::inverse_transform_vector`. Must multiply as **M·v**, not Mᵀ·v.
fn mat3_inv_mul(v: vec3<f32>) -> vec3<f32> {
    let c0 = globals.inv_rot_c0.xyz;
    let c1 = globals.inv_rot_c1.xyz;
    let c2 = globals.inv_rot_c2.xyz;
    return vec3<f32>(
        c0.x * v.x + c1.x * v.y + c2.x * v.z,
        c0.y * v.x + c1.y * v.y + c2.y * v.z,
        c0.z * v.x + c1.z * v.y + c2.z * v.z,
    );
}

/// `dir_x` is the same x passed to CPU `Camera::ray` (already includes aspect).
fn camera_ray(dir_x: f32, ndc_y: f32) -> Ray {
    let dir_cam = vec3<f32>(dir_x, -ndc_y, -1.0 / globals.fov_tan_half);
    let dir_w = normalize(mat3_inv_mul(dir_cam));
    return Ray(globals.cam_pos.xyz, dir_w);
}

fn ray_aabb_hit(ro: vec3<f32>, rd: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> bool {
    let inv_r = 1.0 / rd;
    let t0 = (bmin - ro) * inv_r;
    let t1 = (bmax - ro) * inv_r;
    let tsm = min(t0, t1);
    let tlg = max(t0, t1);
    let tmin = max(max(tsm.x, tsm.y), tsm.z);
    let tmax = min(min(tlg.x, tlg.y), tlg.z);
    return tmax >= max(tmin, 0.0);
}

fn intersect_triangle(ray: Ray, tri: GpuTriangle) -> vec4<f32> {
    let ab = tri.b.xyz - tri.a.xyz;
    let ac = tri.c.xyz - tri.a.xyz;
    let p = cross(ray.direction, ac);
    let det = dot(ab, p);
    if abs(det) < 1e-8 {
        return vec4<f32>(-1.0, 0.0, 0.0, 0.0);
    }
    let inv_det = 1.0 / det;
    let ao = ray.origin - tri.a.xyz;
    let u = dot(ao, p) * inv_det;
    if u < 0.0 || u > 1.0 {
        return vec4<f32>(-1.0, 0.0, 0.0, 0.0);
    }
    let q = cross(ao, ab);
    let v = dot(ray.direction, q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return vec4<f32>(-1.0, 0.0, 0.0, 0.0);
    }
    let t_hit = dot(ac, q) * inv_det;
    if t_hit < globals.delta {
        return vec4<f32>(-1.0, 0.0, 0.0, 0.0);
    }
    let w = 1.0 - u - v;
    return vec4<f32>(t_hit, u, v, w);
}

fn intersect_scene(ray: Ray) -> Hit {
    var best = Hit(1e30, 0xffffffffu, 0.0, 0.0, 0.0);
    if globals.node_count == 0u {
        return best;
    }

    var stack: array<u32, 64>;
    var sp = 0u;
    stack[sp] = 0u;
    sp = 1u;

    loop {
        if sp == 0u {
            break;
        }
        sp -= 1u;
        let idx = stack[sp];
        let node = nodes[idx];

        if node.kind == 1u {
            let tri = triangles[node.tri_index];
            let th = intersect_triangle(ray, tri);
            if th.x > 0.0 && th.x < best.t {
                best = Hit(th.x, node.tri_index, th.y, th.z, th.w);
            }
            continue;
        }

        if ray_aabb_hit(ray.origin, ray.direction, node.child_l_min.xyz, node.child_l_max.xyz) {
            if node.child_l_is_leaf == 1u {
                let tri = triangles[node.child_l_index];
                let th = intersect_triangle(ray, tri);
                if th.x > 0.0 && th.x < best.t {
                    best = Hit(th.x, node.child_l_index, th.y, th.z, th.w);
                }
            } else if sp < 63u {
                stack[sp] = node.child_l_index;
                sp += 1u;
            }
        }

        if ray_aabb_hit(ray.origin, ray.direction, node.child_r_min.xyz, node.child_r_max.xyz) {
            if node.child_r_is_leaf == 1u {
                let tri = triangles[node.child_r_index];
                let th = intersect_triangle(ray, tri);
                if th.x > 0.0 && th.x < best.t {
                    best = Hit(th.x, node.child_r_index, th.y, th.z, th.w);
                }
            } else if sp < 63u {
                stack[sp] = node.child_r_index;
                sp += 1u;
            }
        }
    }

    return best;
}

fn resolve_visible_hit(ray: Ray) -> Hit {
    var r = ray;
    for (var step = 0u; step < 16u; step++) {
        let h = intersect_scene(r);
        if h.tri_index == 0xffffffffu {
            return h;
        }
        let mat_idx = min(triangles[h.tri_index].material_index, globals.material_count - 1u);
        let mat = materials[mat_idx];
        let illum = mat.illum;
        if illum == 6 || illum == 7 {
            let p = r.origin + r.direction * h.t;
            r.origin = p + r.direction * 0.05;
            continue;
        }
        return h;
    }
    return Hit(1e30, 0xffffffffu, 0.0, 0.0, 0.0);
}

fn tri_normal(tri: GpuTriangle, u: f32, v: f32, w: f32) -> vec3<f32> {
    return normalize(tri.na.xyz * w + tri.nb.xyz * u + tri.nc.xyz * v);
}

fn tri_uv(tri: GpuTriangle, u: f32, v: f32, w: f32) -> vec2<f32> {
    return tri.uv_a * w + tri.uv_b * u + tri.uv_c * v;
}

fn sample_atlas_bilinear(uv: vec2<f32>) -> vec3<f32> {
    let aw = i32(globals.atlas_width);
    let ah = i32(globals.atlas_height);
    let fx = uv.x * f32(aw - 1);
    let fy = uv.y * f32(ah - 1);
    let x0 = i32(floor(fx));
    let y0 = i32(floor(fy));
    let x1 = min(x0 + 1, aw - 1);
    let y1 = min(y0 + 1, ah - 1);
    let tx = fract(fx);
    let ty = fract(fy);
    let c00 = textureLoad(diffuse_atlas, vec2<i32>(x0, y0), 0).rgb;
    let c10 = textureLoad(diffuse_atlas, vec2<i32>(x1, y0), 0).rgb;
    let c01 = textureLoad(diffuse_atlas, vec2<i32>(x0, y1), 0).rgb;
    let c11 = textureLoad(diffuse_atlas, vec2<i32>(x1, y1), 0).rgb;
    let a = mix(c00, c10, tx);
    let b = mix(c01, c11, tx);
    return mix(a, b, ty);
}

fn diffuse_albedo(mat: GpuMaterial, uvw: vec2<f32>) -> vec3<f32> {
    if mat.has_texture != 0u {
        let mn = mat.texture_uv_min;
        let mx = mat.texture_uv_max;
        // Match CPU `texture()`: flip V (OBJ / image row order vs atlas).
        let uv_mesh = vec2<f32>(uvw.x, 1.0 - uvw.y);
        let uv = mix(mn, mx, uv_mesh);
        return sample_atlas_bilinear(uv);
    }
    return mat.diffuse_color;
}

fn reflect_vec(i: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return i - 2.0 * dot(i, n) * n;
}

fn sky_color_dir(dir: vec3<f32>) -> vec3<f32> {
    if globals.sky_has_tex == 0u {
        return globals.sky_color;
    }
    let d = normalize(dir);
    let x = (0.5 + atan2(d.z, d.x) / (6.28318530718)) * f32(globals.sky_width);
    let y = (0.5 - asin(clamp(d.y, -1.0, 1.0)) / 3.14159265359) * f32(globals.sky_height);
    let xi = i32(floor(x)) % i32(globals.sky_width);
    let yi = i32(floor(y)) % i32(globals.sky_height);
    let sx = (xi + i32(globals.sky_width)) % i32(globals.sky_width);
    let sy = (yi + i32(globals.sky_height)) % i32(globals.sky_height);
    return textureLoad(sky_tex, vec2<i32>(sx, sy), 0).rgb;
}

/// Matches CPU `shade_impl`: occlusion tint uses `running_color` (ambient + prior lights) ×
/// blocker `diffuse_color` × `dissolve`, then × `light.color` — same order as `raytracer/mod.rs`.
fn shadow_light_transmission(
    hit_p: vec3<f32>,
    light: GpuLight,
    running_color: vec3<f32>,
) -> vec3<f32> {
    let light_dir = normalize(light.position - hit_p);
    var r = Ray(hit_p + light_dir * globals.delta, light_dir);
    for (var step = 0u; step < 32u; step++) {
        let h = intersect_scene(r);
        if h.tri_index == 0xffffffffu {
            return light.color;
        }
        let mat_idx = min(triangles[h.tri_index].material_index, globals.material_count - 1u);
        let mat = materials[mat_idx];
        if mat.illum == 6 || mat.illum == 7 {
            let p = r.origin + r.direction * h.t;
            r.origin = p + r.direction * 0.05;
            continue;
        }
        return running_color * mat.diffuse_color * mat.dissolve * light.color;
    }
    return vec3<f32>(0.0, 0.0, 0.0);
}

/// Direct lighting + ambient only (no mirror reflection term). WGSL forbids recursion.
fn shade_no_reflection(ray: Ray) -> vec3<f32> {
    let h0 = resolve_visible_hit(ray);
    if h0.tri_index == 0xffffffffu {
        return sky_color_dir(ray.direction);
    }

    let tri = triangles[h0.tri_index];
    let mat_idx = min(tri.material_index, globals.material_count - 1u);
    let mat = materials[mat_idx];
    let n = tri_normal(tri, h0.u, h0.v, h0.w);
    let uvw = tri_uv(tri, h0.u, h0.v, h0.w);
    let diffuse = diffuse_albedo(mat, uvw);
    let spec = mat.specular_color;
    let hit_p = ray.origin + ray.direction * h0.t;

    var color = globals.ambient_color * diffuse * globals.ambient_intensity;

    for (var li = 0u; li < min(globals.light_count, MAX_LIGHTS); li++) {
        let light = lights[li];
        let light_dir = normalize(light.position - hit_p);
        let light_trans = shadow_light_transmission(hit_p, light, color);

        if length(light_trans) < 0.01 {
            continue;
        }

        let dist2 = dot(light.position - hit_p, light.position - hit_p);
        let light_intensity = light.intensity / max(dist2, 1e-8);
        let diff_int = max(dot(light_dir, n), 0.0) * light_intensity;
        color += diffuse * light_trans * diff_int;

        if mat.illum == 2 {
            let refl_dir = reflect_vec(-ray.direction, n);
            let spec_int =
                pow(max(dot(light_dir, refl_dir), 0.0), mat.specular_exponent) * light_intensity;
            color += spec * light_trans * spec_int;
        }
    }

    return color;
}

fn shade(ray: Ray, depth: u32) -> vec3<f32> {
    if depth >= globals.max_depth {
        return sky_color_dir(ray.direction);
    }

    let h0 = resolve_visible_hit(ray);
    if h0.tri_index == 0xffffffffu {
        return sky_color_dir(ray.direction);
    }

    let tri = triangles[h0.tri_index];
    let mat_idx = min(tri.material_index, globals.material_count - 1u);
    let mat = materials[mat_idx];
    let n = tri_normal(tri, h0.u, h0.v, h0.w);
    let uvw = tri_uv(tri, h0.u, h0.v, h0.w);
    let diffuse = diffuse_albedo(mat, uvw);
    let spec = mat.specular_color;
    let hit_p = ray.origin + ray.direction * h0.t;

    var color = globals.ambient_color * diffuse * globals.ambient_intensity;

    for (var li = 0u; li < min(globals.light_count, MAX_LIGHTS); li++) {
        let light = lights[li];
        let light_dir = normalize(light.position - hit_p);
        let light_trans = shadow_light_transmission(hit_p, light, color);

        if length(light_trans) < 0.01 {
            continue;
        }

        let dist2 = dot(light.position - hit_p, light.position - hit_p);
        let light_intensity = light.intensity / max(dist2, 1e-8);
        let diff_int = max(dot(light_dir, n), 0.0) * light_intensity;
        color += diffuse * light_trans * diff_int;

        if mat.illum == 2 {
            let refl_dir = reflect_vec(-ray.direction, n);
            let spec_int =
                pow(max(dot(light_dir, refl_dir), 0.0), mat.specular_exponent) * light_intensity;
            color += spec * light_trans * spec_int;
        }

        if mat.illum == 3 || mat.illum == 4 {
            let refl_ray = Ray(hit_p + n * globals.delta, reflect_vec(ray.direction, n));
            // WGSL forbids recursion; inner term is direct-only (no further mirror bounces on GPU).
            var inner: vec3<f32>;
            if depth + 1u >= globals.max_depth {
                inner = sky_color_dir(refl_ray.direction);
            } else {
                inner = shade_no_reflection(refl_ray);
            }
            color += color * inner;
        }
    }

    return color;
}

fn hash01(p: vec2<f32>, seed: u32) -> f32 {
    let v = sin(dot(p, vec2<f32>(12.9898, 78.233)) + f32(seed) * 43.0);
    return fract(v * 43758.5453);
}

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let bx = globals.block_origin_x;
    let by = globals.block_origin_y;
    let bw = globals.block_size_x;
    let bh = globals.block_size_y;
    let lx = gid.x;
    let ly = gid.y;
    if lx >= bw || ly >= bh {
        return;
    }

    let px = bx + lx;
    let py = by + ly;
    let fw = globals.full_width;
    let fh = globals.full_height;
    if px >= fw || py >= fh {
        return;
    }

    let idx = lx + ly * bw;
    var col = vec3<f32>(0.0, 0.0, 0.0);

    if globals.antialias != 0u {
        let spp = globals.samples;
        let root = globals.sqrt_samples;
        for (var s = 0u; s < spp; s++) {
            let xi = s % root;
            let yi = s / root;
            let jx = hash01(vec2<f32>(f32(px), f32(py)), s * 2u);
            let jy = hash01(vec2<f32>(f32(px), f32(py)), s * 2u + 1u);
            let jitter_x =
                (f32(px) + (f32(xi) + jx * 2.0 - 1.0) / f32(root)) / f32(fw);
            let jitter_y =
                (f32(py) + (f32(yi) + jy * 2.0 - 1.0) / f32(root)) / f32(fh);
            let dir_x = (jitter_x * 2.0 - 1.0) * globals.aspect;
            let ndc_y = jitter_y * 2.0 - 1.0;
            let ray = camera_ray(dir_x, ndc_y);
            col += shade(ray, 0u);
        }
        col /= f32(spp);
    } else {
        let dir_x = ((f32(px) / f32(fw)) * 2.0 - 1.0) * globals.aspect;
        let ndc_y = (f32(py) / f32(fh)) * 2.0 - 1.0;
        let ray = camera_ray(dir_x, ndc_y);
        col = shade(ray, 0u);
    }

    block_pixels[idx] = vec4<f32>(col, 1.0);
}
