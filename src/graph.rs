mod chgraph;
mod fmigraph;
mod hsgraph;

use chgraph::Edge as CH_Edge;
use chgraph::Node as CH_Node;
use hsgraph::Edge as HS_Edge;
use hsgraph::Node as HS_Node;

use std::{fs::File, io::BufWriter};

use bincode::serialize_into;
use derive_index::Index;
use derive_more::{Add, From, Sub};
use serde::{Deserialize, Serialize};

use crate::paths::{CHEdgeList, EdgeList};

pub use chgraph::AdjArrayGraph;
pub use fmigraph::{FMIEdge, FMIGraph, FMINode};
pub use hsgraph::ToporderedGraph;

// define some types for use in derive and index macros
type NodeEdgePair = (NodeId, EdgeId);
type VecUsize = Vec<usize>;
type VecEdgeId = Vec<EdgeId>;
type Hittingsettype = (usize, EdgeId);

#[derive(
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Add,
    Sub,
    From,
    Index,
    Hash,
)]
#[index_type(
    NodeId,
    u64,
    u32,
    usize,
    NodeEdgePair,
    VecUsize,
    CH_Node,
    HS_Node,
    FMINode,
    bool
)]
pub struct NodeId(usize);

impl From<NodeId> for String {
    fn from(n: NodeId) -> String {
        n.0.to_string()
    }
}

#[derive(
    Copy,
    Clone,
    Serialize,
    Deserialize,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Add,
    Sub,
    From,
    Index,
    Hash,
)]
#[index_type(
    u32,
    u64,
    VecUsize,
    VecEdgeId,
    bool,
    Hittingsettype,
    usize,
    CH_Edge,
    HS_Edge,
    FMIEdge
)]
pub struct EdgeId(usize);

/// the CHDirection describes the direction of an edge in the CH graph
/// this enum is used to match specific edges of the graph, to create G_up and G_down
/// an edge (u,v) is
/// Up => level(u) < level(v) => in G_up
/// Down => level(u) > level(v) => in G_down
/// Both => either => G
#[derive(Copy, Clone, Debug)]
pub enum CHDirection {
    Up,
    Down,
    Both,
}

// node traits

pub trait BaseNode {
    fn id(&self) -> NodeId;
}

pub trait GeoNode {
    fn lat(&self) -> f64;
    fn lon(&self) -> f64;
}

/// provides a parents() function to traverse the metagraph
pub trait HSNode {
    fn parents(&self) -> &[EdgeId];
}

pub trait CHNode {
    fn level(&self) -> u32;
}

// edge traits

pub trait BaseEdge {
    fn source(&self) -> NodeId;
    fn target(&self) -> NodeId;
}

pub trait CostEdge {
    fn cost(&self) -> u32;
}

/// provides childrenX() functions to traverse the metagraph or chgraph
pub trait ChildEdge {
    fn child1(&self) -> Option<EdgeId>;
    fn child2(&self) -> Option<EdgeId>;
}

pub trait CHEdge: BaseEdge + CostEdge + ChildEdge {}

/// provides a parents() function to traverse the metagraph
pub trait HSEdge {
    fn parents(&self) -> &[EdgeId];
    fn id(&self) -> EdgeId;
}

// graph traits

pub trait BaseGraph: Sync + Send {
    type Node: BaseNode;
    type Edge: BaseEdge;
    fn edge(&self, id: EdgeId) -> &Self::Edge;
    fn node(&self, id: NodeId) -> &Self::Node;
    fn size(&self);
    fn iter_edges(&self) -> std::slice::Iter<'_, Self::Edge>;
    fn iter_nodes(&self) -> std::slice::Iter<'_, Self::Node>;
    fn num_nodes(&self) -> usize;
    fn num_edges(&self) -> usize;
}

/// this trait is used to store a graph as binary file for faster loading
pub trait StoreableGraph {
    fn to_file_binary(&self, filename: &str) -> Result<(), bincode::Error>
    where
        Self: Serialize,
    {
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        serialize_into(&mut writer, self)
    }
    fn from_file_binary(filename: &str) -> Result<Self, bincode::Error>
    where
        Self: Sized;
}

pub trait CHGraph: BaseGraph {
    fn unpack_ch_edges(&self, path: &CHEdgeList) -> EdgeList;
    /// returns outdoing edges of a node, using CHDirection to filter the result and return either G, G_up, or G_down
    fn out_edges(&self, node_id: NodeId, ch_direction: CHDirection) -> &[EdgeId];
    /// returns incoming edges of a node, using CHDirection to filter the result and return either G, G_up, or G_down
    fn in_edges(&self, node_id: NodeId, ch_direction: CHDirection) -> &[EdgeId];
}

pub trait HSGraph: BaseGraph {
    fn toporder(&self, edge_id: EdgeId) -> usize;
    fn iter_edges_topordered(&self) -> std::slice::Iter<Self::Edge>;
}

pub trait GeoGraph: BaseGraph {
    fn node(&self, id: NodeId) -> &dyn GeoNode;
}
