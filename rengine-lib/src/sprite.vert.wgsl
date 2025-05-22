// WGSL vertex shader for a fullscreen quad with position/size uniforms
struct SpriteUniforms {
    sprite_pos: vec2<f32>,
    sprite_size: vec2<f32>,
    screen_size: vec2<f32>,
};

struct VertexInput {
    @location(0) position: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: SpriteUniforms;

@vertex
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Convert sprite position/size to normalized device coordinates
    let pos = uniforms.sprite_pos + input.position * uniforms.sprite_size;
    let ndc = vec2<f32>(
        (pos.x / uniforms.screen_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / uniforms.screen_size.y) * 2.0
    );
    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = input.position;
    return out;
}
