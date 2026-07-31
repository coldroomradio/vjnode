struct TimeUniform {
    time: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    rms: f32,
};
@group(0) @binding(0)
var<uniform> u_time: TimeUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle trick: 3 vertices, no vertex buffer, covers the
// whole screen with one triangle (cheaper than two triangles / a quad).
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
    // Bass drives how fast the pattern churns, mid/treble drive per-channel
    // brightness, rms pulses overall brightness with loudness.
    let t = u_time.time * (0.5 + u_time.bass * 3.0);
    let brightness = 0.6 + u_time.rms * 0.5;
    let r = (0.5 + 0.5 * sin(t + in.uv.x * 6.28318)) * (0.4 + u_time.bass * 0.6) * brightness;
    let g = (0.5 + 0.5 * sin(t * 1.3 + in.uv.y * 6.28318)) * (0.4 + u_time.mid * 0.6) * brightness;
    let b = (0.5 + 0.5 * sin(t * 0.7 + (in.uv.x + in.uv.y) * 3.14159)) * (0.4 + u_time.treble * 0.6) * brightness;
    return vec4<f32>(r, g, b, 1.0);
}
