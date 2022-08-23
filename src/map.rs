//! An ordered map based on a B-Tree that keeps insertion order of elements.

use alloc::collections::{btree_map, BTreeMap};
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt;
use core::iter::FusedIterator;
use core::mem::replace;
use core::ops::Index;
use core::slice::Iter as SliceIter;
use core::slice::IterMut as SliceIterMut;

/// A slot index referencing a [`Slot`] in an [`IndexMap`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SlotIndex(usize);

impl SlotIndex {
    /// Returns the raw `usize` index of the [`SlotIndex`].
    pub fn index(self) -> usize {
        self.0
    }
}

/// A hash table where the iteration order of the key-value
/// pairs is independent of the hash values of the keys.
///
/// The interface is closely compatible with the [`indexmap` crate]
/// and a subset of the features that is relevant for the
/// [`wasmparser-nostd` crate].
///
/// # Differences to original IndexMap
///
/// Since the goal of this crate was to maintain a simple
/// `no_std` compatible fork of the [`indexmap` crate] there are some
/// downsides and differences.
///
/// - Some operations such as `IndexMap::insert` now require `K: Clone`.
/// - It is to be expected that this fork performs worse than the original
/// [`indexmap` crate] implementation.
/// - The implementation is based on `BTreeMap` internally instead of
/// `HashMap` which has the effect that methods no longer require `K: Hash`
/// but `K: Ord` instead.
///
/// [`indexmap` crate]: https://crates.io/crates/indexmap
/// [`wasmparser-nostd` crate]: https://crates.io/crates/wasmparser-nostd
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexMap<K, V> {
    /// A mapping from keys to slot indices.
    key2slot: BTreeMap<K, SlotIndex>,
    /// A vector holding all slots of key value pairs.
    slots: Vec<Slot<K, V>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Slot<K, V> {
    /// The key of the [`Slot`].
    key: K,
    /// The value of the [`Slot`].
    value: V,
}

impl<K, V> Slot<K, V> {
    /// Creates a new [`Slot`] from the given `key` and `value`.
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    /// Returns the [`Slot`] as a pair of references to its `key` and `value`.
    pub fn as_pair(&self) -> (&K, &V) {
        (&self.key, &self.value)
    }

    /// Returns the [`Slot`] as a pair of references to its `key` and `value`.
    pub fn as_pair_mut(&mut self) -> (&K, &mut V) {
        (&self.key, &mut self.value)
    }
}

impl<K, V> Default for IndexMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> IndexMap<K, V> {
    /// Makes a new, empty `IndexMap`.
    ///
    /// Does not allocate anything on its own.
    pub fn new() -> Self {
        Self {
            key2slot: BTreeMap::new(),
            slots: Vec::new(),
        }
    }

    /// Returns the number of elements in the map.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len() != 0
    }

    /// Returns true if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map’s key type,
    /// but the ordering on the borrowed form must match the ordering on the key type.
    pub fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.key2slot.contains_key(key)
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned. The key is not updated, though; this matters for
    /// types that can be `==` without being identical.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Ord + Clone,
    {
        match self.key2slot.entry(key.clone()) {
            btree_map::Entry::Vacant(entry) => {
                let new_slot = self.slots.len();
                entry.insert(SlotIndex(new_slot));
                self.slots.push(Slot::new(key, value));
                None
            }
            btree_map::Entry::Occupied(entry) => {
                let index = entry.get().index();
                let new_slot = Slot::new(key, value);
                let old_slot = replace(&mut self.slots[index], new_slot);
                Some(old_slot.value)
            }
        }
    }

    /// Gets the given key’s corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<K, V>
    where
        K: Ord + Clone,
    {
        match self.key2slot.entry(key) {
            btree_map::Entry::Vacant(entry) => Entry::Vacant(VacantEntry {
                vacant: entry,
                slots: &mut self.slots,
            }),
            btree_map::Entry::Occupied(entry) => Entry::Occupied(OccupiedEntry {
                occupied: entry,
                slots: &mut self.slots,
            }),
        }
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map’s key type,
    /// but the ordering on the borrowed form must match the ordering on the key type.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.key2slot
            .get(key)
            .map(|slot| &self.slots[slot.index()].value)
    }

    /// Gets an iterator over the entries of the map, sorted by key.
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            iter: self.slots.iter(),
        }
    }

    /// Gets a mutable iterator over the entries of the map, sorted by key.
    pub fn iter_mut(&mut self) -> IterMut<K, V> {
        IterMut {
            iter: self.slots.iter_mut(),
        }
    }

    /// Clears the map, removing all elements.
    pub fn clear(&mut self) {
        self.key2slot.clear();
        self.slots.clear();
    }
}

