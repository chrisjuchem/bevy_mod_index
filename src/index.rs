use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::system::{SystemChangeTick, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashSet;
use std::hash::Hash;

pub trait IndexInfo {
    type Component: Component;
    type Value: Send + Sync + Hash + Eq + Clone;

    fn value(c: &Self::Component) -> Self::Value;
}

#[derive(Resource)]
pub struct IndexStorage<I: IndexInfo> {
    map: UniqueMultiMap<I::Value, Entity>,
    last_refresh_tick: u32,
}
impl<I: IndexInfo> Default for IndexStorage<I> {
    fn default() -> Self {
        IndexStorage {
            map: Default::default(),
            last_refresh_tick: 0,
        }
    }
}

#[derive(SystemParam)]
pub struct Index<'w, 's, T: IndexInfo + 'static> {
    storage: ResMut<'w, IndexStorage<T>>,
    // TODO: Figure out if the static lifetime is right here
    changes: Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>,
    ticks: SystemChangeTick,
}

impl<'w, 's, T: IndexInfo> Index<'w, 's, T> {
    pub fn lookup(&mut self, val: &T::Value) -> HashSet<Entity> {
        self.refresh();
        self.storage.map.get(val)
    }

    pub fn refresh(&mut self) {
        if self.storage.last_refresh_tick >= self.ticks.change_tick() {
            return; // Already updated in this system.
        }

        for (entity, component, change_tracker) in &self.changes {
            if change_tracker
                .ticks()
                .is_changed(self.storage.last_refresh_tick, self.ticks.change_tick())
            {}
            self.storage.map.insert(&T::value(component), &entity);
        }
        self.storage.last_refresh_tick = self.ticks.change_tick();
    }
}
