use std::{iter::repeat, time::Instant};

use chext::{
    dijkstra::Dijkstra,
    graph::{BaseGraph, BaseNode, CHEdge, CHGraph, NodeId},
    load_ch_graph,
    quadtree::{QuadTree, TreeNode},
    wspd::WSPD,
};
use clap::{App, Arg};
use rand::prelude::*;
use rayon::iter::{ParallelBridge, ParallelIterator};
use rustc_hash::FxHashMap;

struct Args<'a> {
    /// .fmi graph file
    graph_file: &'a str,

    /// max tree depth. defaults to usize::MAX
    maxdepth: usize,

    /// epsilon parameter for the WSPD
    epsilon: f64,

    geom_check_percent: Option<f64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("WSPD analysis - takes a graph and calculates a well separated pair decomposition based on a quad tree. outputs data for analysis")
        .arg(Arg::with_name("graph file")
            .short("g")
            .help(".fmi graph file")
            .takes_value(true)
            .value_name("*.fmi")
            .required(true))
        .arg(Arg::with_name("max tree depth")
            .short("d")
            .help("maximum depth of the quad tree")
            .takes_value(true))
        .arg(Arg::with_name("epsilon")
            .short("e")
            .help("epsilon for wspd calculation")
            .default_value("0.5")
            .takes_value(true))
        .arg(Arg::with_name("geom check")
            .long("geom_check")
            .takes_value(true))
        .get_matches();

    let args = Args {
        graph_file: matches.value_of("graph file").unwrap(),
        maxdepth: matches
            .value_of("max tree depth")
            .and_then(|d| d.parse().ok())
            .unwrap_or(usize::MAX),
        epsilon: matches
            .value_of("epsilon")
            .unwrap()
            .parse()
            .expect("no valid epsilon provided"),
        geom_check_percent: matches.value_of("geom check").and_then(|d| d.parse().ok()),
    };

    println!("running with d={} and e={}", args.maxdepth, args.epsilon);

    let graph = load_ch_graph(args.graph_file)?;

    let now = Instant::now();
    // 1. generate quad tree
    let tree_data = graph
        .iter_nodes()
        .map(|n| {
            let tn: &dyn TreeNode = n;
            tn
        })
        .collect();
    let quadtree = QuadTree::new(tree_data, args.maxdepth);

    let duration = now.elapsed();
    println!("tree constructed. duration: {:?}", duration);

    let now = Instant::now();
    // 2. calculate wspd
    let wspd = WSPD::new(&quadtree, args.epsilon);

    let duration = now.elapsed();

    println!("wspd done. duration: {:?}", duration);
    println!("wspd size: {}", wspd.len());

    // 3. verification code
    let now = Instant::now();

    // point covering error
    let wspd_point_pairs_count = wspd
        .iter()
        .par_bridge()
        .map(|(u, v)| u.size() * v.size())
        .sum::<usize>();
    let all_pairs_count =
        (0.5 * graph.num_nodes() as f64 * (graph.num_nodes() - 1) as f64) as usize;
    println!(
        "#pairs/#potential pairs: {}/{} (Covering Error: {:.3}%)",
        wspd_point_pairs_count,
        all_pairs_count,
        (1.0 - wspd_point_pairs_count as f64 / all_pairs_count as f64) * 100.
    );

    // point covering per cell size
    let mut point_pairs_counts = FxHashMap::default();
    for (u, v) in wspd.iter() {
        let pair_level = pair_level(u, v);
        *point_pairs_counts.entry(pair_level).or_insert(0) += u.size() * v.size();
    }
    println!("#pairs per depth: {:#?}", point_pairs_counts);

    if let Some(check_percent) = args.geom_check_percent {
        println!("geometric error checking");
        geometric_error_check(&graph, &wspd, check_percent);
    }
    println!("verification time: {:?}", now.elapsed());

    // print cell size statistics
    let mut hist = vec![0; args.maxdepth + 1];

    for (u, v) in wspd.iter() {
        hist[pair_level(u, v)] += 1;
    }

    println!("pair depth histogram: {:#?}", hist);

    Ok(())
}

fn pair_level(u: &QuadTree, v: &QuadTree) -> usize {
    u.id.len().max(v.id.len())
}

fn geometric_error_check<N: BaseNode, E: CHEdge>(
    graph: &dyn CHGraph<Node = N, Edge = E>,
    wspd: &WSPD<QuadTree>,
    check_percent: f64,
) {
    // check for each pair a few paths for intersections

    let mut rng = StdRng::seed_from_u64(42);
    let sample = wspd
        .iter()
        .choose_multiple(&mut rng, (wspd.len() as f64 * check_percent) as usize);

    // (pair_depth, weight)
    let violating_count_weighted: Vec<(usize, usize)> = sample
        .iter()
        .par_bridge()
        .map_init(
            || Dijkstra::new(graph),
            |dijkstra, (u, v)| {
                let pair_depth = pair_level(u, v);

                let u_points = u.points();
                let v_points = v.points();

                let mut rng = rand::thread_rng();

                let u_samples = u_points.choose_multiple(&mut rng, 3);
                let v_samples = v_points.choose_multiple(&mut rng, 3);

                let paths: Vec<Vec<NodeId>> = u_samples
                    .iter()
                    .flat_map(|u| repeat(u).zip(v_samples.iter()))
                    .filter_map(|(us, vs)| dijkstra.ch_search(us.id(), vs.id()))
                    .map(|(_dist, path)| path)
                    .map(|path| graph.unpack_ch_edges(&path))
                    .map(|edgelist| {
                        let mut unpacked: Vec<NodeId> = edgelist
                            .iter()
                            .map(|&edge| graph.edge(edge).source())
                            .collect();
                        unpacked.push(graph.edge(*edgelist.last().unwrap()).target());
                        unpacked
                    })
                    .collect();

                let num_paths = paths.len();

                if num_paths == 0 {
                    return None; // no paths found => there is no geometric error
                }

                // check for intersections by counting the occ of nodes.
                let mut histogram = FxHashMap::default();
                for node in paths.iter().flatten() {
                    *histogram.entry(node).or_insert(0) += 1;
                }

                for (_, &occ) in &histogram {
                    if occ == num_paths {
                        return None; // a node is visited by each path => no geometric error
                    }
                }

                // there is a geom error => return the depth and weight of this cell
                Some((pair_depth, u.size() * v.size()))
            },
        )
        .filter_map(|res| res)
        .collect(); //.sum(); //filter(|x| x.is_some()).map(|x| x.unwrap()).collect();

    let mut hist = FxHashMap::default();

    for (depth, weight) in violating_count_weighted {
        *hist.entry(depth).or_insert(0) += weight;
    }

    println!("geometric error hist: {:#?}", hist);
}