impl<'a, K, Q, V> Index<&'a Q> for IndexMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    type Output = V;

    fn index(&self, key: &'a Q) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}

impl<'a, K, V> Extend<(&'a K, &'a V)> for IndexMap<K, V>
where
    K: Ord + Copy,
    V: Copy,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (&'a K, &'a V)>,
    {
        self.extend(iter.into_iter().map(|(key, value)| (*key, *value)))
    }
}

impl<K, V> Extend<(K, V)> for IndexMap<K, V>
where
    K: Ord + Clone,
{
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (K, V)>,
    {
        iter.into_iter().for_each(move |(k, v)| {
            self.insert(k, v);
        });
    }
}

impl<K, V> FromIterator<(K, V)> for IndexMap<K, V>
where
    K: Ord + Clone,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        let mut map = IndexMap::new();
        map.extend(iter);
        map
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for IndexMap<K, V>
where
    K: Ord + Clone,
{
    fn from(items: [(K, V); N]) -> Self {
        items.into_iter().collect()
    }
}

impl<'a, K, V> IntoIterator for &'a IndexMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut IndexMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`iter`] method on [`IndexMap`]. See its
/// documentation for more.
///
/// [`iter`]: IndexMap::iter
#[derive(Debug, Clone)]
pub struct Iter<'a, K, V> {
    iter: SliceIter<'a, Slot<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Slot::as_pair)
    }
}

impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(Slot::as_pair)
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

/// A mutable iterator over the entries of an [`IndexMap`].
///
/// This `struct` is created by the [`iter_mut`] method on [`IndexMap`]. See its
/// documentation for more.
///
/// [`iter_mut`]: IndexMap::iter_mut
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    iter: SliceIterMut<'a, Slot<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(Slot::as_pair_mut)
    }
}

impl<'a, K, V> DoubleEndedIterator for IterMut<'a, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back().map(Slot::as_pair_mut)
    }
}

impl<'a, K, V> ExactSizeIterator for IterMut<'a, K, V> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'a, K, V> FusedIterator for IterMut<'a, K, V> {}

/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This `enum` is constructed from the [`entry`] method on [`IndexMap`].
///
/// [`entry`]: IndexMap::entry
pub enum Entry<'a, K, V> {
    /// A vacant entry.
    Vacant(VacantEntry<'a, K, V>),
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, K, V>),
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
    /// Ensures a value is in the entry by inserting the default if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert(self, default: V) -> &'a mut V
    where
        K: Clone,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result
    /// of the default function if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> &'a mut V
    where
        K: Clone,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting,
    /// if empty, the result of the default function.
    ///
    /// This method allows for generating key-derived values for
    /// insertion by providing the default function a reference
    /// to the key that was moved during the `.entry(key)` method call.
    ///
    /// The reference to the moved key is provided
    /// so that cloning or copying the key is
    /// unnecessary, unlike with `.or_insert_with(|| ... )`.
    pub fn or_insert_with_key<F: FnOnce(&K) -> V>(self, default: F) -> &'a mut V
    where
        K: Clone,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Returns a reference to this entry’s key.
    pub fn key(&self) -> &K {
        match *self {
            Self::Occupied(ref entry) => entry.key(),
            Self::Vacant(ref entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry
    /// before any potential inserts into the map.
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Self::Occupied(mut entry) => {
                f(entry.get_mut());
                Self::Occupied(entry)
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        }
    }
}

impl<'a, K, V> Entry<'a, K, V>
where
    K: Ord + Clone,
    V: Default,
{
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    pub fn or_default(self) -> &'a mut V {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

impl<'a, K, V> fmt::Debug for Entry<'a, K, V>
where
    K: fmt::Debug + Ord,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Entry::Vacant(entry) => entry.fmt(f),
            Entry::Occupied(entry) => entry.fmt(f),
        }
    }
}

