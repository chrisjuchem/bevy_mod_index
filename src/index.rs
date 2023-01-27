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

pub struct FetchState<'w, 's, T: IndexInfo + 'static> {
    state: (
        <ResMut<'w, IndexStorage<T>> as bevy::ecs::system::SystemParam>::State,
        <Query<
            'w,
            's,
            (Entity, &'static T::Component, ChangeTrackers<T::Component>),
        > as bevy::ecs::system::SystemParam>::State,
        <SystemChangeTick as bevy::ecs::system::SystemParam>::State,
    ),
    marker: std::marker::PhantomData<
        (
            <bevy::ecs::prelude::Query<
                'w,
                's,
                (),
            > as bevy::ecs::system::SystemParam>::State,
        ),
    >,
}
unsafe impl<'w, 's, T: IndexInfo + 'static> bevy::ecs::system::SystemParam for Index<'w, 's, T> {
    type State = FetchState<'static, 'static, T>;
    type Item<'_w, '_s> = Index<'_w, '_s, T>;
    fn init_state(
        world: &mut bevy::ecs::world::World,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) -> Self::State {
        FetchState {
            state: <(
                ResMut<'w, IndexStorage<T>>,
                Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>,
                SystemChangeTick,
            ) as bevy::ecs::system::SystemParam>::init_state(world, system_meta),
            marker: std::marker::PhantomData,
        }
    }
    fn new_archetype(
        state: &mut Self::State,
        archetype: &bevy::ecs::archetype::Archetype,
        system_meta: &mut bevy::ecs::system::SystemMeta,
    ) {
        <(
            ResMut<'w, IndexStorage<T>>,
            Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>,
            SystemChangeTick,
        ) as bevy::ecs::system::SystemParam>::new_archetype(
            &mut state.state,
            archetype,
            system_meta,
        )
    }
    fn apply(
        state: &mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &mut bevy::ecs::world::World,
    ) {
        <(
            ResMut<'w, IndexStorage<T>>,
            Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>,
            SystemChangeTick,
        ) as bevy::ecs::system::SystemParam>::apply(&mut state.state, system_meta, world);
    }
    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &bevy::ecs::system::SystemMeta,
        world: &'w2 bevy::ecs::world::World,
        change_tick: u32,
    ) -> Self::Item<'w2, 's2> {
        let (f0, f1, f2) = <(
            ResMut<'w, IndexStorage<T>>,
            Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>,
            SystemChangeTick,
        ) as bevy::ecs::system::SystemParam>::get_param(
            &mut state.state,
            system_meta,
            world,
            change_tick,
        );
        Index {
            storage: f0,
            changes: f1,
            ticks: f2,
        }
    }
}
unsafe impl<'w, 's, T: IndexInfo + 'static> bevy::ecs::system::ReadOnlySystemParam
    for Index<'w, 's, T>
where
    ResMut<'w, IndexStorage<T>>: bevy::ecs::system::ReadOnlySystemParam,
    Query<'w, 's, (Entity, &'static T::Component, ChangeTrackers<T::Component>)>:
        bevy::ecs::system::ReadOnlySystemParam,
    SystemChangeTick: bevy::ecs::system::ReadOnlySystemParam,
{
}
