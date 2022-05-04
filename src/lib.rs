pub mod dijkstra;
pub mod file_handling;
pub mod graph;
pub mod hittingset;
pub mod paths;
pub mod quadtree;
pub mod random_pairs;
pub mod vis;
pub mod wspd;

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter},
};

use csv::{ReaderBuilder, Writer, WriterBuilder};
use graph::{AdjArrayGraph, FMIGraph, NodeId, StoreableGraph, ToporderedGraph};
use paths::CHEdgeList;


// helper functions for loading and caching data

/// load a FMIGraph from a .fmi text file or a cache file if available
pub fn load_fmi_graph(filename: &str) -> Result<FMIGraph, Box<dyn std::error::Error>> {
    match FMIGraph::from_file_binary(&[filename, ".fmigraph"].concat()) {
        Ok(g) => Ok(g),
        Err(_) => {
            let g = FMIGraph::from_fmi_maxspeed_ch_txt(&filename)?;
            g.to_file_binary(&[filename, ".fmigraph"].concat())?;
            Ok(g)
        }
    }
}
/// load a CHGraph from a .fmi text file or a cache file if available
pub fn load_ch_graph(filename: &str) -> Result<AdjArrayGraph, Box<dyn std::error::Error>> {
    match AdjArrayGraph::from_file_binary(&[filename, ".chgraph"].concat()) {
        Ok(g) => Ok(g),
        Err(_) => {
            let g = load_fmi_graph(filename)?;
            let g: AdjArrayGraph = g.into();
            g.to_file_binary(&[filename, ".chgraph"].concat())?;
            Ok(g)
        }
    }
}

/// load a HSGraph from a .fmi text file or a cache file if available
pub fn load_hs_graph(filename: &str) -> Result<ToporderedGraph, Box<dyn std::error::Error>> {
    match ToporderedGraph::from_file_binary(&[filename, ".hsgraph"].concat()) {
        Ok(g) => Ok(g),
        Err(_) => {
            let g = load_fmi_graph(filename)?;
            let g: ToporderedGraph = g.into();
            g.to_file_binary(&[filename, ".hsgraph"].concat())?;
            Ok(g)
        }
    }
}

/// load a paths file
pub fn load_paths(filename: &str) -> Result<Vec<CHEdgeList>, Box<dyn std::error::Error>> {
    let mut rdr = ReaderBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_path(filename)
        .unwrap();

    let mut paths = Vec::new();

    for record in rdr.deserialize() {
        let path: CHEdgeList = record?;
        paths.push(path);
    }
    Ok(paths)
}

pub struct PathWriter {
    wtr: Writer<BufWriter<File>>,
}

impl PathWriter {
    pub fn new(filename: &str, append: bool) -> Self {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(filename)
            .expect(&format!("creating out file {} failed", filename));

        let buf = BufWriter::new(file);
        let wtr = WriterBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_writer(buf);

        Self { wtr }
    }

    /// store a paths file
    pub fn save_path(&mut self, path: CHEdgeList) {
        self.wtr.serialize(path).unwrap();
    }
}

/// load a hitting set file
pub fn load_hittingset(filename: &str) -> Vec<NodeId> {
    let file = File::open(filename).expect("no such file");
    let rdr = BufReader::new(file);
    rdr.lines()
        .map(|l| {
            l.expect("parsing error")
                .parse::<usize>()
                .expect("parsing error")
                .into()
        })
        .collect()
}
