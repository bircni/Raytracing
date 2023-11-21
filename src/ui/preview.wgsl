struct VertexOut {
    @builtin(position) result: vec4<f32>,
    @location(0) position: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
}

struct Uniforms {
    view: mat4x4<f32>,
    lights_count: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> VertexOut {
    var out: VertexOut;

    out.result = uniforms.view * vec4<f32>(position, 1.0);
    out.position = vec4<f32>(position, 1.0);
    out.normal = normal;
    out.color = vec4<f32>(color, 1.0);

    return out;
}

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
}

@group(0) @binding(1)
var<storage, read> lights: array<Light>;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    var AMBIENT_STRENGTH: f32 = 0.1;

    var color: vec3<f32> = in.color.xyz * AMBIENT_STRENGTH;

    for (var i = 0u; i < uniforms.lights_count; i = i + 1u) {
        var light: Light = lights[i];
        var light_dir: vec3<f32> = normalize(light.position - in.position.xyz);
        var diff: f32 = max(dot(in.normal, light_dir), 0.0);
        color = color + light.color * diff * light.intensity / pow(length(light.position - in.position.xyz), 2.0);
    }

    return vec4<f32>(color, 1.0);
}