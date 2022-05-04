use rayon::iter::{ParallelBridge, ParallelIterator};
use rustc_hash::FxHashSet as Set;
use std::{cmp::Ordering, hash::Hash, ops::Deref};

// define reference pair of type T
type Pair<'a, T> = (&'a T, &'a T);

// wspd is defined based on this tree trait
pub trait Tree<'a>
where
    Self: 'a,
{
    type Iter: Iterator<Item = &'a Self> + Send;
    fn diameter(&self) -> f64;
    fn children(&'a self) -> Self::Iter;
    fn id(&self) -> &str;
}

pub trait Distance: Sized {
    fn distance(&self, other: &Self) -> f64;
}

pub struct WSPD<'a, T> {
    pairs: Set<Pair<'a, T>>,
}

impl<'a, T: Tree<'a> + Distance + Eq + Hash + Sync> WSPD<'a, T> {
    pub fn new(tree: &'a T, e: f64) -> Self {
        Self {
            pairs: Self::alg_wspd(&tree, &tree, e),
        }
    }

    /// from har-peled book
    fn alg_wspd(u: &'a T, v: &'a T, e: f64) -> Set<Pair<'a, T>> {
        if u == v && u.diameter() == 0.0 {
            return Set::default();
        }
        let (u, v) = match u.diameter().partial_cmp(&v.diameter()) {
            Some(Ordering::Less) => (v, u),
            Some(Ordering::Greater) => (u, v),
            Some(Ordering::Equal) if u.id() > v.id() => (u, v),
            _ => (v, u),
        };

        if u.diameter() <= e * u.distance(v) {
            let mut s = Set::default();
            s.insert((u, v));
            return s;
        }

        // parallel impl of recursive call on all children
        let res: Set<Pair<'a, T>> = u
            .children()
            .par_bridge()
            .flat_map(|child| Self::alg_wspd(child, v, e))
            .collect();

        res
    }
}

impl<'a, T> Deref for WSPD<'a, T> {
    type Target = Set<Pair<'a, T>>;

    fn deref(&self) -> &Self::Target {
        &self.pairs
    }
}
