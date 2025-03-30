use crate::index::{Index, IndexInfo};

#[derive(Copy, Clone, Eq, PartialEq)]
/// Defines when an [`Index`] should be automatically refreshed.
///
/// Refreshing an [`Index`] is required for it to be able to accurately reflect the status of
/// [`Component`][bevy::ecs::component::Component]s as they are added, changed, and removed.
pub enum IndexRefreshPolicy {
    /// Refresh the index whenever a system with an [`Index`] argument is run.
    ///
    /// This is a good default for most use cases.
    WhenRun,
    /// Refresh the index the first time [`lookup`][crate::index::Index::lookup] is called in each system.
    ///
    /// Compared to `WhenRun`, this requires an extra check in each [`lookup`][crate::index::Index::lookup]
    /// to see if the index needs to be refreshed or not, but saves the overhead of an entire refresh
    /// when [`lookup`][crate::index::Index::lookup] is never called.
    WhenUsed,
    /// Refresh the index once during the [`First`][bevy::app::First]
    /// [`Schedule`][bevy::ecs::schedule::Schedule].
    ///
    /// To refresh during a different schedule, you should use the [`Manual`][`IndexRefreshPolicy::Manual`]
    /// refresh policy and manually add the [`refresh_index_system`] to the desired schedule.
    EachFrame,
    /// Use [`Observers`][bevy::ecs::observer::Observer] to refresh the index on a per-entity basis
    /// as components are inserted and removed.
    ///
    /// This is best used with [`Immutable`][bevy::ecs::component::Immutable] components, as otherwise,
    /// component mutations will be missed unless you refresh the index manually.
    WhenInserted,
    /// Never refresh the [`Index`] automatically.
    ///
    /// You must call [`refresh`][crate::index::Index::refresh] manually if any components are
    /// changed or removed.
    Manual,
}

/// A [`System`][bevy::ecs::system::System] that refreshes the index every frame.
///
/// This system can be useful to ensure that all removed entities are reflected properly
/// by the index. It is automatically added to the app for each index with its
/// [`REFRESH_POLICY`][`IndexInfo::REFRESH_POLICY`] set to [`EachFrame`][`IndexRefreshPolicy::EachFrame`]
pub fn refresh_index_system<I: IndexInfo>(mut idx: Index<I>) {
    idx.refresh();
}
