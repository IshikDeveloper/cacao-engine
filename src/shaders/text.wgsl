// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct TextUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> text_uniform: TextUniform;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_position = vec4<f32>(model.position, 0.0, 1.0);
    out.clip_position = text_uniform.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    out.color = model.color;
    return out;
}

// Fragment shader
@group(1) @binding(0)
var t_font: texture_2d<f32>;
@group(1) @binding(1)
var s_font: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(t_font, s_font, in.tex_coords).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}