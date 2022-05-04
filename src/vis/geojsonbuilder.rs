use std::{fs::File, io::BufWriter};

use geojson::{Feature, FeatureCollection, Geometry, Value};
use serde_json::{to_value, Map};

use crate::{
    graph::{BaseEdge, BaseNode, GeoGraph, GeoNode, NodeId},
    paths::EdgeList,
};

use super::{Color, VisBuilder};

pub struct GeoJsonBuilder<'a, N, E> {
    geojson: FeatureCollection,
    graph: &'a dyn GeoGraph<Node = N, Edge = E>,
}

impl<'a, N: GeoNode + BaseNode, E: BaseEdge> GeoJsonBuilder<'a, N, E> {
    pub fn new(graph: &'a dyn GeoGraph<Node = N, Edge = E>) -> Self {
        Self {
            geojson: FeatureCollection {
                bbox: None,
                features: vec![],
                foreign_members: None,
            },
            graph,
        }
    }

    pub fn path_with_color(&mut self, path: &EdgeList, color: Color) {
        if path.len() == 0 {
            return;
        }
        let mut geom_linestring = Vec::new();
        for edge in path {
            let start = self.graph.edge(*edge).source();
            let node = GeoGraph::node(self.graph, start);

            let position = vec![node.lon(), node.lat()];
            geom_linestring.push(position);
        }
        let last_end = self.graph.edge(*path.last().unwrap()).target();
        let node = GeoGraph::node(self.graph, last_end);

        let position = vec![node.lon(), node.lat()];
        geom_linestring.push(position);

        let geom = Geometry::new(Value::LineString(geom_linestring));

        let mut properties = Map::new();
        // format: {:02X} -> two digit with leading zero, hex with uppercase
        properties.insert(
            String::from("stroke"),
            to_value(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)).unwrap(),
        );

        let feature = Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        self.geojson.features.push(feature);
    }
}

impl<'a, N: GeoNode + BaseNode, E: BaseEdge> VisBuilder for GeoJsonBuilder<'a, N, E> {
    fn save(&mut self, filename: &str) {
        let file = File::create(filename).expect("error writing file");
        let buf = BufWriter::new(file);

        serde_json::to_writer(buf, &self.geojson).expect("error writing file");
    }

    fn point_with_color(&mut self, point: NodeId, color: Color) {
        let node = GeoGraph::node(self.graph, point);
        let geom = Geometry::new(Value::Point(vec![node.lon(), node.lat()]));

        let mut properties = Map::new();
        // format: {:02X} -> two digit with leading zero, hex with uppercase
        properties.insert(
            String::from("marker-color"),
            to_value(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)).unwrap(),
        );

        let feature = Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        self.geojson.features.push(feature);
    }

    fn path(&mut self, path: &EdgeList) {
        self.path_with_color(path, Color::RED)
    }

    fn line_with_color(&mut self, from: (f64, f64), to: (f64, f64), color: Color) {
        let mut geom_linestring = Vec::new();
        geom_linestring.push(vec![from.1, from.0]);
        geom_linestring.push(vec![to.1, to.0]);

        let geom = Geometry::new(Value::LineString(geom_linestring));

        let mut properties = Map::new();
        // format: {:02X} -> two digit with leading zero, hex with uppercase
        properties.insert(
            String::from("stroke"),
            to_value(format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)).unwrap(),
        );

        let feature = Feature {
            bbox: None,
            geometry: Some(geom),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        self.geojson.features.push(feature);
    }
}
