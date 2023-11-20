struct VertexOut {
    @location(0) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Uniforms {
    view: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    @location(0) Position : vec3<f32>,
    @location(1) Color : vec3<f32>
    ) -> VertexOut {
    var out: VertexOut;

    out.position = uniforms.view * vec4<f32>(Position, 1.0);
    out.color = vec4<f32>(Color, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return in.color;
}