/// A view into a vacant entry in an [`IndexMap`]. It is part of the [`Entry`] `enum`.
pub struct VacantEntry<'a, K, V> {
    /// The underlying vacant entry.
    vacant: btree_map::VacantEntry<'a, K, SlotIndex>,
    /// The vector that stores all slots.
    slots: &'a mut Vec<Slot<K, V>>,
}

impl<'a, K, V> VacantEntry<'a, K, V>
where
    K: Ord,
{
    /// Gets a reference to the key that would be used when inserting a value through the VacantEntry.
    pub fn key(&self) -> &K {
        self.vacant.key()
    }

    /// Take ownership of the key.
    pub fn into_key(self) -> K {
        self.vacant.into_key()
    }

    /// Sets the value of the entry with the `VacantEntry`’s key,
    /// and returns a mutable reference to it.
    pub fn insert(self, value: V) -> &'a mut V
    where
        K: Clone,
    {
        let index = self.slots.len();
        let key = self.vacant.key().clone();
        self.vacant.insert(SlotIndex(index));
        self.slots.push(Slot::new(key.clone(), value));
        &mut self.slots[index].value
    }
}

impl<'a, K, V> fmt::Debug for VacantEntry<'a, K, V>
where
    K: fmt::Debug + Ord,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VacantEntry")
            .field("key", self.key())
            .finish()
    }
}

/// A view into an occupied entry in a [`IndexMap`]. It is part of the [`Entry`] `enum`.
pub struct OccupiedEntry<'a, K, V> {
    /// The underlying occupied entry.
    occupied: btree_map::OccupiedEntry<'a, K, SlotIndex>,
    /// The vector that stores all slots.
    slots: &'a mut Vec<Slot<K, V>>,
}

impl<'a, K, V> OccupiedEntry<'a, K, V>
where
    K: Ord,
{
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &K {
        self.occupied.key()
    }

    /// Gets a reference to the value in the entry.
    pub fn get(&self) -> &V {
        let index = self.occupied.get().index();
        &self.slots[index].value
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// If you need a reference to the `OccupiedEntry` that may outlive the
    /// destruction of the `Entry` value, see [`into_mut`].
    ///
    /// [`into_mut`]: OccupiedEntry::into_mut
    pub fn get_mut(&mut self) -> &mut V {
        let index = self.occupied.get().index();
        &mut self.slots[index].value
    }

    /// Converts the entry into a mutable reference to its value.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: OccupiedEntry::get_mut
    pub fn into_mut(self) -> &'a mut V {
        let index = self.occupied.get().index();
        &mut self.slots[index].value
    }

    /// Sets the value of the entry with the `OccupiedEntry`’s key,
    /// and returns the entry’s old value.
    pub fn insert(&mut self, value: V) -> V
    where
        K: Clone,
    {
        let index = self.occupied.get().index();
        let key = self.key().clone();
        let new_slot = Slot::new(key, value);
        let old_slot = replace(&mut self.slots[index], new_slot);
        old_slot.value
    }
}

impl<'a, K, V> fmt::Debug for OccupiedEntry<'a, K, V>
where
    K: fmt::Debug + Ord,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OccupiedEntry")
            .field("key", self.key())
            .field("value", self.get())
            .finish()
    }
}
