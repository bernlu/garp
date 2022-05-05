use std::time::Instant;

use garp::{
    graph::BaseGraph,
    load_ch_graph,
    quadtree::{QuadTree, TreeNode},
    wspd::Tree,
};
use clap::{App, Arg};

struct Args<'a> {
    /// .fmi graph file
    graph_file: &'a str,

    /// min tree depth
    mindepth: usize,

    /// max tree depth
    maxdepth: usize,
}

/// calculates some information on quadtrees of different depths
fn main() {
    let matches = App::new("Quadtree analysis - calculates some information on quadtrees of different depths")
    .arg(Arg::with_name("graph file")
        .help(".fmi graph file")
        .takes_value(true)
        .value_name("*.fmi")
        .required(true)
        .index(1))
    .arg(Arg::with_name("min tree depth")
        .help("smallest value for max tree depth")
        .takes_value(true)
        .required(true)
        .index(2))
    .arg(Arg::with_name("max tree depth")
        .help("largest value for max tree depth")
        .takes_value(true)
        .required(true)
        .index(3))
    .get_matches();

    let args = Args {
        graph_file: matches.value_of("graph file").unwrap(),
        maxdepth: matches
            .value_of("max tree depth")
            .and_then(|d| d.parse().ok())
            .expect("no valid max depth value provided"),
        mindepth: matches
            .value_of("min tree depth")
            .and_then(|d| d.parse().ok())
            .expect("no valid min depth value provided"),
    };

    let graph = load_ch_graph(args.graph_file).unwrap();

    let tree_data: Vec<&dyn TreeNode> = graph
        .iter_nodes()
        .map(|n| {
            let tn: &dyn TreeNode = n;
            tn
        })
        .collect();

    println!("maxdepth, duration, #leafs, #nodes, mean leaf size, leafs with one point");
    for k in args.mindepth..args.maxdepth + 1 {
        let now = Instant::now();
        let quadtree = QuadTree::new(tree_data.clone(), k);
        let duration = now.elapsed();

        let leafcount = count_leafs(&quadtree);
        let mean_leaf_size = graph.num_nodes() as f64 / leafcount as f64;

        let one_count_leafs = count_one_point_leafs(&quadtree);

        // maxdepth | duration | #leafs | #nodes | mean leaf size | leafs with one point
        println!(
            "{}, {:?}, {}, {}, {}, {}",
            k,
            duration,
            leafcount,
            graph.num_nodes(),
            mean_leaf_size,
            one_count_leafs
        );
    }
}

/// counts the number of leafs in a tree
fn count_leafs(tree: &QuadTree) -> usize {
    let children: Vec<&QuadTree> = tree.children().collect();
    if children.len() == 0 {
        return 1;
    }
    children.iter().map(|node| count_leafs(*node)).sum()
}

/// counts the number of leafs with exactly one point in a tree
fn count_one_point_leafs(tree: &QuadTree) -> usize {
    let children: Vec<&QuadTree> = tree.children().collect();
    let treepoints: Vec<&&dyn TreeNode> = tree.points().collect();
    if children.len() == 0 && treepoints.len() == 1 {
        return 1;
    }
    children
        .iter()
        .map(|node| count_one_point_leafs(*node))
        .sum()
}
