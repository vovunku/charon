#![allow(dead_code)]
//! A map with custom index types.
//!
//! This data-structure is mostly meant to be used with the index types defined
//! with [macros::generate_index_type]: by using custom index types, we
//! leverage the type checker to prevent us from mixing them.
use serde::{Serialize, Serializer};
pub use std::collections::btree_map::Iter as IterAll;
pub use std::collections::btree_map::IterMut as IterAllMut;
pub use std::collections::BTreeMap;
use std::iter::{FromIterator, IntoIterator};

pub struct Map<Id, T> {
    // We use a btree map so that the bindings are sorted by key
    pub map: std::collections::BTreeMap<Id, T>,
}

impl<Id: std::cmp::Ord, T> Map<Id, T> {
    pub fn new() -> Self {
        Map {
            map: std::collections::BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, id: Id, x: T) {
        self.map.insert(id, x);
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        self.map.get(&id)
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.map.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.map.iter().map(|(_, x)| x)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.map.iter_mut().map(|(_, x)| x)
    }

    pub fn iter_indexed(&self) -> impl Iterator<Item = (&Id, &T)> {
        self.map.iter()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

impl<'a, Id, T> IntoIterator for &'a Map<Id, T>
where
    T: Clone,
{
    type Item = (&'a Id, &'a T);
    type IntoIter = std::collections::btree_map::Iter<'a, Id, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<Id: Serialize, T: Clone + Serialize> Serialize for Map<Id, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;

        // Serialize as a sequence of pairs
        let mut seq = serializer.serialize_seq(Some(self.map.len()))?;
        for e in self {
            seq.serialize_element(&e)?;
        }
        seq.end()
    }
}

impl<Id, T> FromIterator<(Id, T)> for Map<Id, T>
where
    Id: std::cmp::Ord,
    T: Clone,
{
    #[inline]
    fn from_iter<It: IntoIterator<Item = (Id, T)>>(iter: It) -> Map<Id, T> {
        Map {
            map: BTreeMap::from_iter(iter),
        }
    }
}
