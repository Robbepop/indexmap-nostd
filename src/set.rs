//! An ordered set based on a B-Tree that keeps insertion order of elements.

use super::SlotIndex;
use alloc::collections::{btree_map, BTreeMap};
use alloc::vec::IntoIter as VecIntoIter;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::iter::FusedIterator;
use core::slice::Iter as SliceIter;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexSet<T> {
    /// A mapping from keys to slot indices.
    key2slot: BTreeMap<T, SlotIndex>,
    /// A vector holding all keys.
    slots: Vec<T>,
}

impl<T> Default for IndexSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IndexSet<T> {
    /// Makes a new, empty `IndexSet`.
    ///
    /// Does not allocate anything on its own.
    pub fn new() -> Self {
        Self {
            key2slot: BTreeMap::new(),
            slots: Vec::new(),
        }
    }

    /// Returns the number of elements in the set.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() != 0
    }

    /// Returns `true` if the set contains an element equal to the value.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    pub fn contains<Q: ?Sized>(&self, key: &Q) -> bool
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.key2slot.contains_key(key)
    }

    /// Returns a reference to the element in the set, if any, that is equal to
    /// the value.
    ///
    /// The value may be any borrowed form of the set's element type,
    /// but the ordering on the borrowed form *must* match the
    /// ordering on the element type.
    pub fn get<Q: ?Sized>(&self, value: &Q) -> Option<&T>
    where
        T: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.key2slot
            .get(value)
            .map(|index| &self.slots[index.index()])
    }

    /// Adds a value to the set.
    ///
    /// Returns whether the value was newly inserted. That is:
    ///
    /// - If the set did not previously contain an equal value, `true` is
    ///   returned.
    /// - If the set already contained an equal value, `false` is returned, and
    ///   the entry is not updated.
    pub fn insert(&mut self, value: T) -> bool
    where
        T: Ord + Clone,
    {
        match self.key2slot.entry(value.clone()) {
            btree_map::Entry::Vacant(entry) => {
                let new_slot = self.slots.len();
                entry.insert(SlotIndex(new_slot));
                self.slots.push(value);
                true
            }
            btree_map::Entry::Occupied(entry) => {
                let index = entry.get().index();
                self.slots[index] = value;
                false
            }
        }
    }

    /// Gets an iterator that visits the elements in the [`IndexSet`]
    /// in the order in which they have been inserted into the set unless
    /// there have been removals.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            iter: self.slots.iter(),
        }
    }

    /// Clears the set, removing all elements.
    pub fn clear(&mut self) {
        self.key2slot.clear();
        self.slots.clear();
    }
}

impl<'a, T> Extend<&'a T> for IndexSet<T>
where
    T: Ord + Copy,
{
    #[allow(clippy::map_clone)] // lifetime issue: seems to be a clippy bug
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = &'a T>,
    {
        self.extend(iter.into_iter().map(|value| *value))
    }
}

impl<T> Extend<T> for IndexSet<T>
where
    T: Ord + Clone,
{
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        iter.into_iter().for_each(move |value| {
            self.insert(value);
        });
    }
}

impl<T> FromIterator<T> for IndexSet<T>
where
    T: Ord + Clone,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut set = IndexSet::new();
        set.extend(iter);
        set
    }
}

impl<T, const N: usize> From<[T; N]> for IndexSet<T>
where
    T: Ord + Clone,
{
    fn from(items: [T; N]) -> Self {
        items.into_iter().collect()
    }
}

impl<'a, T> IntoIterator for &'a IndexSet<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T> IntoIterator for IndexSet<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.slots.into_iter(),
        }
    }
}

/// An iterator over the items of a [`IndexSet`].
///
/// This `struct` is created by the [`iter`] method on [`IndexSet`].
#[derive(Debug, Clone)]
pub struct Iter<'a, T> {
    iter: SliceIter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<'a, T> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<'a, T> ExactSizeIterator for Iter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// An owning iterator over the items of a [`IndexSet`].
///
/// This `struct` is created by the [`into_iter`] method on [`IndexSet`]
/// (provided by the [`IntoIterator`] trait).
impl<'a, T> FusedIterator for Iter<'a, T> {}
#[derive(Debug)]
pub struct IntoIter<T> {
    iter: VecIntoIter<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<T> ExactSizeIterator for IntoIter<T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<T> FusedIterator for IntoIter<T> {}
