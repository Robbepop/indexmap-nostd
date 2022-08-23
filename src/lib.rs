#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std as alloc;

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod map;

pub use self::map::IndexMap;
