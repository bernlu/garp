use serde::{Deserialize, Serialize};

use super::{BaseEdge, BaseGraph, BaseNode, EdgeId, GeoGraph, GeoNode, NodeId, StoreableGraph};

#[derive(Deserialize, Serialize, Debug)]
pub struct FMINode {
    pub id: usize,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub level: u32,
}

impl BaseNode for FMINode {
    fn id(&self) -> NodeId {
        self.id.into()
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct FMIEdge {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub cost: u32,
    pub child1: i32,
    pub child2: i32,
}

#[derive(Serialize, Deserialize)]
pub struct FMIGraph {
    pub nodes: Vec<FMINode>,
    pub edges: Vec<FMIEdge>,
}

impl GeoNode for FMINode {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

impl BaseEdge for FMIEdge {
    fn source(&self) -> NodeId {
        self.source.into()
    }
    fn target(&self) -> NodeId {
        self.target.into()
    }
}

impl BaseGraph for FMIGraph {
    type Node = FMINode;
    type Edge = FMIEdge;
    fn size(&self) {
        println!("usize: {}", std::mem::size_of::<usize>());
        println!(
            "#Nodes: {}\t Node: {}\t all: {}",
            self.nodes.len(),
            std::mem::size_of::<FMINode>(),
            std::mem::size_of_val(&*self.nodes)
        );
        println!(
            "#Edges: {}\t Edge: {}\t all: {}",
            self.edges.len(),
            std::mem::size_of::<FMIEdge>(),
            std::mem::size_of_val(&*self.edges)
        );

        println!(
            "full size: {}",
            std::mem::size_of_val(&*self.nodes) + std::mem::size_of_val(&*self.edges)
        );
    }

    fn edge(&self, id: EdgeId) -> &Self::Edge {
        &self.edges[id]
    }
    fn node(&self, id: NodeId) -> &Self::Node {
        &self.nodes[id]
    }
    fn iter_nodes(&self) -> std::slice::Iter<'_, Self::Node> {
        self.nodes.iter()
    }
    fn iter_edges(&self) -> std::slice::Iter<'_, Self::Edge> {
        self.edges.iter()
    }
    fn num_edges(&self) -> usize {
        self.edges.len()
    }
    fn num_nodes(&self) -> usize {
        self.nodes.len()
    }
}

impl GeoGraph for FMIGraph {
    fn node(&self, id: NodeId) -> &dyn GeoNode {
        &self.nodes[id]
    }
}

impl StoreableGraph for FMIGraph {
    fn from_file_binary(filename: &str) -> Result<Self, bincode::Error> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        bincode::deserialize_from(reader)
    }
}
