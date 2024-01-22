//! A crate that allows using indexes to efficiently query for components
//! by their values in the game engine Bevy.
//!
//! To use indexes, include the [`Index`][crate::index::Index]
//! [`SystemParam`](bevy::ecs::system::SystemParam) as an argument to your systems.
//! [`Index`][crate::index::Index] is generic over [`IndexInfo`][crate::index::IndexInfo], which is
//! a trait that you must implement on your own types to define the behavior of the index.

#![warn(missing_docs)]

/// Main index logic.
pub mod index;

/// Various types of storage for maintaining indexes.
pub mod storage;

/// Policy definitions and utilities for automatically refreshing indexes.
pub mod refresh_policy;

mod component_tuple;
mod unique_multimap;

/// Commonly used types.
pub mod prelude {
    pub use crate::index::{Index, IndexInfo};
    pub use crate::refresh_policy::*;
    pub use crate::storage::{HashmapStorage, IndexStorage, NoStorage};
}
