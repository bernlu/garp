use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Deref,
};

use crate::{
    graph::{BaseNode, CHNode, GeoNode},
    wspd::{Distance, Tree},
};

pub trait TreeNode: GeoNode + CHNode + BaseNode + Sync {}
impl<N> TreeNode for N where N: GeoNode + CHNode + BaseNode + Sync {}

struct Entry<'a> {
    point: &'a dyn TreeNode,
    x: f64,
    y: f64,
}

impl<'a> Deref for Entry<'a> {
    type Target = &'a dyn TreeNode;

    fn deref(&self) -> &Self::Target {
        &self.point
    }
}

struct Children<'a> {
    a: Option<Box<QuadTree<'a>>>,
    b: Option<Box<QuadTree<'a>>>,
    c: Option<Box<QuadTree<'a>>>,
    d: Option<Box<QuadTree<'a>>>,
}

impl<'a> Children<'a> {
    pub fn empty() -> Self {
        Self {
            a: None,
            b: None,
            c: None,
            d: None,
        }
    }

    pub fn iter(&self) -> ChildrenIterator {
        (&self).into_iter()
    }
}

pub struct ChildrenIterator<'a> {
    children: &'a Children<'a>,
    index: u8,
}

impl<'a> IntoIterator for &'a Children<'a> {
    type Item = &'a QuadTree<'a>;
    type IntoIter = ChildrenIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ChildrenIterator {
            children: self,
            index: 0,
        }
    }
}

impl<'a> Iterator for ChildrenIterator<'a> {
    type Item = &'a QuadTree<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index == 0 {
            self.index += 1;
            if let Some(c) = &self.children.a {
                return Some(c);
            }
        }
        if self.index == 1 {
            self.index += 1;
            if let Some(c) = &self.children.b {
                return Some(c);
            }
        }
        if self.index == 2 {
            self.index += 1;
            if let Some(c) = &self.children.c {
                return Some(c);
            }
        }
        if self.index == 3 {
            self.index += 1;
            if let Some(c) = &self.children.d {
                return Some(c);
            }
        }
        return None;
    }
}

pub struct QuadTree<'a> {
    children: Children<'a>,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    data: Vec<&'a dyn TreeNode>,
    scaler: MinMaxScaler,
    pub id: String, // define:  a b
                    //          c d
                    // and self: ""
                    // ordered from bottom to top: cba => topleft then topright then bottomleft
}

impl<'a> PartialEq for QuadTree<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<'a> Hash for QuadTree<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<'a> Eq for QuadTree<'a> {}

impl<'a> Distance for QuadTree<'a> {
    fn distance(&self, other: &Self) -> f64 {
        // distance between corners, return minimal distance
        let sa = (self.xmin, self.ymin);
        let sb = (self.xmin, self.ymax);
        let sc = (self.xmax, self.ymin);
        let sd = (self.xmax, self.ymax);

        let oa = (other.xmin, other.ymin);
        let ob = (other.xmin, other.ymax);
        let oc = (other.xmax, other.ymin);
        let od = (other.xmax, other.ymax);

        let scorners = vec![sa, sb, sc, sd];
        let ocorners = vec![oa, ob, oc, od];

        scorners
            .iter()
            .flat_map(|&sc| {
                ocorners
                    .iter()
                    .clone()
                    .map(move |&oc| Self::point_distance(oc, sc))
            })
            .reduce(|a, b| a.min(b))
            .unwrap()
    }
}

impl<'a> QuadTree<'a> {
    /// returns the point with largest level
    pub fn rep(&'a self) -> &'a dyn TreeNode {
        if self.data.len() > 0 {
            self.data[0]
        } else {
            self.children
                .iter()
                .map(|c| c.rep())
                .max_by(|a, b| a.level().cmp(&b.level()))
                .unwrap()
        }
    }

    /// iterates all points in this tree
    pub fn points(&'a self) -> Box<dyn Iterator<Item = &&'a dyn TreeNode> + 'a> {
        Box::new(
            self.children
                .iter()
                .flat_map(|c| c.points())
                .chain(&self.data),
        )
    }

    /// number of elements in this tree
    pub fn size(&self) -> usize {
        self.children.iter().fold(0, |acc, c| acc + c.size()) + self.data.len()
    }

    /// L2 distance
    fn point_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
        ((a.0 - b.0) * (a.0 - b.0) + (a.1 - b.1) * (a.1 - b.1)).sqrt()
    }

