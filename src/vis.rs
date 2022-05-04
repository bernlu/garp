mod drawmap;
mod geojsonbuilder;

pub use drawmap::MapBuilder;
pub use geojsonbuilder::GeoJsonBuilder;
use rand::Rng;

use crate::{graph::NodeId, paths::EdgeList};

pub trait VisBuilder {
    fn save(&mut self, file: &str);
    fn path(&mut self, path: &EdgeList);
    fn point(&mut self, point: NodeId) {
        self.point_with_color(point, Color::RED);
    }
    // both: (lat, lon)
    fn line(&mut self, from: (f64, f64), to: (f64, f64)) {
        self.line_with_color(from, to, Color::BLACK);
    }
    fn line_with_color(&mut self, from: (f64, f64), to: (f64, f64), color: Color);
    fn point_with_color(&mut self, point: NodeId, color: Color);
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn random() -> Self {
        Self {
            r: rand::thread_rng().gen(),
            g: rand::thread_rng().gen(),
            b: rand::thread_rng().gen(),
        }
    }

    pub const RED: Self = Self { r: 255, g: 0, b: 0 };
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
}

impl From<Color> for staticmap::tools::Color {
    fn from(c: Color) -> Self {
        Self::new(true, c.r, c.g, c.b, 255)
    }
}
