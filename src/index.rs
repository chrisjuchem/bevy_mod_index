use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use std::hash::Hash;

pub trait IndexInfo {
    type Component: Component;
    type Value: Send + Sync + Hash + Eq + Clone;

    fn value(c: &Self::Component) -> Self::Value;
}

#[derive(Resource)]
pub struct IndexStorage<I: IndexInfo> {
    map: UniqueMultiMap<I::Value, Entity>,
}
impl<I: IndexInfo> Default for IndexStorage<I> {
    fn default() -> Self {
        IndexStorage {
            map: Default::default(),
        }
    }
}

#[derive(SystemParam)]
pub struct Index<'w, 's, T: IndexInfo + 'static> {
    storage: ResMut<'w, IndexStorage<T>>,
    // TODO: Figure out if the static lifetime is right here
    adds: Query<'w, 's, (Entity, &'static T::Component), Added<T::Component>>,
    //Todo: strictly changes and not adds
    changes: Query<'w, 's, (Entity, &'static T::Component), Changed<T::Component>>,
}

impl<'w, 's, T: IndexInfo> Index<'w, 's, T> {
    pub fn lookup(&mut self, val: &T::Value) -> HashSet<Entity> {
        //todo: if we dont refresh every frame, we lose data????
        self.refresh();
        self.storage.map.get(val)
    }

    pub fn refresh(&mut self) {
        for (e, c) in &self.adds {
            self.storage.map.insert(&T::value(c), &e);
        }

        for (e, c) in &self.changes {
            self.storage.map.insert(&T::value(c), &e);
        }
    }
}
