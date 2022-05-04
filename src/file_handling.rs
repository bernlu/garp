use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};

use bincode::{deserialize_from, serialize_into};

use crate::graph::{FMIEdge, FMIGraph, FMINode};

impl FMIGraph {
    /// reader for a .fmi file containing a CH graph
    pub fn from_fmi_maxspeed_ch_txt(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        /*
            FMI maxspeed ch format

            required metadata:
            # Id : [hexstring]
            # Timestamp : [int]
            # Type: chgraph
            # Revision: 1


            File structure:

            # xx...x //Metadaten
            ...

            [Anzahl Knoten]
            [Anzahl Kanten]
            [Id] [osmId] [lat] [lon] [elevation] [chlevel] //Knoten
            ...
            [source] [target] [weight] [type] [maxspeed] [child1] [child2]  //Kante. child = -1 <=> kein child
            ...
        */

        // check metadata
        let file = File::open(filename)?;
        let file = BufReader::new(file);

        for line in file.lines() {
            let line = line?;
            if line.starts_with("#") {
                if line.contains(" Type ") && !line.contains("chgraph") {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrong file type",
                    )));
                }
            }
            if line.is_empty() {
                break;
            }
        }

        // setup header
        let header_nodes =
            csv::StringRecord::from(vec!["id", "osm", "lat", "lon", "elevation", "level"]);
        let header_edges = csv::StringRecord::from(vec![
            "source", "target", "cost", "type", "maxspeed", "child1", "child2", "id",
        ]);

        // send to generic reader
        let file = File::open(filename)?;
        let file = BufReader::new(file);
        let reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .comment(Some(b'#'))
            .delimiter(b' ')
            .from_reader(file);

        Self::from_fmi_csv(header_nodes, header_edges, reader)
    }

    /// reader for a .fmi file containing a non-ch graph
    pub fn from_fmi_maxspeed_txt(filename: &str) -> Result<Self, Box<dyn std::error::Error>> {
        /*
            FMI maxspeed format.

            required metadata:
            # Id : [hexstring]
            # Timestamp : [int]
            # Type: maxspeed
            # Revision: 1

            File structure:

            # xx...x //Metadaten
            ...

            [Anzahl Knoten]
            [Anzahl Kanten]
            [Id] [osmId] [lat] [lon] [elevation] //Knoten
            ...
            [source] [target] [weight] [type] [maxspeed] //Kante
            ...
        */

        // check metadata
        let file = File::open(filename)?;
        let file = BufReader::new(file);

        for line in file.lines() {
            let line = line?;
            if line.starts_with("#") {
                if line.contains(" Type ") && !line.contains("maxspeed") {
                    return Err(Box::new(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "wrong file type",
                    )));
                }
            }
            if line.is_empty() {
                break;
            }
        }

        // setup header
        let header_nodes = csv::StringRecord::from(vec!["id", "osm", "lat", "lon", "elevation"]);
        let header_edges =
            csv::StringRecord::from(vec!["source", "target", "cost", "type", "maxspeed", "id"]);

        // send to generic reader
        let file = File::open(filename)?;
        let file = BufReader::new(file);
        let reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .comment(Some(b'#'))
            .delimiter(b' ')
            .from_reader(file);

        Self::from_fmi_csv(header_nodes, header_edges, reader)
    }

    /// generic fmi csv reader, using headers set in other reader functions
    fn from_fmi_csv<R: std::io::Read>(
        header_nodes: csv::StringRecord,
        header_edges: csv::StringRecord,
        mut reader: csv::Reader<R>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // read #nodes = n
        let mut record = csv::StringRecord::new();
        reader.read_record(&mut record)?;
        let n: usize = record.deserialize(None)?;

        // read #edges = m
        reader.read_record(&mut record)?;
        let m: usize = record.deserialize(None)?;

        let mut nodes = Vec::with_capacity(n);
        let mut edges = Vec::with_capacity(m);

        // read nodes
        for _i in 0..n {
            reader.read_record(&mut record)?;
            let node: FMINode = record.deserialize(Some(&header_nodes))?;
            nodes.push(node);
        }

        // read edges
        for i in 0..m {
            reader.read_record(&mut record)?;
            record.push_field(&i.to_string());
            let edge: FMIEdge = record.deserialize(Some(&header_edges))?;
            edges.push(edge);
        }
        Ok(Self { nodes, edges })
    }

    pub fn to_file_binary(&self, filename: &str) -> Result<(), bincode::Error> {
        let file = File::create(filename)?;
        let mut writer = BufWriter::new(file);
        serialize_into(&mut writer, self)
    }

    pub fn from_file_binary(filename: &str) -> Result<Self, bincode::Error> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        deserialize_from(reader)
    }
}
