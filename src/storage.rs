use crate::index::IndexInfo;
use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::component::Tick;
use bevy::ecs::system::{StaticSystemParam, SystemChangeTick, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashSet;
use std::marker::PhantomData;

/// Defines the internal storage for an index, which is stored as a [`Resource`].
///
/// You should not need this for normal use beyond including the `Storage` type
/// in your [`IndexInfo`] implementations, but you can use this to customize
/// the storage of your index's data if necessary
///
/// This crate provides the following storage implementations:
///
/// | Feature | [`HashmapStorage`] | [`NoStorage`] |
/// |---|---|---|
/// | Automatic refresh timing | When used; once per-system run | N/A - it always reads the world data directly |
/// | Sees updates from earlier in the system? | After manually calling [`refresh`][Self::refresh] | Yes |
/// | Sees updates from earlier in the frame? | Yes | Yes |
pub trait IndexStorage<I: IndexInfo>: Resource + Default {
    /// [`SystemParam`] that is fetched alongside this storage [`Resource`] when
    /// an [`Index`][crate::index::Index] is included in a system.
    ///
    /// It is passed in when querying or updating the index.
    type RefreshData<'w, 's>: SystemParam;

    /// Get all of the entities with relevant components that evaluate to the given value
    /// using [`T::value`][`IndexInfo::value`].
    fn get<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> HashSet<Entity>;

    /// Refresh this storage with the latest state from the world.
    fn refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>);
}

// ==================================================================

/// [`IndexStorage`] implementation that maintains a mapping from values to components
/// which have that value.
///
/// | Feature | `HashmapStorage` |
/// |---|---|
/// | Automatic refresh timing | When used; once per-system run |
/// | Sees updates from earlier in the system? | After manually calling [`refresh`][Self::refresh] |
/// | Sees updates from earlier in the frame? | Yes |
#[derive(Resource)]
pub struct HashmapStorage<I: IndexInfo> {
    map: UniqueMultiMap<I::Value, Entity>,
    last_refresh_tick: u32,
}

impl<I: IndexInfo> Default for HashmapStorage<I> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            last_refresh_tick: 0,
        }
    }
}

impl<I: IndexInfo> IndexStorage<I> for HashmapStorage<I> {
    type RefreshData<'w, 's> = HashmapStorageRefreshData<'w, 's, I>;

    fn get<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> HashSet<Entity> {
        if self.last_refresh_tick != data.ticks.change_tick() {
            self.refresh(data);
        }
        self.map.get(val)
    }

    fn refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {
        for entity in data.removals.iter() {
            self.map.remove(&entity);
        }
        for (entity, component) in &data.components {
            if Tick::new(component.last_changed()).is_newer_than(
                // Subtract 1 so that changes from the system where the index was updated are seen.
                // The `changed` implementation assumes we don't care about those changes since
                // "this" system is the one that made the change, but for indexing, we do care.
                self.last_refresh_tick.wrapping_sub(1),
                data.ticks.change_tick(),
            ) {
                self.map.insert(&I::value(&component), &entity);
            }
        }
        self.last_refresh_tick = data.ticks.change_tick();
    }
}

type ComponentsQuery<'w, 's, T> =
    Query<'w, 's, (Entity, Ref<'static, <T as IndexInfo>::Component>)>;

#[doc(hidden)]
#[derive(SystemParam)]
pub struct HashmapStorageRefreshData<'w, 's, I: IndexInfo> {
    components: ComponentsQuery<'w, 's, I>,
    removals: RemovedComponents<'w, 's, I::Component>,
    ticks: SystemChangeTick,
}

//======================================================================

/// [`IndexStorage`] implementation that doesn't actually store anything.
///
/// Whenever it is queried, it iterates over all components like you would if you weren't
/// using an index.
///
/// | Feature | `NoStorage` |
/// |---|---|
/// | Automatic refresh timing | N/A - it always reads the world data directly |
/// | Sees updates from earlier in the system? | Yes |
/// | Sees updates from earlier in the frame? | Yes |
#[derive(Resource)]
pub struct NoStorage<I: IndexInfo> {
    phantom: PhantomData<fn() -> I>,
}
impl<I: IndexInfo> Default for NoStorage<I> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<I: IndexInfo> IndexStorage<I> for NoStorage<I> {
    type RefreshData<'w, 's> = Query<'w, 's, (Entity, &'static I::Component)>;

    fn get<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> HashSet<Entity> {
        data.iter()
            .filter_map(|(e, c)| if I::value(c) == *val { Some(e) } else { None })
            .collect()
    }

    fn refresh<'w, 's>(&mut self, _data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {}
}
