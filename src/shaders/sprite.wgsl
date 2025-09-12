// Vertex shader
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct SpriteUniform {
    view_proj: mat4x4<f32>,
    transform: mat4x4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> sprite_uniform: SpriteUniform;

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_position = sprite_uniform.transform * vec4<f32>(model.position, 0.0, 1.0);
    out.clip_position = sprite_uniform.view_proj * world_position;
    out.tex_coords = model.tex_coords;
    return out;
}

// Fragment shader
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    return tex_color * sprite_uniform.color;
}