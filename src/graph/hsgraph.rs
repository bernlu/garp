use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

use super::{
    fmigraph::{FMIEdge, FMIGraph, FMINode},
    BaseEdge, BaseGraph, BaseNode, ChildEdge, EdgeId, HSEdge, HSGraph, HSNode, NodeId,
    StoreableGraph,
};

// define Node, Edge, Graph for this HSGraph

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub parents: Vec<EdgeId>,
}

#[derive(Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub child1: Option<EdgeId>,
    pub child2: Option<EdgeId>,
    pub parents: Vec<EdgeId>, // for DAG of edge replacements
}

#[derive(Serialize, Deserialize)]
pub struct ToporderedGraph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,        // sorted by toporder
    edge_id_map: Vec<usize>, // map: EdgeId -> index into edges
}

// impl traits

impl BaseNode for Node {
    fn id(&self) -> NodeId {
        self.id
    }
}

impl HSNode for Node {
    fn parents(&self) -> &[EdgeId] {
        &self.parents
    }
}

impl From<FMINode> for Node {
    fn from(n: FMINode) -> Self {
        Self {
            id: n.id.into(),
            parents: Vec::new(),
        }
    }
}

impl BaseEdge for Edge {
    fn source(&self) -> NodeId {
        self.source
    }
    fn target(&self) -> NodeId {
        self.target
    }
}

impl HSEdge for Edge {
    fn parents(&self) -> &[EdgeId] {
        &self.parents
    }
    fn id(&self) -> EdgeId {
        self.id
    }
}

impl ChildEdge for Edge {
    fn child1(&self) -> Option<EdgeId> {
        self.child1
    }
    fn child2(&self) -> Option<EdgeId> {
        self.child2
    }
}

impl From<FMIEdge> for Edge {
    fn from(e: FMIEdge) -> Self {
        Self {
            id: e.id.into(),
            source: e.source.into(),
            target: e.target.into(),
            child1: match usize::try_from(e.child1) {
                Ok(c) => Some(c.into()),
                Err(_) => None,
            },
            child2: match usize::try_from(e.child2) {
                Ok(c) => Some(c.into()),
                Err(_) => None,
            },
            parents: Vec::new(),
        }
    }
}

impl From<FMIGraph> for ToporderedGraph {
    /// takes a FMIGraph and creates a ToporderedGraph (a graph that implements HSGraph and stores its edges in topological order w.r.t. the metagraph)
    fn from(fmi: FMIGraph) -> Self {
        let FMIGraph { nodes, edges } = fmi;
        // convert nodes and edges
        let mut nodes: Vec<Node> = nodes.into_iter().map(Into::into).collect();
        let edges: Vec<Edge> = edges.into_iter().map(Into::into).collect();

        // make sure that nodes are sorted, nodes[a].id == a
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        // add an index to all edges before reordering
        let mut edges_with_index: Vec<(usize, Edge)> = edges.into_iter().enumerate().collect();

        // calculate parents data for each edge
        let mut parents: Vec<Vec<EdgeId>> = vec![Vec::new(); edges_with_index.len()];

        for (edgeid, edge) in &edges_with_index {
            let edgeid: EdgeId = (*edgeid).into();
            if let (Some(c1), Some(c2)) = (edge.child1, edge.child2) {
                // edge is ch edge. add edge to children's parents list
                parents[c1].push(edgeid);
                parents[c2].push(edgeid);
            } else {
                // edge is base edge. add edge to its nodes parents list
                nodes[edge.source].parents.push(edgeid);
                nodes[edge.target].parents.push(edgeid);
            }
        }

        // store parents in the edge struct
        for (id, parent) in parents.into_iter().enumerate() {
            edges_with_index[id].1.parents = parent;
        }

        // topsort edges array & calculate edge_id_map
        let topsorted = edge_top_sort(&edges_with_index);
        let mut edge_id_map = vec![0; topsorted.len()]; // edge_id_map[edge_id] = index in edges array
        for (index, &edge_id) in topsorted.iter().enumerate() {
            edge_id_map[edge_id] = index;
        }

        // sort edges by topsort order
        edges_with_index.sort_by(|(ida, _), (idb, _)| {
            let index_a = edge_id_map[*ida];
            let index_b = edge_id_map[*idb];
            index_a.cmp(&index_b)
        });

        // sanity check
        for (index, (edge_id, _)) in edges_with_index.iter().enumerate() {
            let mapped_index = edge_id_map[*edge_id];
            assert_eq!(index, mapped_index);
        }

        Self {
            nodes: nodes,
            edge_id_map,
            edges: edges_with_index.into_iter().map(|(_, e)| e).collect(),
        }
    }
}

/// sorts edges in topological order
/// impl taken from https://www.geeksforgeeks.org/topological-sorting/
fn edge_top_sort(edges_with_index: &Vec<(usize, Edge)>) -> Vec<EdgeId> {
    let mut visited = vec![false; edges_with_index.len()];
    let mut res = Vec::new();

    for (id, _) in edges_with_index {
        if !visited[*id] {
            let mut sub_res = top_sort_rec((*id).into(), &mut visited, edges_with_index);
            res.append(&mut sub_res);
        }
    }
    res.reverse();
    res
}

/// internal function for topological sorting - see edge_top_sort
fn top_sort_rec(
    id: EdgeId,
    mut visited: &mut Vec<bool>,
    edges_with_index: &Vec<(usize, Edge)>,
) -> Vec<EdgeId> {
    visited[id] = true;
    let mut res = Vec::new();

    let (index, edge) = &edges_with_index[id.0];
    assert_eq!(*index, id.0);
    if let (Some(c1), Some(c2)) = (edge.child1, edge.child2) {
        if !visited[c1] {
            res.append(&mut top_sort_rec(c1, &mut visited, edges_with_index));
        }
        if !visited[c2] {
            res.append(&mut top_sort_rec(c2, &mut visited, edges_with_index));
        }
    }

    res.push(id);
    res
}

impl BaseGraph for ToporderedGraph {
    type Node = Node;
    type Edge = Edge;
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
            "full size: {}",
            std::mem::size_of_val(&*self.nodes)
                + std::mem::size_of_val(&*self.edges)
                + std::mem::size_of_val(&*self.edge_id_map)
        );
    }

    fn edge(&self, id: EdgeId) -> &Self::Edge {
        &self.edges[self.edge_id_map[id]]
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

impl HSGraph for ToporderedGraph {
    fn toporder(&self, edge_id: EdgeId) -> usize {
        self.edge_id_map[edge_id]
    }
    fn iter_edges_topordered(&self) -> std::slice::Iter<Self::Edge> {
        self.edges.iter()
    }
}

impl StoreableGraph for ToporderedGraph {
    fn from_file_binary(filename: &str) -> Result<Self, bincode::Error> {
        let file = std::fs::File::open(filename)?;
        let reader = std::io::BufReader::new(file);
        bincode::deserialize_from(reader)
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::HSGraph, load_hs_graph};

    use super::ToporderedGraph;

    fn load_toy_hs() -> ToporderedGraph {
        load_hs_graph("../toy_ch.fmi").unwrap()
    }

    #[test]
    fn topsorted_toy() {
        let g = load_toy_hs();

        for edge in g.iter_edges_topordered() {
            let e_topidx = g.toporder(edge.id);
            if let (Some(c1), Some(c2)) = (edge.child1, edge.child2) {
                let c1_topidx = g.toporder(c1);
                let c2_topidx = g.toporder(c2);
                assert!(e_topidx < c1_topidx);
                assert!(e_topidx < c2_topidx);
            }
        }
    }
}
