use serde::{Deserialize, Serialize};

use std::convert::TryFrom;

use super::{
    fmigraph::{FMIEdge, FMIGraph, FMINode},
    BaseEdge, BaseGraph, BaseNode, CHDirection, CHEdge, CHGraph, CHNode, ChildEdge, CostEdge,
    EdgeId, NodeId, StoreableGraph,
};
use crate::{
    graph::{GeoGraph, GeoNode},
    paths::{CHEdgeList, EdgeList},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    pub id: NodeId,
    pub lat: f64,
    pub lon: f64,
    pub level: u32,
}

impl CHNode for Node {
    fn level(&self) -> u32 {
        self.level
    }
}

impl BaseNode for Node {
    fn id(&self) -> NodeId {
        self.id
    }
}

impl GeoNode for Node {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

impl From<FMINode> for Node {
    fn from(n: FMINode) -> Self {
        Self {
            id: n.id.into(),
            level: n.level,
            lat: n.lat,
            lon: n.lon,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub cost: u32,
    pub child1: Option<EdgeId>,
    pub child2: Option<EdgeId>,
}

impl BaseEdge for Edge {
    fn source(&self) -> NodeId {
        self.source
    }
    fn target(&self) -> NodeId {
        self.target
    }
}

impl ChildEdge for Edge {
    fn child1(&self) -> Option<EdgeId> {
        self.child1
    }
    fn child2(&self) -> Option<EdgeId> {
        self.child1
    }
}

impl CostEdge for Edge {
    fn cost(&self) -> u32 {
        self.cost
    }
}

impl CHEdge for Edge {}

impl From<FMIEdge> for Edge {
    fn from(e: FMIEdge) -> Self {
        Self {
            source: e.source.into(),
            target: e.target.into(),
            cost: e.cost,
            child1: match usize::try_from(e.child1) {
                Ok(c) => Some(c.into()),
                Err(_) => None,
            },
            child2: match usize::try_from(e.child2) {
                Ok(c) => Some(c.into()),
                Err(_) => None,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AdjArrayGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,        // sorted by id
    offsets_out: Vec<usize>, // maps nodeId -> index_out: offsets_out[nodeId] = first edge in edges_out with e=(nodeId,*)
    offsets_in: Vec<usize>, // maps nodeId -> index_in:  offsets_in[nodeId] = first edge in edges_in with e=(*,nodeId)
    edges_out: Vec<EdgeId>, // stores edges, sorted by source node. [(0,*), (0,*), (0,*), (1,*), ..., (n,*)]
    edges_in: Vec<EdgeId>, // stores edges, sorted by target node. [(*,0), (*,0), (*,0), (*,1), ..., (*,n)]
}

impl From<FMIGraph> for AdjArrayGraph {
    /// turn FMIGraph into AdjArrayGraph (implements CHGraph and GeoGraph)
    fn from(fmi: FMIGraph) -> Self {
        let FMIGraph { nodes, edges } = fmi;
        let mut nodes: Vec<Node> = nodes.into_iter().map(Into::into).collect();
        let edges: Vec<Edge> = edges.into_iter().map(Into::into).collect();

        // make sure that nodes are sorted, nodes[a].id == a
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        let mut edges_with_index: Vec<(usize, Edge)> = edges.into_iter().enumerate().collect();

        // sort edges by target node. [(*,0), (*,0), (*,0), (*,1), ..., (*,n)]
        // where for a target node i: [(a,i), (b,i), ..., (k,i), (l,i), ..., (m,i)] s.t. level(a..k) < level(i) && level(i) < level(l..m)
        edges_with_index.sort_by(|(_, a), (_, b)| {
            a.target
                .cmp(&b.target)
                .then(nodes[a.source].level.cmp(&nodes[b.source].level))
        });

        // create offsets arrays

        // maps nodeId -> index_in:  offsets_in[nodeId] = first edge in edges_in with e=(*,nodeId)
        let mut offsets_in = vec![0; nodes.len() + 1];
        for (_, e) in &edges_with_index {
            offsets_in[e.target + 1.into()] += 1;
        }
        for i in 1..offsets_in.len() {
            offsets_in[i] += offsets_in[i - 1];
        }

        let edges_in = edges_with_index.iter().map(|&(id, _)| id.into()).collect();

        // stores edges, sorted by source node. [(0,*), (0,*), (0,*), (1,*), ..., (n,*)]
        edges_with_index.sort_by(|(_, a), (_, b)| {
            a.source
                .cmp(&b.source)
                .then(nodes[a.target].level.cmp(&nodes[b.target].level))
        });

        // maps nodeId -> index_out: offsets_out[nodeId] = first edge in edges_out with e=(nodeId,*)
        let mut offsets_out = vec![0; nodes.len() + 1];
        for (_, e) in &edges_with_index {
            offsets_out[e.source + 1.into()] += 1;
        }
        for i in 1..offsets_out.len() {
            offsets_out[i] += offsets_out[i - 1];
        }

        let edges_out = edges_with_index.iter().map(|&(id, _)| id.into()).collect();

        // sort edges based on id for efficient access
        edges_with_index.sort_by(|(ida, _), (idb, _)| ida.cmp(&idb));

        Self {
            nodes: nodes,
            edges: edges_with_index.into_iter().map(|(_, e)| e).collect(),
            offsets_out: offsets_out,
            offsets_in: offsets_in,
            edges_out: edges_out,
            edges_in: edges_in,
        }
    }
}

impl GeoGraph for AdjArrayGraph {
    fn node(&self, id: NodeId) -> &dyn GeoNode {
        &self.nodes[id]
    }
}

impl BaseGraph for AdjArrayGraph {
    type Node = Node;
    type Edge = Edge;
    fn edge(&self, id: EdgeId) -> &Self::Edge {
        &self.edges[id]
    }
    fn iter_edges(&self) -> std::slice::Iter<Self::Edge> {
        self.edges.iter()
    }
    fn node(&self, id: NodeId) -> &Self::Node {
        &self.nodes[id]
    }
    fn iter_nodes(&self) -> std::slice::Iter<Self::Node> {
        self.nodes.iter()
    }
    fn num_edges(&self) -> usize {
        self.edges.len()
    }
    fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    fn size(&self) {
        println!("usize: {}", std::mem::size_of::<usize>());
        println!(
            "#Nodes: {}\t Node: {}\t all: {}",
            self.nodes.len(),
            std::mem::size_of::<Node>(),
            std::mem::size_of_val(&*self.nodes)
        );
        println!(
            "#Edges: {}\t Edge: {}\t all: {}",
            self.edges.len(),
            std::mem::size_of::<Edge>(),
            std::mem::size_of_val(&*self.edges)
        );
        println!(
            "#offset: {}\t Edge: {}\t all: {}",
            self.offsets_in.len(),
            std::mem::size_of::<usize>(),
            std::mem::size_of_val(&*self.offsets_in)
        );
        println!(
            "#edges: {}\t Edge: {}\t all: {}",
            self.edges_in.len(),
            std::mem::size_of::<EdgeId>(),
            std::mem::size_of_val(&*self.edges_in)
        );

        println!(
            "full size: {}",
            std::mem::size_of_val(&*self.nodes)
                + std::mem::size_of_val(&*self.edges)
                + std::mem::size_of_val(&*self.edges_in)
                + std::mem::size_of_val(&*self.edges_out)
                + std::mem::size_of_val(&*self.offsets_in)
                + std::mem::size_of_val(&*self.offsets_out)
        );
    }
}

impl CHGraph for AdjArrayGraph {
    fn unpack_ch_edges(&self, path: &CHEdgeList) -> EdgeList {
        let mut unpacked = Vec::new();
        for &e in path {
            unpacked.append(&mut self.unpack_edge(e));
        }
        EdgeList(unpacked)
    }

    fn out_edges(&self, node_id: NodeId, ch_direction: CHDirection) -> &[EdgeId] {
        // we know that the edges are sorted by their target level
        // level(e[0]) < level(e[1]) ... < level(e[m])
        let edges =
            &self.edges_out[self.offsets_out[node_id]..self.offsets_out[node_id + 1.into()]];
        let this_level = self.node(node_id).level;
        // find first edge where target level is larger than this node's level
        let k = edges.partition_point(|&e| self.node(self.edge(e).target).level < this_level);

        let down = &edges[..k];
        let up = &edges[k..];

        // for i<k edges are down edges, for i>=k edges are up edges. sanity check:
        for &e in up {
            let e = self.edge(e);
            let source = self.node(e.source);
            let target = self.node(e.target);
            assert!(source.level < target.level, "correctness of up edges");
        }
        for &e in down {
            let e = self.edge(e);
            let source = self.node(e.source);
            let target = self.node(e.target);
            assert!(source.level > target.level, "correctness of down edges");
        }

        // return requested edges
        match ch_direction {
            CHDirection::Both => {
                &self.edges_out[self.offsets_out[node_id]..self.offsets_out[node_id + 1.into()]]
            }
            CHDirection::Up => up,
            CHDirection::Down => down,
        }
    }

    fn in_edges(&self, node_id: NodeId, ch_direction: CHDirection) -> &[EdgeId] {
        let edges = &self.edges_in[self.offsets_in[node_id]..self.offsets_in[node_id + 1.into()]];
        let this_level = self.node(node_id).level;
        let k = edges.partition_point(|&e| self.node(self.edge(e).source).level < this_level);

        // for i<k edges are down edges, for i>=k edges are up edges
        let up = &edges[..k];
        let down = &edges[k..];

        for &e in up {
            let e = self.edge(e);
            let source = self.node(e.source);
            let target = self.node(e.target);
            assert!(source.level < target.level, "correctness of up edges");
        }
        for &e in down {
            let e = self.edge(e);
            let source = self.node(e.source);
            let target = self.node(e.target);
            assert!(source.level > target.level, "correctness of down edges");
        }

        match ch_direction {
            CHDirection::Both => edges,
            CHDirection::Up => up,
            CHDirection::Down => down,
        }
    }
}

impl AdjArrayGraph {
    fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    // utility function to unpack a ch edge into base edges
    fn unpack_edge(&self, edge: EdgeId) -> Vec<EdgeId> {
        let e = self.edge(edge);
        match (e.child1, e.child2) {
            (Some(c1), Some(c2)) => {
                let mut res = self.unpack_edge(c1);
                res.append(&mut self.unpack_edge(c2));
                res
            }
            _ => vec![edge],
        }
    }
}

impl StoreableGraph for AdjArrayGraph {
    fn from_file_binary(filename: &str) -> Result<Self, bincode::Error> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        bincode::deserialize_from(reader)
    }
}
