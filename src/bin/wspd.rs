use std::io::{stdout, Write};

use garp::{
    dijkstra::Dijkstra,
    graph::{BaseGraph, BaseNode, CHEdge, CHGraph},
    load_ch_graph,
    quadtree::{QuadTree, TreeNode},
    wspd::WSPD,
    PathWriter,
};
use clap::{App, Arg};
use rayon::iter::{ParallelBridge, ParallelIterator};

struct Args<'a> {
    /// .fmi graph file
    graph_file: &'a str,

    /// file to store the paths
    out_file: &'a str,

    /// max tree depth. defaults to usize::MAX
    maxdepth: usize,

    /// epsilon parameter for the WSPD
    epsilon: f64,

    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("WSPD - takes a graph and calculates a well separated pair decomposition based on a quad tree. outputs weighted paths for each pair")
        .arg(Arg::with_name("graph file")
            .short("g")
            .help(".fmi graph file")
            .takes_value(true)
            .value_name("*.fmi")
            .required(true))
        .arg(Arg::with_name("out file")
            .short("o")
            .help("file to store paths")
            .takes_value(true)
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
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .help("output progress"))
        .get_matches();

    let args = Args {
        graph_file: matches.value_of("graph file").unwrap(),
        out_file: matches.value_of("out file").unwrap(),
        maxdepth: matches
            .value_of("max tree depth")
            .and_then(|d| d.parse().ok())
            .unwrap_or(usize::MAX),
        epsilon: matches
            .value_of("epsilon")
            .unwrap()
            .parse()
            .expect("no valid epsilon provided"),
        verbose: matches.is_present("verbose"),
    };

    println!("running with d={} and e={}", args.maxdepth, args.epsilon);

    let graph = load_ch_graph(args.graph_file)?;

    // 1. generate quad tree
    let tree_data = graph
        .iter_nodes()
        .map(|n| {
            let tn: &dyn TreeNode = n;
            tn
        })
        .collect();
    let quadtree = QuadTree::new(tree_data, args.maxdepth);

    // 2. calculate wspd
    let wspd = WSPD::new(&quadtree, args.epsilon);

    if args.verbose {
        println!("wspd done. size: {}", wspd.len());
    }

    // 3. iterate pairs, pick a repr point from both sets and find a shortest path. store to file.
    sample_path_and_store_par(&graph, &wspd, args.out_file.to_string(), args.verbose);

    Ok(())
}

fn sample_path_and_store_par<N: BaseNode, E: CHEdge>(
    graph: &dyn CHGraph<Node = N, Edge = E>,
    wspd: &WSPD<QuadTree>,
    filename: String,
    verbose: bool,
) {
    // create channel to send all results to a writer thread
    let (send, recv) = std::sync::mpsc::sync_channel(rayon::current_num_threads());

    // Spawn a thread that is dedicated to writing results
    let writer_thread = std::thread::spawn(move || {
        let mut i = 0;
        if verbose {
            print!("paths generated: {}", i);
        }
        let mut wtr = PathWriter::new(&filename, false);
        for path in recv {
            wtr.save_path(path);
            i += 1;
            if i % 10000 == 0 && verbose {
                print!("\rpaths generated: {}", i);
                stdout().flush().unwrap();
            }
        }
        println!();
    });

    if verbose {
        println!("starting path generation");
    }

    // iterate the wspd and calculate a path for each pair.
    wspd.iter().par_bridge().for_each_init(
        || Dijkstra::new(graph),
        |dijkstra, (u, v)| {
            let u_nodes = u.points().map(|p| p.id());
            let v_nodes = v.points().map(|p| p.id());

            if let Some((_dist, mut path)) = dijkstra.ch_search_multi(u_nodes, v_nodes) {
                path.weight = (u.size() * v.size()) as u64; // set weight to #point pairs in the wspd pair
                send.send(path).expect("error sending data");
            }
        },
    );

    drop(send); // ! without this line the receiver will never stop waiting for more data

    if let Err(e) = writer_thread.join() {
        eprintln!("Unable to join internal thread: {:?}", e);
    }
}
