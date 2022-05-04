use staticmap::{
    tools::{CircleBuilder, LineBuilder},
    Error, StaticMap, StaticMapBuilder,
};

use crate::{
    graph::{BaseEdge, BaseNode, EdgeId, GeoGraph, GeoNode, NodeId},
    paths::EdgeList,
};

use super::{Color, VisBuilder};

pub struct MapBuilder<'a, N, E> {
    map: StaticMap,
    graph: &'a dyn GeoGraph<Node = N, Edge = E>,
}

impl<'a, N: GeoNode + BaseNode, E: BaseEdge> MapBuilder<'a, N, E> {
    pub fn germany(graph: &'a dyn GeoGraph<Node = N, Edge = E>) -> Result<Self, Error> {
        let germany = StaticMapBuilder::new()
            .width(2000)
            .height(2600)
            .lat_center(52.16)
            .lon_center(10.44)
            .zoom(8)
            .build()?;
        Ok(Self {
            map: germany,
            graph,
        })
    }

    pub fn line(&mut self, edge: EdgeId) -> Result<(), Error> {
        self.line_with_color(edge, Color::RED)
    }

    pub fn line_with_color(&mut self, edge: EdgeId, color: Color) -> Result<(), Error> {
        let e = self.graph.edge(edge);
        let p = GeoGraph::node(self.graph, e.source());
        let q = GeoGraph::node(self.graph, e.target());
        let lat = [p.lat(), q.lat()];
        let lon = [p.lon(), q.lon()];

        let line = LineBuilder::new()
            .lat_coordinates(lat)
            .lon_coordinates(lon)
            .simplify(true)
            .color(color.into())
            .build()?;

        self.map.add_tool(line);
        Ok(())
    }

    pub fn point_with_color_size(&mut self, p: NodeId, c: Color, size: f32) -> Result<(), Error> {
        let node = GeoGraph::node(self.graph, p);
        let point = CircleBuilder::new()
            .lat_coordinate(node.lat())
            .lon_coordinate(node.lon())
            .radius(size)
            .color(c.into())
            .build()?;

        self.map.add_tool(point);
        Ok(())
    }
}

impl<'a, N: GeoNode + BaseNode, E: BaseEdge> VisBuilder for MapBuilder<'a, N, E> {
    fn save(&mut self, filename: &str) {
        self.map.save_png(filename).expect("error writing file")
    }
    fn point_with_color(&mut self, p: NodeId, color: Color) {
        self.point_with_color_size(p, color, 3.)
            .expect("error drawing point")
    }
    fn path(&mut self, path: &EdgeList) {
        for &edge in path {
            self.line(edge).expect("error drawing line");
        }
    }
    fn line_with_color(&mut self, from: (f64, f64), to: (f64, f64), color: Color) {
        let lat = [from.0, to.0];
        let lon = [from.1, to.1];

        let line = LineBuilder::new()
            .lat_coordinates(lat)
            .lon_coordinates(lon)
            .simplify(true)
            .color(color.into())
            .build()
            .expect("error drawing line");

        self.map.add_tool(line);
    }
}
