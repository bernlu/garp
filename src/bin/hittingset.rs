use std::fs::File;
use std::io::{BufWriter, Write};

use chext::graph::NodeId;
use chext::graph::{BaseNode, CHEdge, CHGraph};
use chext::hittingset::HittingSet;
use chext::paths::CHEdgeList;
use chext::{load_ch_graph, load_hs_graph, load_paths};
use clap::{App, Arg};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::time::Instant;

struct Args<'a> {
    /// .fmi graph file
    graph_file: &'a str,

    /// file to store the hittingset
    out_file: &'a str,

    /// list of files containing paths
    paths_files: Vec<&'a str>,

    /// disables hitting set verification
    skip_verification: bool,

    verbose: bool,

    /// limits the maximum iterations that the hitting set algorithm will run for
    maxiter: Option<usize>,
}

/// takes a graph and one or more paths files and generates a hitting set
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new(
        "Hittingset - takes a graph and one or more paths files and calculates a hitting set",
    )
    .arg(
        Arg::with_name("graph file")
            .short("g")
            .help(".fmi ch graph file")
            .takes_value(true)
            .value_name("*.fmi")
            .required(true),
    )
    .arg(
        Arg::with_name("out file")
            .short("o")
            .help("file to store result")
            .takes_value(true)
            .required(true),
    )
    .arg(
        Arg::with_name("paths files")
            .short("p")
            .help("paths files")
            .takes_value(true)
            .multiple(true)
            .required(true),
    )
    .arg(
        Arg::with_name("skip verification")
            .long("skip_verification")
            .help("skip verification"),
    )
    .arg(
        Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .help("output progress"),
    )
    .arg(
        Arg::with_name("maxiter")
            .short("m")
            .long("max-iterations")
            .help("limits the maximum iterations that the hitting set algorithm will run for")
            .takes_value(true),
    )
    .get_matches();

    let args = Args {
        graph_file: matches.value_of("graph file").unwrap(),
        out_file: matches.value_of("out file").unwrap(),
        paths_files: matches
            .values_of("paths files")
            .unwrap()
            .into_iter()
            .collect(),
        skip_verification: matches.is_present("skip verification"),
        verbose: matches.is_present("verbose"),
        maxiter: matches.value_of("maxiter").and_then(|d| d.parse().ok()),
    };

    // load paths
    let paths = {
        if args.verbose {
            println!("loading path files");
        }
        let mut paths = Vec::new();
        for file in &args.paths_files {
            let mut p = load_paths(file)?;
            paths.append(&mut p);
        }
        paths
    };
    if args.verbose {
        println!("number of paths: {}", paths.len());
    }
    if args.verbose {
        println!(
            "sum of path weights: {}",
            paths.par_iter().map(|p| p.weight).sum::<u64>()
        );
    }

    // calculate hitting set & lower bound if required
    let hittingset = {
        if args.verbose {
            println!("loading graph");
        }
        let hsgraph = load_hs_graph(&args.graph_file)?;
        let hs_calc = HittingSet::new(&hsgraph, paths);

        if args.verbose {
            println!("calculating hitting set");
        }
        let now = Instant::now();
        let hittingset = hs_calc.run_with_stats_maxiter(false, args.maxiter);
        let duration = now.elapsed();
        if args.verbose {
            println!("hitting set found. duration: {:?}", duration);
        }

        hittingset
    };

    if args.verbose {
        println!("hs size: {}", hittingset.len());
    }

    // check hittingset
    if !args.skip_verification {
        if args.verbose {
            println!("checking hitting set");
        }
        let check = {
            let g = load_ch_graph(&args.graph_file)?;
            let mut paths = Vec::new();
            for file in &args.paths_files {
                let mut p = load_paths(file)?;
                paths.append(&mut p);
            }
            check_hitting_set_par(&hittingset, &paths, &g)
        };
        assert!(check, "hittingset not correct");
    }

    // store hitting set to file
    let file = File::create(args.out_file)?;
    let mut buf = BufWriter::new(file);
    for h in hittingset {
        buf.write(format!("{}\n", String::from(h)).as_bytes())
            .unwrap();
    }

    Ok(())
}

/// checks the hitting set by expanding each path and checking if one of the nodes is in the hitting set
/// parallel with rayon
fn check_hitting_set_par<N: BaseNode, E: CHEdge>(
    hittingset: &Vec<NodeId>,
    paths: &Vec<CHEdgeList>,
    graph: &dyn CHGraph<Node = N, Edge = E>,
) -> bool {
    paths
        .par_iter()
        .map(|path| {
            // map a path to true if there is a node in the hittingset that hits this path, otherwise map to false
            // unpack ch path to full path
            let full_path = graph.unpack_ch_edges(path);
            // turn edge-path into node-path
            let mut node_path = Vec::with_capacity(full_path.0.len() + 1);
            node_path.push(graph.edge(full_path.0[0]).source());
            for e in full_path {
                node_path.push(graph.edge(e).target());
            }
            // check if a node of the hittingset is on the path
            for node in hittingset {
                if node_path.contains(node) {
                    return true;
                }
            }
            return false;
        })
        .all(|t| t) // returns true if all map results are true => all paths are hit by the set
}
