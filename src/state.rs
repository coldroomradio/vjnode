use std::sync::{Arc, Mutex};
use winit::window::Window;

use crate::audio::AudioBands;
use crate::graph::Graph;
use crate::nodes::{AddNode, ColorSourceNode, InvertNode, ParticleNode};
use crate::ui::Ui;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,

    graph: Graph,
    ui: Ui,
    audio_bands: Arc<Mutex<AudioBands>>,
}

impl State {
    pub async fn new(window: Arc<Window>, audio_bands: Arc<Mutex<AudioBands>>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY, // uses Metal on macOS
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("vjnode device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // ---------- Build the graph ----------
        // This is the whole "node system" right now: add_node returns an
        // index, later nodes reference earlier indices as their inputs.
        // Adding a third effect is just one more add_node call with the
        // previous node's index as its input.
        let mut graph = Graph::new(surface_format, (config.width, config.height));

        let source_idx = graph.add_node(
            &device,
            Box::new(ColorSourceNode::new(
                &device,
                surface_format,
                audio_bands.clone(),
            )),
            vec![],
        );
        let inverted_idx = graph.add_node(
            &device,
            Box::new(InvertNode::new(&device, surface_format)),
            vec![source_idx],
        );
        let particles_idx = graph.add_node(
            &device,
            Box::new(ParticleNode::new(
                &device,
                surface_format,
                audio_bands.clone(),
            )),
            vec![],
        );
        // Add is the last node added, so it's the one that writes
        // straight to the screen — see Graph::execute.
        graph.add_node(
            &device,
            Box::new(AddNode::new(&device, surface_format)),
            vec![inverted_idx, particles_idx],
        );

        let ui = Ui::new(&device, surface_format, &window);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            graph,
            ui,
            audio_bands,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.graph
                .resize(&self.device, (new_size.width, new_size.height));
        }
    }

    /// Forwards a window event to the UI (mouse/keyboard/etc). Returns
    /// true if egui consumed it.
    pub fn handle_window_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.ui.handle_event(&self.window, event)
    }

    pub fn update(&mut self) {
        // Nodes now manage their own time/audio state internally (see
        // ColorSourceNode), so there's nothing global to update here yet.
        // This stays as the hook point for anything graph-wide later —
        // e.g. a global "master intensity" the UI controls.
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        self.graph.execute(&self.device, &self.queue, &mut encoder, &view);

        let node_names: Vec<String> = self
            .graph
            .node_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let bands = self.audio_bands.lock().map(|b| *b).unwrap_or_default();

        self.ui.render(
            &self.device,
            &self.queue,
            &mut encoder,
            &self.window,
            &view,
            (self.config.width, self.config.height),
            &node_names,
            bands,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
