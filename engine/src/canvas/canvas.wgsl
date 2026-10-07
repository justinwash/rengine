struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color:    vec4<f32>,
    @location(2) uv:       vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       color:         vec4<f32>,
    @location(1)       uv:            vec2<f32>,
};

@group(0) @binding(0) var canvas_texture: texture_2d<f32>;
@group(0) @binding(1) var canvas_sampler: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    out.color         = in.color;
    out.uv            = in.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(canvas_texture, canvas_sampler, in.uv);
    return sample * in.color;
}

// Font atlases: glyph coverage, applied as if blended in sRGB space.
//
// The surface blends in linear light, where a half-covered pixel of light
// text on a dark backdrop comes out at 73% brightness rather than 50%: every
// edge is heavier, and small light text reads bold and runs together. Blended
// in sRGB space, as other renderers effectively are, the same pixel is 50%.
//
// Over black, matching that is exactly `coverage^2.2`, whatever the text's
// colour; over white it is `1 - (1 - coverage)^2.2` for black text. The
// backdrop is not known here, so the text's own lightness stands in for it:
// light text is taken to sit on a dark backdrop and dark text on a light one.
// Coverage of 0 or 1 (solid fills, pixel faces) is unchanged either way.
//
// The exponent is 1.45, not the full 2.2. Compared side by side on 10-15px
// UI text (2026-10-07): 1.0 runs letters together, 1.8 and up read thin —
// desktop renderers pair a full correction with a contrast boost for that
// reason — and 1.45 keeps the face's weight with clean gaps.
const TEXT_GAMMA: f32 = 1.45;

@fragment
fn fs_text(in: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(canvas_texture, canvas_sampler, in.uv);
    let a = sample.a;
    let lightness = pow(dot(in.color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)), 1.0 / 2.2);
    let on_dark = pow(a, TEXT_GAMMA);
    let on_light = 1.0 - pow(1.0 - a, TEXT_GAMMA);
    let coverage = mix(on_light, on_dark, smoothstep(0.15, 0.4, lightness));
    return vec4<f32>(sample.rgb, coverage) * in.color;
}