    /// returns (sub) tree by id
    pub fn get_by_id(&self, mut id: String) -> &Self {
        let next = id.pop();
        match next {
            None => &self,
            Some('a') => {
                if let Some(child) = &self.children.a {
                    child.get_by_id(id)
                } else {
                    &self
                }
            }
            Some('b') => {
                if let Some(child) = &self.children.b {
                    child.get_by_id(id)
                } else {
                    &self
                }
            }
            Some('c') => {
                if let Some(child) = &self.children.c {
                    child.get_by_id(id)
                } else {
                    &self
                }
            }
            Some('d') => {
                if let Some(child) = &self.children.d {
                    child.get_by_id(id)
                } else {
                    &self
                }
            }
            Some(_) => {
                unreachable!("id string contains char not in [abcd]");
            }
        }
    }

    /// returns this cell's edges as pairs of lat/lon coordinates
    pub fn cell_edges(&self) -> Vec<((f64, f64), (f64, f64))> {
        let bl = inverse_mercator_projection(self.scaler.inverse_scale(self.xmin, self.ymin)); // bottom left
        let tl = inverse_mercator_projection(self.scaler.inverse_scale(self.xmin, self.ymax)); // top left
        let br = inverse_mercator_projection(self.scaler.inverse_scale(self.xmax, self.ymin)); // bottom right
        let tr = inverse_mercator_projection(self.scaler.inverse_scale(self.xmax, self.ymax)); // top right
        vec![
            (bl, br), // bottom
            (bl, tl), // left
            (br, tr), // right
            (tl, tr), // top
        ]
    }

    /// creates a new tree with max depth
    pub fn new(data: Vec<&'a dyn TreeNode>, maxdepth: usize) -> Self {
        // 1. mercator project points
        let entries: Vec<Entry> = data
            .into_iter()
            .map(|point| {
                let (x, y) = mercator_projection(point.lat(), point.lon());
                Entry { point, x, y }
            })
            .collect();
        // 2. scale to [0,1]
        let scaler = MinMaxScaler::from(&entries); // create scaler object (need this to inverse scale later)
        let entries_scaled = entries
            .into_iter()
            .map(|Entry { point, x, y }| {
                let (x_scaled, y_scaled) = scaler.scale(x, y);
                Entry {
                    point,
                    x: x_scaled,
                    y: y_scaled,
                }
            })
            .collect();
        // create sub trees
        Self::new_cell(
            0.0,
            1.0,
            0.0,
            1.0,
            entries_scaled,
            maxdepth,
            0,
            "".to_string(),
            scaler,
        )
    }

