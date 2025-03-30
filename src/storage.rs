use crate::index::IndexInfo;
use crate::refresh_policy::IndexRefreshPolicy;
use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::component::Tick;
use bevy::ecs::system::{StaticSystemParam, SystemChangeTick, SystemParam};
use bevy::prelude::*;
use std::marker::PhantomData;

#[cfg(feature = "reflect")]
use bevy::reflect::Reflect;

/// Defines the internal storage for an index, which is stored as a [`Resource`].
///
/// You should not need this for normal use beyond including the `Storage` type
/// in your [`IndexInfo`] implementations, but you can use this to customize
/// the storage of your index's data if necessary
///
/// This crate provides the following storage implementations:
///
/// [`HashmapStorage`], [`NoStorage`]
pub trait IndexStorage<I: IndexInfo>: Resource + Default {
    /// [`SystemParam`] that is fetched alongside this storage [`Resource`] when
    /// an [`Index`][crate::index::Index] is included in a system.
    ///
    /// It is passed in when querying or updating the index.
    type RefreshData<'w, 's>: SystemParam;

    /// Get all of the entities with relevant components that evaluate to the given value
    /// using [`I::value`][`IndexInfo::value`].
    fn lookup<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> impl Iterator<Item = Entity>;

    /// Refresh this storage with the latest state from the world if it hasn't already been refreshed
    /// this [`Tick`].
    ///
    /// Note: 1 [`Tick`] = 1 system, not 1 frame.
    fn refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>);

    /// Unconditionally refresh this storage with the latest state from the world.
    fn force_refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>);

    /// Observer to be run whenever a component tracked by this Index is inserted.
    ///
    /// No observer will be registered if this returns `None`.
    fn insertion_observer() -> Option<Observer>;

    /// Observer to be run whenever a component tracked by this Index is removed.
    ///
    /// No observer will be registered if this returns `None`.
    fn removal_observer() -> Option<Observer>;
}

// ==================================================================

/// [`IndexStorage`] implementation that maintains a HashMap from values to [`Entity`]s whose
/// components have that value.
#[cfg_attr(feature = "reflect", derive(Reflect))]
#[cfg_attr(feature = "reflect", reflect(Resource))]
#[derive(Resource)]
pub struct HashmapStorage<I: IndexInfo> {
    map: UniqueMultiMap<I::Value, Entity>,
    last_refresh_tick: Tick,
    removed_entities: Vec<Entity>,
}

impl<I: IndexInfo> Default for HashmapStorage<I> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            last_refresh_tick: Tick::new(0),
            removed_entities: Vec::with_capacity(16),
        }
    }
}

impl<I: IndexInfo> IndexStorage<I> for HashmapStorage<I> {
    type RefreshData<'w, 's> = HashmapStorageRefreshData<'w, 's, I>;

    fn lookup<'w, 's>(
        &mut self,
        val: &I::Value,
        _data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> impl Iterator<Item = Entity> {
        self.map.get(val).copied()
    }

    fn refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {
        if self.last_refresh_tick != data.ticks.this_run() {
            self.force_refresh(data);
        }
    }

    fn force_refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {
        for entity in self.removed_entities.iter() {
            self.map.remove(entity);
        }
        self.removed_entities.clear();
        for (entity, component) in &data.components {
            if component.last_changed().is_newer_than(
                // Subtract 1 so that changes from the system where the index was updated are seen.
                // The `is_newer_than` implementation assumes we don't care about those changes since
                // "this" system is the one that made the change, but for indexing, we do care.
                Tick::new(self.last_refresh_tick.get().wrapping_sub(1)),
                data.ticks.this_run(),
            ) {
                self.map.insert(&I::value(&component), entity);
            }
        }
        self.last_refresh_tick = data.ticks.this_run();
    }

    fn insertion_observer() -> Option<Observer> {
        if I::REFRESH_POLICY == IndexRefreshPolicy::WhenInserted {
            Some(Observer::new(
                |trigger: Trigger<OnInsert, I::Component>,
                 mut storage: ResMut<HashmapStorage<I>>,
                 components: Query<&I::Component>| {
                    let target = trigger.target();
                    let component = components
                        .get(target)
                        .expect("Component that was just inserted is missing!");

                    println!("INSERTION");
                    storage.map.insert(&I::value(component), target);
                },
            ))
        } else {
            None
        }
    }

    fn removal_observer() -> Option<Observer> {
        Some(Observer::new(
            |trigger: Trigger<OnRemove, I::Component>, mut storage: ResMut<HashmapStorage<I>>| {
                if I::REFRESH_POLICY.is_when_inserted() {
                    storage.map.remove(&trigger.target());
                } else {
                    storage.removed_entities.push(trigger.target());
                }
            },
        ))
    }
}

type ComponentsQuery<'w, 's, T> =
    Query<'w, 's, (Entity, Ref<'static, <T as IndexInfo>::Component>)>;

#[doc(hidden)]
#[derive(SystemParam)]
pub struct HashmapStorageRefreshData<'w, 's, I: IndexInfo> {
    components: ComponentsQuery<'w, 's, I>,
    ticks: SystemChangeTick,
}

//======================================================================

/// [`IndexStorage`] implementation that doesn't actually store anything.
///
/// Whenever it is queried, it iterates over all components like you naively would if you weren't
/// using an index. This allows you to use the `Index` interface without actually using any extra
/// memory.
///
/// This storage never needs to be refreshed, so the [`Manual`](IndexRefreshPolicy::Manual) refresh
/// policy is usually the best choice for index definitions that use `NoStorage`.
#[derive(Resource)]
#[cfg_attr(feature = "reflect", derive(Reflect))]
#[cfg_attr(feature = "reflect", reflect(Resource))]
pub struct NoStorage<I: IndexInfo> {
    #[cfg_attr(feature = "reflect", reflect(ignore))]
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

    fn lookup<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> impl Iterator<Item = Entity> {
        data.iter()
            .filter_map(|(e, c)| if I::value(c) == *val { Some(e) } else { None })
    }

    fn refresh<'w, 's>(&mut self, _data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {}

    fn force_refresh<'w, 's>(&mut self, _data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>) {}

    fn insertion_observer() -> Option<Observer> {
        None
    }

    fn removal_observer() -> Option<Observer> {
        None
    }
}
