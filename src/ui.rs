use winit::event::WindowEvent;
use winit::window::Window;

use crate::audio::AudioBands;

/// Read-only debug overlay — no editing, no dragging, just visibility
/// into what the graph is doing. This is deliberately small; the real
/// draggable node editor (egui_node_graph) is a separate, later step
/// once there's more than a couple node types worth arranging.
pub struct Ui {
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl Ui {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, window: &Window) -> Self {
        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let state = egui_winit::State::new(context.clone(), viewport_id, window, None, None);
        let renderer = egui_wgpu::Renderer::new(device, surface_format, None, 1);

        Self {
            context,
            state,
            renderer,
        }
    }

    /// Feed a window event to egui. Returns true if egui consumed it
    /// (e.g. the mouse was over a UI panel) — not used yet since nothing
    /// in the 3D/graph view responds to input, but this is where you'd
    /// check it later to avoid e.g. rotating a camera while dragging a
    /// UI slider.
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        target: &wgpu::TextureView,
        screen_size: (u32, u32),
        node_names: &[String],
        bands: AudioBands,
    ) {
        let raw_input = self.state.take_egui_input(window);

        let full_output = self.context.run(raw_input, |ctx| {
            egui::Window::new("vjnode — graph").show(ctx, |ui| {
                ui.label("Chain (executes top to bottom):");
                ui.separator();
                for (i, name) in node_names.iter().enumerate() {
                    ui.label(format!("{i}. {name}"));
                }
                ui.separator();
                ui.label("Audio bands:");
                ui.add(egui::ProgressBar::new(bands.bass).text("bass"));
                ui.add(egui::ProgressBar::new(bands.mid).text("mid"));
                ui.add(egui::ProgressBar::new(bands.treble).text("treble"));
                ui.add(egui::ProgressBar::new(bands.rms).text("rms"));
            });
        });

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let clipped_primitives = self
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [screen_size.0, screen_size.1],
            pixels_per_point: full_output.pixels_per_point,
        };

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            // Loads the existing frame content (don't clear — the graph's
            // output is already in `target`) and draws UI on top of it.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.renderer
                .render(&mut pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
    }
}
