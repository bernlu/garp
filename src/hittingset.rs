use std::{cmp::Ordering, collections::BinaryHeap, time::Instant};

use rustc_hash::FxHashSet;

use crate::{
    graph::{BaseEdge, BaseNode, ChildEdge, EdgeId, HSEdge, HSGraph, HSNode, NodeId},
    paths::CHEdgeList,
};

pub struct HittingSet<'a, N, E> {
    hist: Vec<u64>, // current node hist. hist[nodeId] = #occurences
    graph: &'a dyn HSGraph<Node = N, Edge = E>,
    edge_path_map: Vec<Vec<usize>>, // maps edgeId -> all pathIds where the edge is on the path
    paths: Vec<CHEdgeList>,
    adaptive_threshold: usize, // this threshold decides if a full scan or an explorative scan is used for a iteration
}

impl<'a, N: HSNode + BaseNode, E: HSEdge + BaseEdge + ChildEdge> HittingSet<'a, N, E> {
    pub fn new_with_threshold(
        graph: &'a dyn HSGraph<Node = N, Edge = E>,
        paths: Vec<CHEdgeList>,
        adaptive_threshold: usize,
    ) -> Self {
        // build the edge path map: scan all paths and save them for each edge
        let mut edge_path_map: Vec<Vec<usize>> = vec![Vec::new(); graph.num_edges()];
        for (id, path) in paths.iter().enumerate() {
            for &edge in path {
                edge_path_map[edge].push(id);
            }
        }

        Self {
            hist: vec![0; graph.num_nodes()],
            graph: graph.into(),
            edge_path_map,
            paths,
            adaptive_threshold,
        }
    }

    pub fn new(graph: &'a dyn HSGraph<Node = N, Edge = E>, paths: Vec<CHEdgeList>) -> Self {
        Self::new_with_threshold(graph, paths, 400000)
    }

    /// finds all paths that intersect path_id. will ignore paths that are tagged as false in the paths_todo list.
    fn intersecting_paths(&self, path_id: usize, paths_todo: &Vec<bool>) -> FxHashSet<usize> {
        let mut res = FxHashSet::default();

        // add all edges on the path to the up and down queue.
        // and tag them as visited
        let mut up_visited = vec![false; self.graph.num_edges()];
        let mut down_visited = vec![false; self.graph.num_edges()];
        let mut up_queue = Vec::new();
        let mut down_queue = Vec::new();
        for &e in &self.paths[path_id] {
            if !up_visited[e] {
                up_queue.push(e);
                up_visited[e] = true;
            }
            if !down_visited[e] {
                down_queue.push(e);
                down_visited[e] = true;
            }
        }

        // first, handle the down queue
        while let Some(edge) = down_queue.pop() {
            // any paths in the edge path map for this edge intersect the input path.
            for &path in &self.edge_path_map[edge] {
                if paths_todo[path] {
                    res.insert(path);
                }
            }

            // we are handling the down queue. add all children of this edge to the up and down queue.
            if let (Some(c1), Some(c2)) = (
                self.graph.edge(edge).child1(),
                self.graph.edge(edge).child2(),
            ) {
                if !down_visited[c1] {
                    down_queue.push(c1);
                    down_visited[c1] = true;
                }
                if !down_visited[c2] {
                    down_queue.push(c2);
                    down_visited[c2] = true;
                }
                if !up_visited[c1] {
                    up_queue.push(c1);
                    up_visited[c1] = true;
                }
                if !up_visited[c2] {
                    up_queue.push(c2);
                    up_visited[c2] = true;
                }
            } else {
                // edge is base edge
                let source = self.graph.edge(edge).source();
                let target = self.graph.edge(edge).target();
                for &parent in self.graph.node(source).parents() {
                    if parent != edge {
                        if !up_visited[parent] {
                            up_queue.push(parent);
                            up_visited[parent] = true;
                        }
                    }
                }
                for &parent in self.graph.node(target).parents() {
                    if parent != edge {
                        if !up_visited[parent] {
                            up_queue.push(parent);
                            up_visited[parent] = true;
                        }
                    }
                }
            }
        }

        // handle up queue
        while let Some(edge) = up_queue.pop() {
            // any paths in the edge path map for this edge intersect the input path.
            for &path in &self.edge_path_map[edge] {
                if paths_todo[path] {
                    res.insert(path);
                }
            }
            // add parents to the up queue.
            for &parent in self.graph.edge(edge).parents() {
                if !up_visited[parent] {
                    up_queue.push(parent);
                    up_visited[parent] = true;
                }
            }
        }
        res
    }

