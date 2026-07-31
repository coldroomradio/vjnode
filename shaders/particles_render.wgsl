struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

struct Uniforms {
    time: f32,
    dt: f32,
    bass: f32,
    mid: f32,
    treble: f32,
    rms: f32,
};

@group(0) @binding(0)
var<storage, read> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> u: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_uv: vec2<f32>,
};

// 2 triangles (6 verts) forming a small quad in local -1..1 space,
// offset per-instance by that particle's position. No vertex buffer —
// the quad shape is hardcoded here, position comes from the storage
// buffer via instance_index.
@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var offsets = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    let particle = particles[instance_index];
    let local = offsets[vertex_index];

    // Fixed-ish screen-space size, pulses slightly with loudness.
    let size = 0.006 + u.rms * 0.01;
    let world_pos = particle.position + local * size;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    out.local_uv = local;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft circular falloff so each particle reads as a glowing dot
    // instead of a hard square.
    let d = length(in.local_uv);
    let alpha = smoothstep(1.0, 0.0, d);

    let color = vec3<f32>(0.3 + u.treble, 0.5 + u.mid * 0.5, 0.9 + u.bass * 0.3);
    return vec4<f32>(color * alpha, alpha);
}
