use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audio::AudioBands;

pub type NodeId = Uuid;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum NodeKind {
    ColorSource,
    Invert,
    Particles,
    Add,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeModel {
    pub id: NodeId,
    pub kind: NodeKind,
    pub pos: egui::Pos2,
    /// ordered inputs: each entry is the source node id for that input slot
    pub inputs: Vec<NodeId>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct GraphModel {
    pub nodes: Vec<NodeModel>,
}

impl GraphModel {
    pub fn find(&self, id: &NodeId) -> Option<&NodeModel> {
        self.nodes.iter().find(|n| &n.id == id)
    }
    pub fn find_mut(&mut self, id: &NodeId) -> Option<&mut NodeModel> {
        self.nodes.iter_mut().find(|n| &n.id == id)
    }
}

/// Returns a topologically sorted list of NodeIds or an Err("cycle")
pub fn topological_sort(model: &GraphModel) -> Result<Vec<NodeId>, &'static str> {
    // Build adjacency and indegree maps
    let mut indeg: HashMap<NodeId, usize> = HashMap::new();
    let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for n in &model.nodes {
        indeg.insert(n.id, 0);
        adj.insert(n.id, Vec::new());
    }

    for n in &model.nodes {
        for inp in &n.inputs {
            // incoming edge from inp -> n.id
            if let Some(entry) = indeg.get_mut(&n.id) {
                *entry += 1;
            }
            if let Some(vec) = adj.get_mut(inp) {
                vec.push(n.id);
            }
        }
    }

    let mut q: VecDeque<NodeId> = indeg
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(id, _)| *id)
        .collect();

    let mut out = Vec::new();
    while let Some(nid) = q.pop_front() {
        out.push(nid);
        if let Some(neigh) = adj.get(&nid) {
            for &m in neigh {
                if let Some(d) = indeg.get_mut(&m) {
                    *d -= 1;
                    if *d == 0 {
                        q.push_back(m);
                    }
                }
            }
        }
    }

    if out.len() == model.nodes.len() {
        Ok(out)
    } else {
        Err("cycle")
    }
}

/// Build a runtime Graph (crate::graph::Graph) from the editable GraphModel.
/// This creates new GraphNodes for each model node in topological order.
pub fn build_runtime_graph(
    model: &GraphModel,
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    audio_bands: Arc<Mutex<AudioBands>>,
) -> Result<crate::graph::Graph, String> {
    let order = topological_sort(model).map_err(|_| "cycle in graph model")?;
    let mut id_to_index: HashMap<NodeId, usize> = HashMap::new();
    let mut graph = crate::graph::Graph::new(format, size);

    for id in order {
        let node_model = model
            .find(&id)
            .ok_or_else(|| format!("node id {} not found", id))?;

        // map input ids to indices
        let mut input_indices: Vec<usize> = Vec::new();
        for inp_id in &node_model.inputs {
            if let Some(idx) = id_to_index.get(inp_id) {
                input_indices.push(*idx);
            } else {
                return Err(format!("input id {} not found/earlier", inp_id));
            }
        }

        let boxed: Box<dyn crate::graph::GraphNode> = match node_model.kind {
            NodeKind::ColorSource => Box::new(crate::nodes::ColorSourceNode::new(
                device,
                format,
                audio_bands.clone(),
            )),
            NodeKind::Invert => Box::new(crate::nodes::InvertNode::new(device, format)),
            NodeKind::Particles => Box::new(crate::nodes::ParticleNode::new(
                device,
                format,
                audio_bands.clone(),
            )),
            NodeKind::Add => Box::new(crate::nodes::AddNode::new(device, format)),
        };

        let idx = graph.add_node(device, boxed, input_indices);
        id_to_index.insert(id, idx);
    }

    Ok(graph)
}
