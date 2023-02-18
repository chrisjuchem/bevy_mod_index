pub mod index;
pub mod storage;
mod unique_multimap;

pub mod prelude {
    pub use crate::index::{Index, IndexInfo};
    pub use crate::storage::{HashmapStorage, IndexStorage};
}
