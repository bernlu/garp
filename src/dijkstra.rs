use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::{
    graph::{BaseNode, CHDirection, CHEdge, CHGraph, EdgeId, NodeId},
    paths::CHEdgeList,
};

pub struct Dijkstra<'a, N, E> {
    frontier_fwd: BinaryHeap<HeapElement>, // frontier, cost, and prev node data for the forward and backward search
    cost_fwd: Vec<Option<u32>>,            // structures are reused in each run
    prev_fwd: Vec<Option<(NodeId, EdgeId)>>,
    frontier_bwd: BinaryHeap<HeapElement>,
    cost_bwd: Vec<Option<u32>>,
    prev_bwd: Vec<Option<(NodeId, EdgeId)>>,
    graph: &'a dyn CHGraph<Node = N, Edge = E>,
    visited: Vec<NodeId>, // tracks which nodes were visited for faster resetting
}

// entry for the heap / frontier set / yet-to-visit nodes
#[derive(Debug)]
struct HeapElement {
    cost: u32,
    id: NodeId,                     // id of the node stored here
    prev: Option<(NodeId, EdgeId)>, // the previous node and the edge (prev, this) to this entry
}

// inverse ordering to create a min heap (BinaryHeap is a maxheap)
impl Ord for HeapElement {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.cost.cmp(&other.cost) {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => self.id.cmp(&other.id).reverse(),
        }
    }
}
impl PartialOrd for HeapElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Eq for HeapElement {}
impl PartialEq for HeapElement {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl<'a, N, E> Dijkstra<'a, N, E>
where
    N: BaseNode,
    E: CHEdge,
{
    pub fn new(graph: &'a dyn CHGraph<Node = N, Edge = E>) -> Self {
        Self {
            cost_fwd: vec![None; graph.num_nodes()],
            frontier_fwd: BinaryHeap::new(),
            prev_fwd: vec![None; graph.num_nodes()],
            cost_bwd: vec![None; graph.num_nodes()],
            frontier_bwd: BinaryHeap::new(),
            prev_bwd: vec![None; graph.num_nodes()],
            graph: graph.into(),
            visited: Vec::new(),
        }
    }

    /// resets all stored data for the next run
    fn reset(&mut self) {
        self.frontier_fwd.clear();
        self.frontier_bwd.clear();

        for &visited in &self.visited {
            self.cost_fwd[visited] = None;
            self.cost_bwd[visited] = None;
            self.prev_fwd[visited] = None;
            self.prev_bwd[visited] = None;
        }
        self.visited.clear();
    }
    /// dijkstra search for CH graphs
    /// returns: (distance, edges) or None if there is no path
    pub fn ch_search(&mut self, start: NodeId, dest: NodeId) -> Option<(u32, CHEdgeList)> {
        self.ch_search_multi([start], [dest])
    }
    
    /// dijkstra search for CH graphs. 
    /// this function takes multiple start and destination points, but will only return one shortest path
    /// that is the one which is smallest of all start-dest pairs
    pub fn ch_search_multi<I, J>(&mut self, start: I, dest: J) -> Option<(u32, CHEdgeList)>
    where
        I: IntoIterator<Item = NodeId>,
        J: IntoIterator<Item = NodeId>,
    {
        // clear structures
        self.reset();

        // initialize fwd search with start node
        for s in start {
            self.frontier_fwd.push(HeapElement {
                id: s,
                cost: 0,
                prev: None,
            });
        }

        // initialize backward search with dest node
        for d in dest {
            self.frontier_bwd.push(HeapElement {
                id: d,
                cost: 0,
                prev: None,
            });
        }

        let mut peak_candidate: Option<NodeId> = None;
        let mut candidate_cost = u32::MAX;

        loop {
            // track which node was settled in this step
            let settled = match (self.frontier_fwd.peek(), self.frontier_bwd.peek()) {
                // balance both queues by prioritizing the one with smaller next value
                (Some(fwd), Some(bwd)) if fwd <= bwd  => self.ch_fwd_step(),
                (Some(_), Some(_)) /* if fwd > bwd */ => self.ch_bwd_step(),
                (Some(_), None)                       => self.ch_fwd_step(),
                (None, Some(_))                       => self.ch_bwd_step(),
                (None, None) => {
                    // both searches done. find Result
                    let peak = peak_candidate?;

                    // build fwd search path
                    let mut path = Vec::new();
                    let mut next_id = peak;
                    while let Some(prev) = self.prev_fwd[next_id] {
                        path.push(prev.1);
                        next_id = prev.0;
                    } // stops when start is reached
                    path.reverse();
                    // now: path = [(start, p1), (p1, p2), ...,(pk,peak)]

                    next_id = peak;
                    // build bwd search path
                    while let Some(prev) = self.prev_bwd[next_id] {
                        path.push(prev.1);
                        next_id = prev.0;
                    } // stops when dest is reached

                    // now: path = [(start,p1), ..., (pk,peak), (peak,pl) ..., (pm,dest)]

                    return Some((candidate_cost, CHEdgeList::new(path)));
                }
            };

            // early stopping logic

            if let Some(settled) = settled {
                // check if settled node is settled in both directions - candidate update
                if let (Some(cost_start), Some(cost_dest)) =
                    (self.cost_fwd[settled], self.cost_bwd[settled])
                {
                    match peak_candidate {
                        None => {
                            peak_candidate = Some(settled);
                            candidate_cost = cost_start + cost_dest;
                        }
                        Some(_) => {
                            if cost_start + cost_dest < candidate_cost {
                                peak_candidate = Some(settled);
                                candidate_cost = cost_start + cost_dest;
                            }
                        }
                    }
                }
            }
            // if the next node in a frontier set has cost > cost(start|dest, peak_candidate), stop search in that direction
            if let Some(p) = self.frontier_fwd.peek() {
                if p.cost > candidate_cost {
                    self.frontier_fwd.clear();
                }
            }
            if let Some(p) = self.frontier_bwd.peek() {
                if p.cost > candidate_cost {
                    self.frontier_bwd.clear();
                }
            }
        }
    }

    /// runs a step in the forward search
    fn ch_fwd_step(&mut self) -> Option<NodeId> {
        let entry = self.frontier_fwd.pop().unwrap();
        // double insertion check (taken from the benchmark repository code:
        //    https://github.com/Lesstat/dijkstra-performance-study/blob/748e8be73df80cda170674c85cbd8777de3b207e/src/dijkstra.rs#L68 )
        if self.cost_fwd[entry.id].is_some() {
            return None; // entry is old: if self.cost is filled, the node has been settled.
                         // entry.cost will always be greater than the cost already saved to self.cost because we have a min heap.
        }

        // settle
        self.cost_fwd[entry.id] = Some(entry.cost);
        self.prev_fwd[entry.id] = entry.prev;
        self.visited.push(entry.id);

        // stall on demand check
        let mut stall = false;
        for &in_edge in self.graph.in_edges(entry.id, CHDirection::Down) {
            let neighbor_id = self.graph.edge(in_edge).source();
            if let Some(d) = self.cost_fwd[neighbor_id] {
                if d + self.graph.edge(in_edge).cost() < entry.cost {
                    stall = true;
                }
            }
        }

        // if we do not stall on this node, add neighbors into the heap
        if !stall {
            // insert new entries into heap
            // only consider edges (entry, neighbor) with level(neighbor) > level(entry).
            for &edge in self.graph.out_edges(entry.id, CHDirection::Up) {
                let neighbor = self.graph.node(self.graph.edge(edge).target());
                if self.cost_fwd[neighbor.id()].is_none() {
                    // e.target not settled -> add to heap
                    self.frontier_fwd.push(HeapElement {
                        id: neighbor.id(),
                        cost: entry.cost + self.graph.edge(edge).cost(),
                        prev: Some((entry.id, edge)),
                    });
                }
            }
        }
        Some(entry.id)
    }

    fn ch_bwd_step(&mut self) -> Option<NodeId> {
        let entry = self.frontier_bwd.pop().unwrap();
        // double insertion check (taken from the benchmark repository code:
        //    https://github.com/Lesstat/dijkstra-performance-study/blob/748e8be73df80cda170674c85cbd8777de3b207e/src/dijkstra.rs#L68 )
        if self.cost_bwd[entry.id].is_some() {
            return None; // entry is old: if self.cost is filled, the node has been settled.
                         // entry.cost will always be greater than the cost already saved to self.cost because we have a min heap.
        }

        // settle
        self.cost_bwd[entry.id] = Some(entry.cost);
        self.prev_bwd[entry.id] = entry.prev;
        self.visited.push(entry.id);

        // stall on demand check
        let mut stall = false;
        for &out_edge in self.graph.out_edges(entry.id, CHDirection::Up) {
            let neighbor_id = self.graph.edge(out_edge).target();
            if let Some(d) = self.cost_bwd[neighbor_id] {
                if d + self.graph.edge(out_edge).cost() < entry.cost {
                    stall = true;
                }
            }
        }

        if !stall {
            // insert new entries into heap
            // only consider edges (entry, neighbor) with level(neighbor) < level(entry).
            for &edge in self.graph.in_edges(entry.id, CHDirection::Down) {
                let neighbor = self.graph.node(self.graph.edge(edge).source());
                if self.cost_bwd[neighbor.id()].is_none() {
                    // e.target not settled -> add to heap
                    self.frontier_bwd.push(HeapElement {
                        id: neighbor.id(),
                        cost: entry.cost + self.graph.edge(edge).cost(),
                        prev: Some((entry.id, edge)),
                    });
                }
            }
        }
        Some(entry.id)
    }
}
