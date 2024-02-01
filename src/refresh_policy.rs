use crate::index::{Index, IndexInfo};
use bevy::app::First;
use bevy::ecs::schedule::ScheduleLabel;

/// Defines how [`Index`]es are automatically refreshed.
///
/// This crate provides the following implementations:
///
/// [`SimpleRefreshPolicy`], [`ConservativeRefreshPolicy`], [`NoDespawnRefreshPolicy`],
/// [`SnapshotRefreshPolicy`], [`ManualRefreshPolicy`]
pub trait IndexRefreshPolicy {
    /// Refresh the index the first time [`lookup`][crate::index::Index::lookup] is called in each system.
    ///
    /// If components change infrequently, and the index is used many times in a single frame, this
    /// many refreshes may not be necessary.
    ///
    /// If `lookup` is not called every frame, component removals may be missed if no
    /// other refresh mechanisms are used.
    const REFRESH_WHEN_USED: bool;
    /// Refresh the index whenever a system with an [`Index`] argument is run.
    ///
    /// This is like `REFRESH_WHEN_USED` but does not require explicitly calling [`lookup`][crate::index::Index::lookup].
    ///
    /// `REFRESH_WHEN_USED` becomes a no-op if this mechanism is used.
    const REFRESH_WHEN_RUN: bool;
    /// Automatically adds a system that refreshes the index once per frame.
    ///
    /// This could add unnecessary refreshes if the index is already refreshing frequently, but
    /// it may be required to ensure that removed components are not missed, since they are only
    /// buffered for 2 frames.
    const REFRESH_EVERY_FRAME: bool;

    /// The [`Schedule`][bevy::ecs::schedule::Schedule] to refresh the index in when
    /// `REFRESH_EVERY_FRAME` is true. Defaults to [`First`].
    fn schedule() -> impl ScheduleLabel + Clone {
        First
    }
}

/// Refresh policy that requires at least one system using the Index to run each frame.
pub struct SimpleRefreshPolicy;
impl IndexRefreshPolicy for SimpleRefreshPolicy {
    const REFRESH_WHEN_USED: bool = false;
    const REFRESH_WHEN_RUN: bool = true;
    const REFRESH_EVERY_FRAME: bool = false;
}

/// Refresh policy insuring that removals are never missed and that data is always up-to-date.
///
/// There may be more performant policies depending on your use case, but this policy is a good
/// choice if the systems using your index do not run every frame due to run conditions.
pub struct ConservativeRefreshPolicy;
impl IndexRefreshPolicy for ConservativeRefreshPolicy {
    const REFRESH_WHEN_USED: bool = true;
    const REFRESH_WHEN_RUN: bool = false;
    const REFRESH_EVERY_FRAME: bool = true;
}

/// A more performant refresh policy when the [`Component`](bevy::ecs::component::Component)s in
/// the index are never despawned.
///
/// This policy can also be used if the index is used or refreshed manually on the same frame
/// that the despawn happens.
pub struct NoDespawnRefreshPolicy;
impl IndexRefreshPolicy for NoDespawnRefreshPolicy {
    const REFRESH_WHEN_USED: bool = true;
    const REFRESH_WHEN_RUN: bool = false;
    const REFRESH_EVERY_FRAME: bool = false;
}

/// Maximum performance refresh policy for high-use [`Index`]es that sacrifices accuracy
/// for changing [`Component`](bevy::ecs::component::Component)s.
///
/// Refreshes once at the beginning of each frame and does not update until the
/// next frame even if components change.
pub struct SnapshotRefreshPolicy;
impl IndexRefreshPolicy for SnapshotRefreshPolicy {
    const REFRESH_WHEN_USED: bool = false;
    const REFRESH_WHEN_RUN: bool = false;
    const REFRESH_EVERY_FRAME: bool = true;
}

/// An [`Index`] with this refresh policy is never refreshed automatically.
///
/// You must call [`refresh`][crate::index::Index::refresh] manually if any components change.
pub struct ManualRefreshPolicy;
impl IndexRefreshPolicy for ManualRefreshPolicy {
    const REFRESH_WHEN_USED: bool = false;
    const REFRESH_WHEN_RUN: bool = false;
    const REFRESH_EVERY_FRAME: bool = false;
}

/// A [`System`][bevy::ecs::system::System] that refreshes the index every frame.
///
/// This system can be useful to ensure that all removed entities are reflected properly
/// by the index. It is automatically added to the app for each index with a [`IndexRefreshPolicy`]
/// that has `REFRESH_EVERY_FRAME` set to true.
pub fn refresh_index_system<I: IndexInfo>(mut idx: Index<I>) {
    idx.refresh();
}
