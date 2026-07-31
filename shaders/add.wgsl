@group(0) @binding(0)
var t_a: texture_2d<f32>;
@group(0) @binding(1)
var s_a: sampler;
@group(0) @binding(2)
var t_b: texture_2d<f32>;
@group(0) @binding(3)
var s_b: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32((idx << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(idx & 2u) * 2.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) * 0.5, 1.0 - (y + 1.0) * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let a = textureSample(t_a, s_a, in.uv);
    let b = textureSample(t_b, s_b, in.uv);
    return vec4<f32>(min(a.rgb + b.rgb, vec3<f32>(1.0)), 1.0);
}