    /// finds a lower bound for the hitting set size
    pub fn lower_bound(&self) -> usize {
        let mut lower = 0;

        let mut paths_todo: Vec<bool> = self.paths.iter().map(|p| p.len() > 0).collect();

        for (id, _) in self.paths.iter().enumerate() {
            if paths_todo[id] {
                // for a path, find all intersecting paths
                let intersecting = self.intersecting_paths(id, &paths_todo);
                // tag all of them
                for entry in intersecting {
                    paths_todo[entry] = false;
                }
                // count once
                lower += 1;
            }
        }
        lower
    }

    /// calculates the hitting set.
    pub fn run(self) -> Vec<(NodeId, u64)> {
        self.run_with_stats_maxiter(false, None)
    }

    /// calculates the hitting set.
    /// print_stats: outputs information on each iteration
    /// maxiter: stop after reaching given number of iterations
    pub fn run_with_stats_maxiter(
        mut self,
        print_stats: bool,
        maxiter: Option<usize>,
    ) -> Vec<(NodeId, u64)> {
        // preparation: do one full scan
        self.scan_edges_full(false);

        let mut hittingset = Vec::new();
        let mut num_paths = self.paths.len();
        let mut iteration = 0;

        if print_stats {
            println!("iteration, iteration time, #hit paths, #paths left, weighted #hit paths");
        }
        loop {
            iteration += 1;

            // early stopping check
            if let Some(mi) = maxiter {
                if iteration > mi {
                    return hittingset;
                }
            }

            let now = Instant::now();
            // find hitter
            let (max_id, &max_occ) = self
                .hist
                .iter()
                .enumerate()
                .max_by(|(_, v), (_, w)| v.cmp(w))
                .unwrap();
            let hitter = max_id.into();
            if max_occ == 0 {
                // no paths left - stop
                break;
            }


            let mut removed = Vec::new();

            // find & remove paths that were hit (dont delete entries from the list to keep indices stable)
            for &i in &self.hit_paths(hitter) {
                let p: &mut CHEdgeList = &mut self.paths[i];
                if p.len() == 0 {
                    continue;
                }
                removed.push(p.clone());
                p.clear();
            }
            num_paths -= removed.len();

            // add to set
            hittingset.push((hitter, removed.iter().map(|r| r.weight).sum::<u64>()));

            // if there is an input of size < adaptive_threshold, run an explorative scan. otherwise full scan.
            if removed.len() < self.adaptive_threshold || num_paths < self.adaptive_threshold {
                if removed.len() < num_paths {
                    self.scan_edges_explore(Some(&removed), true);
                } else {
                    self.scan_edges_explore(None, false);
                }
            } else {
                self.scan_edges_full(false);
            }

            // print stats
            if print_stats {
                // println!("iteration, iteration time, #hit paths, #paths left, weighted #hit paths");
                println!(
                    "{}, {:?}, {}, {}, {}",
                    iteration,                                     // iteration
                    now.elapsed(),                                 // iteration time
                    removed.len(),                                 // #hit paths
                    num_paths,                                     // #paths left
                    removed.iter().map(|r| r.weight).sum::<u64>(), // weighted #hit paths
                );
            }
        }

        hittingset
    }

    /// returns all paths that contain a node.
    fn hit_paths(&self, hitter: NodeId) -> FxHashSet<usize> {
        // traverse the DAG and check every edge for path parents.
        let mut hit_paths = FxHashSet::default();

        let mut queue: Vec<EdgeId> = self.graph.node(hitter).parents().iter().cloned().collect();

        while let Some(edge) = queue.pop() {
            for &path in &self.edge_path_map[edge] {
                hit_paths.insert(path);
            }
            for &parent in self.graph.edge(edge).parents() {
                queue.push(parent);
            }
        }

        hit_paths
    }

