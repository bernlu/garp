use std::ops::{Deref, DerefMut};

use derive_more::{Deref, Index, IntoIterator};
use serde::{Deserialize, Serialize};

use crate::graph::{EdgeId, NodeId};

#[derive(PartialEq, Eq, Clone)]
pub struct SourceTargetPair {
    pub source: NodeId,
    pub target: NodeId,
}

impl SourceTargetPair {
    pub fn new(s: usize, t: usize) -> Self {
        Self {
            source: s.into(),
            target: t.into(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, IntoIterator, Index, Debug)]
pub struct CHEdgeList {
    #[serde(default = "weight_default")]
    pub weight: u64,
    #[index]
    #[into_iterator(owned, ref, ref_mut)]
    pub list: Vec<EdgeId>,
}

fn weight_default() -> u64 {
    1
}

impl CHEdgeList {
    pub fn weighted(list: Vec<EdgeId>, weight: u64) -> Self {
        Self { list, weight }
    }
    pub fn new(list: Vec<EdgeId>) -> Self {
        Self::weighted(list, 1)
    }
    pub const EMPTY: Self = CHEdgeList {
        list: vec![],
        weight: 0,
    };
}

impl Deref for CHEdgeList {
    type Target = Vec<EdgeId>;
    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for CHEdgeList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

#[derive(PartialEq, Eq, Clone, IntoIterator, Deref)]
#[into_iterator(owned, ref, ref_mut)]
pub struct EdgeList(pub Vec<EdgeId>);
