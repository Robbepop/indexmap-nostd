#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std as alloc;

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod map;
pub mod set;

pub use self::map::IndexMap;
pub use self::set::IndexSet;

/// A slot index referencing a [`Slot`] in an [`IndexMap`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SlotIndex(usize);

impl SlotIndex {
    /// Returns the raw `usize` index of the [`SlotIndex`].
    pub fn index(self) -> usize {
        self.0
    }
}
