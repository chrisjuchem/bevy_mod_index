use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::system::{ReadOnlySystemParam, SystemChangeTick, SystemMeta, SystemParam};
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

type ChangedComponetsQuery<'w, 's, T> = Query<
    'w,
    's,
    (
        Entity,
        // TODO: Figure out if the static lifetime is right here
        &'static <T as IndexInfo>::Component,
        ChangeTrackers<<T as IndexInfo>::Component>,
    ),
>;

pub struct Index<'w, 's, T: IndexInfo + 'static> {
    storage: ResMut<'w, IndexStorage<T>>,
    changes: ChangedComponetsQuery<'w, 's, T>,
    current_tick: u32,
}

impl<'w, 's, T: IndexInfo> Index<'w, 's, T> {
    pub fn lookup(&mut self, val: &T::Value) -> HashSet<Entity> {
        self.refresh();
        self.storage.map.get(val)
    }

    pub fn refresh(&mut self) {
        if self.storage.last_refresh_tick >= self.current_tick {
            return; // Already updated in this system.
        }

        for (entity, component, change_tracker) in &self.changes {
            if change_tracker
                .ticks()
                .is_changed(self.storage.last_refresh_tick, self.current_tick)
            {}
            self.storage.map.insert(&T::value(component), &entity);
        }
        self.storage.last_refresh_tick = self.current_tick;
    }
}

pub struct IndexFetchState<'w, 's, T: IndexInfo + 'static> {
    storage_state: <ResMut<'w, IndexStorage<T>> as SystemParam>::State,
    changed_components_state: <ChangedComponetsQuery<'w, 's, T> as SystemParam>::State,
}
unsafe impl<'w, 's, T: IndexInfo + 'static> SystemParam for Index<'w, 's, T> {
    type State = IndexFetchState<'static, 'static, T>;
    type Item<'_w, '_s> = Index<'_w, '_s, T>;
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        world.init_resource::<IndexStorage<T>>();
        IndexFetchState {
            storage_state: <ResMut<'w, IndexStorage<T>> as SystemParam>::init_state(
                world,
                system_meta,
            ),
            changed_components_state: <ChangedComponetsQuery<'w, 's, T> as SystemParam>::init_state(
                world,
                system_meta,
            ),
        }
    }
    fn new_archetype(state: &mut Self::State, archetype: &Archetype, system_meta: &mut SystemMeta) {
        <ResMut<'w, IndexStorage<T>> as SystemParam>::new_archetype(
            &mut state.storage_state,
            archetype,
            system_meta,
        );
        <ChangedComponetsQuery<'w, 's, T> as SystemParam>::new_archetype(
            &mut state.changed_components_state,
            archetype,
            system_meta,
        );
    }
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <ResMut<'w, IndexStorage<T>> as SystemParam>::apply(
            &mut state.storage_state,
            system_meta,
            world,
        );
        <ChangedComponetsQuery<'w, 's, T> as SystemParam>::apply(
            &mut state.changed_components_state,
            system_meta,
            world,
        );
    }
    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &SystemMeta,
        world: &'w2 World,
        change_tick: u32,
    ) -> Self::Item<'w2, 's2> {
        Index {
            storage: <ResMut<'w, IndexStorage<T>>>::get_param(
                &mut state.storage_state,
                system_meta,
                world,
                change_tick,
            ),
            changes: <ChangedComponetsQuery<'w, 's, T> as SystemParam>::get_param(
                &mut state.changed_components_state,
                system_meta,
                world,
                change_tick,
            ),
            current_tick: change_tick,
        }
    }
}
unsafe impl<'w, 's, T: IndexInfo + 'static> ReadOnlySystemParam for Index<'w, 's, T>
where
    ResMut<'w, IndexStorage<T>>: ReadOnlySystemParam,
    ChangedComponetsQuery<'w, 's, T>: ReadOnlySystemParam,
{
}