    /// runs a full scan of all paths, updating the histogram.
    /// update == true => update old hist (new = old - this)
    /// update == false => create new hist (new = this)
    fn scan_edges_full(&mut self, update: bool) {
        if !update {
            // reset hist
            self.hist = vec![0; self.hist.len()];
        }
        let mut edges_hist: Vec<u64> = vec![0; self.graph.num_edges()];
        // 1. count edges
        for path in &self.paths {
            // we will count the target node of each edge. this will skip the source node of each path -> add them here
            if let Some(&first_edge) = path.first() {
                let source = self.graph.edge(first_edge).source();

                if update {
                    self.hist[source] -= path.weight; // sub because we want to update old data
                } else {
                    self.hist[source] += path.weight;
                }
            }
            for &edge in path {
                edges_hist[edge] += path.weight;
            }
        }

        // 2. replace ch-edges with children, use topological order
        for edge in self.graph.iter_edges_topordered() {
            if let (Some(c1), Some(c2)) = (edge.child1(), edge.child2()) {
                // move counts to child edges
                edges_hist[c1] += edges_hist[edge.id()];
                edges_hist[c2] += edges_hist[edge.id()];
            } else {
                // this is a base graph edge. count the target node
                let target = edge.target();
                if update {
                    self.hist[target] -= edges_hist[edge.id()]; // sub because we want to update old data
                } else {
                    self.hist[target] += edges_hist[edge.id()];
                }
            }
        }
    }

    /// runs an explorative scan, updating the histogram.
    /// update == true => update old hist
    /// update == false => create new hist
    /// removed_paths: if Some, scan these paths. if None, scan self.paths
    fn scan_edges_explore(&mut self, removed_paths: Option<&Vec<CHEdgeList>>, update: bool) {
        let paths = match removed_paths {
            Some(p) => p,
            None => &self.paths,
        };

        if !update {
            // reset hist
            self.hist = vec![0; self.hist.len()];
        }
        let mut edges_hist: Vec<u64> = vec![0; self.graph.num_edges()];

        // 1. count edges
        for path in paths {
            // we will count the target node of each edge. this will skip the source node of each path -> add them here
            if let Some(&first_edge) = path.first() {
                let source = self.graph.edge(first_edge).source();

                if update {
                    self.hist[source] -= path.weight; // sub because we want to update old data
                } else {
                    self.hist[source] += path.weight;
                }
            }
            for &edge in path {
                edges_hist[edge] += path.weight;
            }
        }

        // 2. replace ch-edges with children, sorted by node level. (uses a binary heap for sorting)

        // collect set of all edges in given paths
        let mut unique_edges: Vec<EdgeId> = paths.iter().flatten().cloned().collect();
        unique_edges.sort();
        unique_edges.dedup();

        // create initial heap, use unique edge set to prevent duplicate entries
        let mut pq: BinaryHeap<CHEdgeHeapElement> = unique_edges
            .into_iter()
            .map(|e| CHEdgeHeapElement {
                edge: e,
                prio: self.graph.toporder(e),
            })
            .collect();

        // iterate heap and update histogram, fill node histogram
        while let Some(CHEdgeHeapElement { edge, .. }) = pq.pop() {
            if let (Some(c1), Some(c2)) = (
                self.graph.edge(edge).child1(),
                self.graph.edge(edge).child2(),
            ) {
                // add children to pq

                // children will only be added once by this code block because an edge cannot be the child of two different edges.
                // but: c1 or c2 (or both) may be contained in the initial pq.
                // we can check if this is the case by looking at the histogram first:
                // edges_hist[cx] != 0 <=> there is a ch-path containing cx <=> cx is in the initial pq (exactly once by construction)
                // => do not push into pq in that case

                if edges_hist[c1] == 0 {
                    pq.push(CHEdgeHeapElement {
                        edge: c1,
                        prio: self.graph.toporder(c1),
                    });
                }
                if edges_hist[c2] == 0 {
                    pq.push(CHEdgeHeapElement {
                        edge: c2,
                        prio: self.graph.toporder(c2),
                    });
                }

                // move counts to child edges
                edges_hist[c1] += edges_hist[edge];
                edges_hist[c2] += edges_hist[edge];
            } else {
                // this is a base graph edge. count the target node
                let target = self.graph.edge(edge).target();
                if update {
                    self.hist[target] -= edges_hist[edge]; // sub because we want to update old data
                } else {
                    self.hist[target] += edges_hist[edge];
                }
            }
        }
    }
}

#[derive(Eq, PartialEq)]
struct CHEdgeHeapElement {
    prio: usize,
    edge: EdgeId,
}

impl Ord for CHEdgeHeapElement {
    fn cmp(&self, other: &Self) -> Ordering {
        self.prio
            .cmp(&other.prio)
            .reverse()
            .then(self.edge.cmp(&other.edge))
    }
}
impl PartialOrd for CHEdgeHeapElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
