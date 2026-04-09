struct Uniforms {
    view_proj:   mat4x4<f32>,
    light_dir:   vec4<f32>,
    light_color: vec4<f32>,
    ambient:     vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal:   vec3<f32>,
    @location(2) color:    vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       world_normal:  vec3<f32>,
    @location(1)       color:         vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_normal  = in.normal;
    out.color         = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let ndotl = max(dot(n, u.light_dir.xyz), 0.0);
    let diffuse = u.light_color.rgb * u.light_color.a * ndotl;
    let ambient = u.ambient.rgb * u.ambient.a;
    let lit = in.color.rgb * (diffuse + ambient);
    return vec4<f32>(lit, in.color.a);
}
