@group(0) @binding(1) var sprite_tex: texture_2d<f32>;
@group(0) @binding(2) var sprite_sampler: sampler;

@fragment
fn main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return textureSample(sprite_tex, sprite_sampler, uv);
}
