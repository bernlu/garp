use chext::hittingset::HittingSet;
use chext::{load_hs_graph, load_paths};
use clap::{App, Arg};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::time::Instant;

struct Args<'a> {
    /// .fmi graph file
    graph_file: &'a str,

    /// list of files containing paths
    paths_files: Vec<&'a str>,

    /// limits the maximum iterations that the hitting set algorithm will run for
    maxiter: Option<usize>,
}

/// runs the hitting set algorithm twice.
/// once using only explorative scans and once using only full scans (optional: limited by max iterations)
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new(
        "Hittingset threshold test: calculates the hitting set twice, once using only explorative scans and once using only full scans.",
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
        Arg::with_name("paths files")
            .short("p")
            .help("paths files")
            .takes_value(true)
            .multiple(true)
            .required(true),
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
        paths_files: matches
            .values_of("paths files")
            .unwrap()
            .into_iter()
            .collect(),
        maxiter: matches.value_of("maxiter").and_then(|d| d.parse().ok()),
    };

    // load paths
    let paths = {
        println!("loading path files");

        let mut paths = Vec::new();
        for file in &args.paths_files {
            let mut p = load_paths(file)?;
            paths.append(&mut p);
        }
        paths
    };
    println!("number of paths: {}", paths.len());

    println!(
        "sum of path weights: {}",
        paths.par_iter().map(|p| p.weight).sum::<u64>()
    );

    // calculate hitting set twice

    println!("loading graph");

    let hsgraph = load_hs_graph(&args.graph_file)?;
    let hs_calc_explore = HittingSet::new_with_threshold(&hsgraph, paths.clone(), usize::MAX);
    let hs_calc_fullscan = HittingSet::new_with_threshold(&hsgraph, paths.clone(), 0);

    println!("calculating hitting set - explore");

    let now = Instant::now();
    hs_calc_explore.run_with_stats_maxiter(true, None);
    let duration = now.elapsed();
    println!("hitting set found. duration: {:?}", duration);

    println!("calculating hitting set - fullscan");

    let now = Instant::now();
    hs_calc_fullscan.run_with_stats_maxiter(true, args.maxiter);
    let duration = now.elapsed();
    println!("hitting set found. duration: {:?}", duration);

    Ok(())
}
