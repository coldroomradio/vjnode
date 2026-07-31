# vjnode — starter scaffold

A native (no browser, no webview) real-time GPU renderer using Rust + wgpu
(talks to Metal directly on your Mac). Right now it's a fixed two-node
chain — an animated color source feeding an invert effect, drawn to the
window — as a working example of the pattern you'll generalize into a
full node graph.

## Setup

1. Install Rust if you don't have it: https://rustup.rs
2. Open this folder in VS Code (install the `rust-analyzer` extension —
   it'll prompt you).
3. In a terminal, from this folder:
   ```
   cargo run --release
   ```
   First build will take a couple minutes (compiling wgpu + deps). You
   should get a window with a shifting animated color, inverted.

### If the build errors

wgpu/winit APIs shift between minor versions and I wrote this without
a compiler in front of me to verify against, so there's a real chance
of a small mismatch (a renamed field, an extra struct member some
version added). If `cargo build` throws an error, paste it back to me
— these are almost always a one-line fix, not a structural problem.

## How this maps to what you actually want

- **Current graph shape** (built in `state.rs`):
  ```
  Color Source ──▶ Invert ──┐
                              ├──▶ Add ──▶ screen
  Particles (compute+render) ┘
  ```
  Two independent branches (a shader source and a GPU particle sim)
  combined additively — a real small DAG, not just a chain.
- **`src/nodes.rs` → `ParticleNode`** — a genuine compute-shader
  simulation: `shaders/particles_compute.wgsl` moves 20,000 particles
  each frame in a storage buffer using an audio-reactive pseudo-flow-
  field (bass pushes flow strength, treble speeds up motion),
  `shaders/particles_render.wgsl` draws each one as an instanced
  glowing quad (additive blend, soft circular falloff) — no vertex
  buffer, position comes straight from the storage buffer via
  `instance_index`. This is the pattern for every future sim
  (boids, water, flow fields) — only the compute shader's math changes.
- **`src/nodes.rs` → `AddNode`** — a real 2-input node, additively
  blending two upstream textures. This is the piece that makes the
  graph an actual DAG instead of a straight line; `multiply`/`blend`
  nodes are the same shape with a different fragment shader.
- **`src/ui.rs`** — a small read-only egui overlay: lists the current
  node chain in execution order and shows live bass/mid/treble/rms
  meters. No editing yet, just visibility. `main.rs` forwards every
  window event to it via `state.handle_window_event()`.
- **`src/graph.rs`** — the node system. `GraphNode` is a trait: read some
  input textures, write an output texture, that's the whole contract.
  `Graph` owns every node's offscreen texture and runs them in order.
- **`src/nodes.rs`** — concrete nodes: `ColorSourceNode` (0 inputs, an
  audio-reactive generator) and `InvertNode` (1 input, inverts colors).
  This is the file you'll keep adding to — bloom, glow, multiply, add,
  blend, noise, dither all follow the exact same shape as `InvertNode`:
  a pipeline + a bind group layout + a `render()` that samples its
  input(s) and writes its output.
- **Adding a node to the chain** is one `add_node` call in `state.rs`,
  pointing at the index of whatever should feed it:
  ```rust
  let source = graph.add_node(&device, Box::new(ColorSourceNode::new(...)), vec![]);
  let inverted = graph.add_node(&device, Box::new(InvertNode::new(...)), vec![source]);
  graph.add_node(&device, Box::new(YourNewNode::new(...)), vec![inverted]);
  ```
- **`InvertNode` rebuilds its bind group every frame** from whatever
  texture view the graph hands it as input, rather than baking in a
  reference to a specific upstream node at construction time. That's
  the detail that will let you actually rewire connections at runtime
  later, instead of the graph being fixed at startup.
- **`update()`** in `state.rs` is now mostly empty — each node manages
  its own time/audio state internally (see `ColorSourceNode`). It's
  the hook point for anything graph-wide later, like a global
  intensity control from a UI.

## Roadmap (in the order I'd build it)

1. ~~**Audio input.**~~ Done — `src/audio.rs` captures from the default
   input device via `cpal`, runs an FFT (`rustfft`) on incoming audio,
   and exposes live bass/mid/treble/rms as `AudioBands`, updated
   continuously in a background thread. **Note:** this listens to
   whatever your Mac's default audio *input* is — a mic by default. To
   react to a song playing on your Mac (not through a mic), route
   system/app output into an input device — the standard free tool for
   that on macOS is **BlackHole**, set up as a Multi-Output alongside
   your speakers so you still hear it too.
2. ~~**Generalize the fixed chain into a real graph.**~~ Done — see
   `src/graph.rs` / `src/nodes.rs` above. Still a fixed chain built once
   at startup, not yet editable at runtime — that's step 7 below.
2.5. ~~**Minimal debug overlay.**~~ Done — `src/ui.rs`, read-only node
   list + audio meters. Not the draggable editor yet, just visibility.
3. **File playback.** Use `symphonia` to decode mp3/wav/etc for "load a
   song from disk" and pipe its samples through the same FFT analysis
   path `audio.rs` already has, instead of only live input.
4. ~~**Particles & sims.**~~ Done — `ParticleNode` in `src/nodes.rs`,
   see above. Next sim ideas from here (flow fields, boids, a simple
   water displacement) all reuse this exact compute-pipeline pattern —
   copy `ParticleNode`'s structure and change the compute shader's math.
5. **OBJ import.** The `tobj` crate parses `.obj` files into vertex/index
   buffers you can hand straight to wgpu.
6. **Dithering.** A final post-process pass sampling a small Bayer
   matrix texture (or hardcoded 4x4/8x8 threshold matrix in the shader)
   against a posterized/quantized color — cheap, and it's most of the
   "arcade" look.
7. **The node graph UI.** Once the render side is solid, add `egui` +
   `egui-wgpu` for an immediate-mode UI overlay, and use the
   `egui_node_graph` crate (or its successor `egui_node_graph2`) which
   already implements a draggable node-and-wire editor. The `Graph`
   struct in `src/graph.rs` is already shaped for this — the missing
   piece is calling `add_node`/rewiring `inputs` in response to UI
   events at runtime instead of only at startup, plus a topological
   sort if you let people wire things in arbitrary order.

I'd build step 5 (OBJ import) or the dithering pass (step 6) next —
both are self-contained and quick relative to what you just got
working. Dithering is probably the faster win if you want the arcade
look sooner; OBJ import is the bigger unlock if you want to bring in
actual 3D geometry (mountains, terrain meshes) to react to audio too.
