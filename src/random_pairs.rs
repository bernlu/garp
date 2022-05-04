use crate::graph::{BaseNode, CHEdge, CHGraph};
use crate::paths::SourceTargetPair;

use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use rand::SeedableRng;

use std::io::{stdout, Write};

/// generator struct that can be iterated for n_max random point pairs
pub struct STPGenerator {
    rng: StdRng,
    dist: Uniform<usize>,
    n: usize,
    n_max: usize,
}

impl STPGenerator {
    pub fn new(max: usize, seed: Option<u64>, n: usize) -> Self {
        Self {
            rng: match seed {
                Some(seed) => StdRng::seed_from_u64(seed),
                None => StdRng::from_entropy(),
            },
            dist: Uniform::from(0..max),
            n: 0,
            n_max: n,
        }
    }

    pub fn generate(&mut self) -> SourceTargetPair {
        let p = self.dist.sample(&mut self.rng);
        let q = self.dist.sample(&mut self.rng);
        self.n += 1;
        SourceTargetPair {
            source: p.into(),
            target: q.into(),
        }
    }
}

impl Iterator for STPGenerator {
    type Item = SourceTargetPair;
    fn next(&mut self) -> Option<Self::Item> {
        if self.n >= self.n_max {
            None
        } else {
            Some(self.generate())
        }
    }
}

/// generates random point pairs
pub fn generate_pairs<N: BaseNode, E: CHEdge>(
    graph: &dyn CHGraph<Node = N, Edge = E>,
    n: usize,
    seed: Option<u64>,
) -> Vec<SourceTargetPair> {
    // initialize rng from seed if provided, from entropy otherwise
    let mut rng = match seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    };

    // --- point generation

    // create a distribution to pick random graph nodes
    let nodes = Uniform::from(0..graph.num_nodes());

    // generate random node pairs
    println!("generating random point pairs");
    let mut i = 0;
    print!("{}/{}", i, n);

    let point_pairs = (0..n)
        .map(|_| {
            // print progress
            i += 1;
            print!("\r{}/{}", i, n);
            stdout().flush().unwrap();

            // pick random nodes from the graph
            let p = nodes.sample(&mut rng);
            let q = nodes.sample(&mut rng);
            SourceTargetPair {
                source: p.into(),
                target: q.into(),
            }
        })
        .collect();
    println!("");
    point_pairs
}
