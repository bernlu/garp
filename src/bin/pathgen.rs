use garp::graph::{BaseNode, CHEdge, CHGraph};
use clap::{App, Arg};
use std::io::{stdout, Write};

use garp::dijkstra::Dijkstra;
use garp::paths::SourceTargetPair;
use garp::random_pairs::STPGenerator;
use garp::{load_ch_graph, PathWriter};
use rayon::iter::{ParallelBridge, ParallelIterator};

// struct to store command line args.
//with newer versions of rust, clap can auto fill this, but the rustc on ubuntu (at the time of writing) is too old
struct Args<'a> {
    n: usize,
    seed: Option<u64>,
    graph_file: &'a str,
    out_file: &'a str,
    parallel: bool,
    verbose: bool,
}

/// this binary generates random point pairs and calculates the shortest path.
/// results are stored in a text file where each line is one path in ch-edge representation
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // build cli
    let matches = App::new("random path generator")
        .arg(
            Arg::with_name("number of paths")
                .short("n")
                .value_name("n")
                .takes_value(true)
                .help("number of paths to generate")
                .required(true),
        )
        .arg(
            Arg::with_name("seed")
                .short("s")
                .value_name("SEED")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("graph file")
                .short("g")
                .help(".fmi graph file")
                .takes_value(true)
                .value_name("*.fmi")
                .required(true),
        )
        .arg(
            Arg::with_name("out file")
                .short("o")
                .help("file to store paths")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("parallel processing")
                .short("p")
                .help("run calculation multithreaded"),
        )
        .arg(Arg::with_name("verbose").short("v").help("output progress"))
        .get_matches();

    // store cli args into Args struct
    let args = Args {
        n: matches
            .value_of("number of paths")
            .unwrap()
            .parse::<usize>()
            .unwrap(),
        seed: match matches.value_of("seed") {
            None => None,
            Some(s) => Some(s.parse::<u64>().unwrap()),
        },
        graph_file: matches.value_of("graph file").unwrap(),
        out_file: matches.value_of("out file").unwrap(),
        parallel: matches.is_present("parallel processing"),
        verbose: matches.is_present("verbose"),
    };

    // load the graph
    if args.verbose {
        println!("Loading graph");
    }
    let g = load_ch_graph(&args.graph_file)?;
    if args.verbose {
        println!("done");
    }

    // run path generation
    if args.parallel {
        generate_and_store_par(
            args.n,
            args.seed,
            &g,
            args.out_file.to_string(),
            args.verbose,
        );
    } else {
        generate_and_store(args.n, args.seed, &g, &args.out_file, args.verbose);
    }

    Ok(())
}

fn generate_and_store_par<N: BaseNode, E: CHEdge>(
    n: usize,
    seed: Option<u64>,
    graph: &dyn CHGraph<Node = N, Edge = E>,
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

    // source target pair generator is an iterator - allows easy parallel processing with rayon.
    let generator = STPGenerator::new(graph.num_nodes(), seed, n);

    if verbose {
        println!("starting path generation");
    }

    generator.into_iter().par_bridge().for_each_init(
        || Dijkstra::new(graph),
        |dijkstra, SourceTargetPair { source, target }| {
            if source == target {
                return;
            } // skip cases where there is no real path
            let path = dijkstra.ch_search(source, target);
            if let Some((_, path)) = path {
                // do not store pairs that are not connected
                send.send(path).expect("error sending data");
            }
        },
    );

    drop(send); // ! without this line the receiver will never stop waiting for more data

    println!("par done");

    if let Err(e) = writer_thread.join() {
        eprintln!("Unable to join internal thread: {:?}", e);
    }
}

fn generate_and_store<N: BaseNode, E: CHEdge>(
    n: usize,
    seed: Option<u64>,
    graph: &dyn CHGraph<Node = N, Edge = E>,
    filename: &str,
    verbose: bool,
) {
    let mut dijkstra = Dijkstra::new(graph);

    let generator = STPGenerator::new(graph.num_nodes(), seed, n);

    let mut i = 0;
    if verbose {
        println!("starting path generation");
    }
    if verbose {
        print!("paths generated: {}", i);
    }
    let mut wtr = PathWriter::new(&filename, false);
    for SourceTargetPair { source, target } in generator {
        if source == target {
            continue;
        }
        let path = dijkstra.ch_search(source, target);
        if let Some((_, path)) = path {
            wtr.save_path(path);
        }
        i += 1;
        if i % 1000 == 0 && verbose {
            print!("\rpaths generated: {}", i);
            stdout().flush().unwrap();
        }
    }
}
