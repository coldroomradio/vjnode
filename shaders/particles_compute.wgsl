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
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<uniform> u: Uniforms;

// Cheap hash-based pseudo-noise — good enough for organic-looking drift,
// much cheaper than real Perlin/simplex. Swap this out later if you want
// smoother flow fields.
fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= arrayLength(&particles)) {
        return;
    }

    var p = particles[i];

    // Sample a noise field at two nearby offsets and use the difference
    // as a flow direction — a cheap curl-noise approximation. Bass
    // pushes the flow harder, treble speeds up how fast particles
    // actually move along it.
    let t = u.time * 0.15;
    let n1 = hash(p.position * 3.0 + vec2<f32>(t, 0.0));
    let n2 = hash(p.position * 3.0 + vec2<f32>(0.0, t) + vec2<f32>(0.01, 0.0));
    let flow = vec2<f32>(n2 - n1, n1 - 0.5) * (0.4 + u.bass * 2.5);

    p.velocity = p.velocity * 0.98 + flow * u.dt;
    p.position = p.position + p.velocity * u.dt * (1.0 + u.treble * 3.0);

    // wrap around the -1..1 normalized screen space
    if (p.position.x > 1.0) { p.position.x = -1.0; }
    if (p.position.x < -1.0) { p.position.x = 1.0; }
    if (p.position.y > 1.0) { p.position.y = -1.0; }
    if (p.position.y < -1.0) { p.position.y = 1.0; }

    particles[i] = p;
}
