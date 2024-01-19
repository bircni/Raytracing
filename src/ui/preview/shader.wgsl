struct VertexOut {
    @builtin(position) result: vec4<f32>,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

struct Uniforms {
    view: mat4x4<f32>,
    lights_count: u32,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

@group(0) @binding(1)
var<storage, read> lights: array<Light>;

@group(0) @binding(2)
var<storage, read> transforms: array<mat4x4<f32>>;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) transform_index: u32,
) -> VertexOut {
    var out: VertexOut;

    var transform: mat4x4<f32> = transforms[transform_index];

    out.result = uniforms.view * transform * vec4<f32>(position, 1.0);
    out.position = (transform * vec4<f32>(position, 1.0)).xyz;
    out.normal = (transform * vec4<f32>(normal, 0.0)).xyz;
    out.color = color;

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var color: vec3<f32> = uniforms.ambient_color * uniforms.ambient_intensity * in.color;

    for (var i = 0u; i < uniforms.lights_count; i = i + 1u) {
        var light: Light = lights[i];
        var light_dir: vec3<f32> = normalize(light.position - in.position.xyz);
        var diff: f32 = max(dot(in.normal, light_dir), 0.0);
        color = color + in.color * light.color * diff * light.intensity / pow(length(light.position - in.position.xyz), 2.0);
    }

    return vec4<f32>(color, 1.0);
}