    /// subroutine for creating a new tree. recursively creates a subtree with specified parameters
    fn new_cell(
        xmin: f64,
        xmax: f64,
        ymin: f64,
        ymax: f64,
        data: Vec<Entry<'a>>,
        maxdepth: usize,
        current_depth: usize,
        id: String,
        scaler: MinMaxScaler,
    ) -> Self {
        let mut children = Children::empty();
        // if there is enough data and we are not at maximum depth, create children
        if data.len() > 1 && current_depth < maxdepth {
            // create children
            let mut topleft_data = Vec::new();
            let mut topright_data = Vec::new();
            let mut bottomleft_data = Vec::new();
            let mut bottomright_data = Vec::new();

            let xhalf = xmin + (xmax - xmin) / 2.0;
            let yhalf = ymin + (ymax - ymin) / 2.0;

            // map data to children
            for point in data {
                if point.x > xhalf {
                    if point.y > yhalf {
                        topright_data.push(point);
                    } else {
                        bottomright_data.push(point);
                    }
                } else {
                    if point.y > yhalf {
                        topleft_data.push(point);
                    } else {
                        bottomleft_data.push(point);
                    }
                }
            }

            // if there is data for a child cell, recursive call to create that child cell
            if topleft_data.len() > 0 {
                let topleft_id = "a".to_string() + &id; // the cell's id
                let node = QuadTree::new_cell(
                    xmin,
                    xhalf,
                    yhalf,
                    ymax,
                    topleft_data,
                    maxdepth,
                    current_depth + 1,
                    topleft_id,
                    scaler.clone(),
                );
                children.a = Some(Box::new(node)); // save subtree as child
            }
            if topright_data.len() > 0 {
                let topright_id = "b".to_string() + &id;
                let node = QuadTree::new_cell(
                    xhalf,
                    xmax,
                    yhalf,
                    ymax,
                    topright_data,
                    maxdepth,
                    current_depth + 1,
                    topright_id,
                    scaler.clone(),
                );
                children.b = Some(Box::new(node));
            }
            if bottomleft_data.len() > 0 {
                let bottomleft_id = "c".to_string() + &id;
                let node = QuadTree::new_cell(
                    xmin,
                    xhalf,
                    ymin,
                    yhalf,
                    bottomleft_data,
                    maxdepth,
                    current_depth + 1,
                    bottomleft_id,
                    scaler.clone(),
                );
                children.c = Some(Box::new(node));
            }
            if bottomright_data.len() > 0 {
                let bottomright_id = "d".to_string() + &id;
                let node = QuadTree::new_cell(
                    xhalf,
                    xmax,
                    ymin,
                    yhalf,
                    bottomright_data,
                    maxdepth,
                    current_depth + 1,
                    bottomright_id,
                    scaler.clone(),
                );
                children.d = Some(Box::new(node));
            }
            // return this cell
            Self {
                xmin,
                xmax,
                ymin,
                ymax,
                children,
                data: vec![],
                scaler: scaler,
                id,
            }
        } else {
            // do not create children. this node is a leaf.
            let mut points: Vec<&'a dyn TreeNode> =
                data.into_iter().map(|Entry { point, .. }| point).collect();
            // sort this leaf's data (graph nodes) by level
            points.sort_by(|a, b| a.level().cmp(&b.level()).reverse());
            // return this cell.
            Self {
                xmin,
                xmax,
                ymin,
                ymax,
                children,
                data: points,
                scaler: scaler,
                id,
            }
        }
    }
}

// impl Tree Trait for WSPD
impl<'a> Tree<'a> for QuadTree<'a> {
    type Iter = ChildrenIterator<'a>;
    fn children(&'a self) -> Self::Iter {
        self.children.iter()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn diameter(&self) -> f64 {
        if self.data.len() == 1 {
            // this is a leaf
            0.0 // by definition diameter = 0
        }
        // for cells with more than one node just use the cell size as diameter
        else if self.data.len() == 0 {
            // cell is inner node (stores no points)
            self.xmax - self.xmin
        } else {
            // cell is a leaf but contains more than one point
            self.xmax - self.xmin
        }
    }
}

/// maps from (lat φ, lon λ) to (x,y)
fn mercator_projection(φ: f64, λ: f64) -> (f64, f64) {
    (λ.to_radians(), φ.to_radians().sin().atanh())
}

/// maps from (x,y) to (lat φ, lon λ)
fn inverse_mercator_projection((x, y): (f64, f64)) -> (f64, f64) {
    (y.sinh().atan().to_degrees(), x.to_degrees())
}

#[derive(Clone, Debug)]
struct MinMaxScaler {
    x_max: f64,
    x_min: f64,
    y_max: f64,
    y_min: f64,
}

impl MinMaxScaler {
    /// creates a scaler from a dataset
    pub fn from(data: &[Entry]) -> Self {
        let (mut x_max, mut x_min) = (f64::MIN, f64::MAX);
        let (mut y_max, mut y_min) = (f64::MIN, f64::MAX);

        for d in data {
            if d.x > x_max {
                x_max = d.x;
            }
            if d.x < x_min {
                x_min = d.x;
            }
            if d.y > y_max {
                y_max = d.y;
            }
            if d.y < y_min {
                y_min = d.y;
            }
        }

        Self {
            x_max,
            x_min,
            y_max,
            y_min,
        }
    }

    /// scales (x,y) to min/max range
    pub fn scale(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (x - self.x_min) / (self.x_max - self.x_min),
            (y - self.y_min) / (self.y_max - self.y_min),
        )
    }

    /// scales from min/max range to (x,y)
    pub fn inverse_scale(&self, x: f64, y: f64) -> (f64, f64) {
        (
            (self.x_max - self.x_min) * x + self.x_min,
            (self.y_max - self.y_min) * y + self.y_min,
        )
    }
}
