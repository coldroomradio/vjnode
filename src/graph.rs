/// Anything that can be a node in the render graph. A node reads zero or
/// more input textures and writes to one output texture — that's the
/// entire contract. Effects (invert, blend, bloom...) read 1-2 inputs;
/// sources (color, noise, video, particles) read 0.
pub trait GraphNode {
    /// Short label for debug/UI display — e.g. "Color Source", "Invert".
    fn name(&self) -> &str;

    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        inputs: &[&wgpu::TextureView],
        output: &wgpu::TextureView,
    );
}

struct NodeSlot {
    node: Box<dyn GraphNode>,
    inputs: Vec<usize>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Owns every node's offscreen output texture and runs them in order.
///
/// Current limitation, on purpose for this scaffold: nodes execute in
/// insertion order, and `inputs` must reference nodes added earlier
/// (so it's a DAG by construction, not just convention — you can't
/// accidentally create a cycle). This is exactly what a topological
/// sort produces, so once you add a real editable graph (egui_node_graph
/// or similar), the piece you'll need is: whenever the user changes a
/// connection, topologically sort the node set and call add_node in
/// that order — everything below already handles the rest.
pub struct Graph {
    nodes: Vec<NodeSlot>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
}

impl Graph {
    pub fn new(format: wgpu::TextureFormat, size: (u32, u32)) -> Self {
        Self {
            nodes: Vec::new(),
            format,
            size,
        }
    }

    /// Adds a node. `inputs` are indices returned by earlier `add_node`
    /// calls — those nodes' output textures become this node's inputs,
    /// in the order given.
    pub fn add_node(
        &mut self,
        device: &wgpu::Device,
        node: Box<dyn GraphNode>,
        inputs: Vec<usize>,
    ) -> usize {
        let (texture, view) = Self::make_target(device, self.format, self.size);
        self.nodes.push(NodeSlot {
            node,
            inputs,
            texture,
            view,
        });
        self.nodes.len() - 1
    }

    fn make_target(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("graph node output texture"),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }

    /// Recreates every node's output texture at a new size (call this from
    /// your window resize handler).
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        self.size = size;
        for slot in &mut self.nodes {
            let (texture, view) = Self::make_target(device, self.format, size);
            slot.texture = texture;
            slot.view = view;
        }
    }

    /// Names of every node in insertion (execution) order — the graph as
    /// it would appear listed top-to-bottom in a UI.
    pub fn node_names(&self) -> Vec<&str> {
        self.nodes.iter().map(|slot| slot.node.name()).collect()
    }

    /// Runs every node in insertion order. The last node added writes
    /// straight to `final_output` (the screen) instead of its own
    /// offscreen texture — everything upstream of it renders into its
    /// own texture first.
    pub fn execute(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        final_output: &wgpu::TextureView,
    ) {
        let Some(last) = self.nodes.len().checked_sub(1) else {
            return;
        };

        for i in 0..self.nodes.len() {
            let slot = &self.nodes[i];
            let input_views: Vec<&wgpu::TextureView> = slot
                .inputs
                .iter()
                .map(|&idx| &self.nodes[idx].view)
                .collect();
            let output_view = if i == last { final_output } else { &slot.view };

            slot.node
                .render(device, queue, encoder, &input_views, output_view);
        }
    }
}
