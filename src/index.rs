use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::change_detection::Ref;
use bevy::ecs::component::Tick;
use bevy::ecs::system::{ReadOnlySystemParam, SystemMeta, SystemParam};
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

type ComponetsQuery<'w, 's, T> = Query<'w, 's, (Entity, Ref<'static, <T as IndexInfo>::Component>)>;

pub struct Index<'w, 's, T: IndexInfo + 'static> {
    storage: ResMut<'w, IndexStorage<T>>,
    components: ComponetsQuery<'w, 's, T>,
    removals: RemovedComponents<'w, 's, T::Component>,
    current_tick: u32,
}

impl<'w, 's, T: IndexInfo> Index<'w, 's, T> {
    pub fn lookup(&mut self, val: &T::Value) -> HashSet<Entity> {
        if self.storage.last_refresh_tick != self.current_tick {
            self.refresh();
        }

        self.storage.map.get(val)
    }

    pub fn refresh(&mut self) {
        for entity in self.removals.iter() {
            self.storage.map.remove(&entity);
        }
        for (entity, component) in &self.components {
            // Subtract 1 so that changes from the system where the index was updated are seen.
            // The `changed` implementation assumes we don't care about those changes since
            // "this" system is the one that made the change, but for indexing, we do care.
            if Tick::new(self.storage.last_refresh_tick.wrapping_sub(1))
                .is_older_than(component.last_changed(), self.current_tick)
            {
                self.storage.map.insert(&T::value(&component), &entity);
            }
        }
        self.storage.last_refresh_tick = self.current_tick;
    }
}

pub struct IndexFetchState<'w, 's, T: IndexInfo + 'static> {
    storage_state: <ResMut<'w, IndexStorage<T>> as SystemParam>::State,
    changed_components_state: <ComponetsQuery<'w, 's, T> as SystemParam>::State,
    removed_components_state: <RemovedComponents<'w, 's, T::Component> as SystemParam>::State,
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
            changed_components_state: <ComponetsQuery<'w, 's, T> as SystemParam>::init_state(
                world,
                system_meta,
            ),
            removed_components_state:
                <RemovedComponents<'w, 's, T::Component> as SystemParam>::init_state(
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
        <ComponetsQuery<'w, 's, T> as SystemParam>::new_archetype(
            &mut state.changed_components_state,
            archetype,
            system_meta,
        );
        <RemovedComponents<'w, 's, T::Component> as SystemParam>::new_archetype(
            &mut state.removed_components_state,
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
        <ComponetsQuery<'w, 's, T> as SystemParam>::apply(
            &mut state.changed_components_state,
            system_meta,
            world,
        );
        <RemovedComponents<'w, 's, T::Component> as SystemParam>::apply(
            &mut state.removed_components_state,
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
            components: <ComponetsQuery<'w, 's, T> as SystemParam>::get_param(
                &mut state.changed_components_state,
                system_meta,
                world,
                change_tick,
            ),
            removals: <RemovedComponents<'w, 's, T::Component> as SystemParam>::get_param(
                &mut state.removed_components_state,
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
    ComponetsQuery<'w, 's, T>: ReadOnlySystemParam,
{
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use bevy::prelude::*;

    #[derive(Component, Clone, Eq, Hash, PartialEq, Debug)]
    struct Number(usize);

    //todo: maybe make this a derive macro
    impl IndexInfo for Number {
        type Component = Self;
        type Value = Self;

        fn value(c: &Self::Component) -> Self::Value {
            c.clone()
        }
    }

    fn add_some_numbers(mut commands: Commands) {
        commands.spawn(Number(10));
        commands.spawn(Number(10));
        commands.spawn(Number(20));
        commands.spawn(Number(30));
    }

    fn checker(number: usize, amount: usize) -> impl Fn(Index<Number>) {
        move |mut idx: Index<Number>| {
            let set = idx.lookup(&Number(number));
            assert_eq!(
                set.len(),
                amount,
                "Index returned {} matches for {}, expectd {}.",
                set.len(),
                number,
                amount,
            );
        }
    }

    fn adder_all(n: usize) -> impl Fn(Query<&mut Number>) {
        move |mut nums: Query<&mut Number>| {
            for mut num in &mut nums {
                num.0 += n;
            }
        }
    }

    fn adder_some(
        n: usize,
        condition: usize,
    ) -> impl Fn(ParamSet<(Query<&mut Number>, Index<Number>)>) {
        move |mut nums_and_index: ParamSet<(Query<&mut Number>, Index<Number>)>| {
            for entity in nums_and_index.p1().lookup(&Number(condition)).into_iter() {
                let mut nums = nums_and_index.p0();
                let mut nref: Mut<Number> = nums.get_mut(entity).unwrap();
                nref.0 += n;
            }
        }
    }

    #[test]
    fn test_index_lookup() {
        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(checker(10, 2))
            .add_system(checker(20, 1))
            .add_system(checker(30, 1))
            .add_system(checker(40, 0))
            .run();
    }

    #[test]
    fn test_changing_values() {
        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(checker(10, 2).in_base_set(CoreSet::PreUpdate))
            .add_system(checker(20, 1).in_base_set(CoreSet::PreUpdate))
            .add_system(checker(30, 1).in_base_set(CoreSet::PreUpdate))
            .add_system(adder_all(5))
            .add_system(checker(10, 0).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(20, 0).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(30, 0).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(15, 2).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(25, 1).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(35, 1).in_base_set(CoreSet::PostUpdate))
            .run();
    }

    #[test]
    fn test_changing_with_index() {
        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(checker(10, 2).in_base_set(CoreSet::PreUpdate))
            .add_system(checker(20, 1).in_base_set(CoreSet::PreUpdate))
            .add_system(adder_some(10, 10))
            .add_system(checker(10, 0).in_base_set(CoreSet::PostUpdate))
            .add_system(checker(20, 3).in_base_set(CoreSet::PostUpdate))
            .run();
    }

    #[test]
    fn test_same_system_detection() {
        let manual_refresh_system =
            |mut nums_and_index: ParamSet<(Query<&mut Number>, Index<Number>)>| {
                let mut idx = nums_and_index.p1();
                let twenties = idx.lookup(&Number(20));
                assert_eq!(twenties.len(), 1);

                for entity in twenties.into_iter() {
                    nums_and_index.p0().get_mut(entity).unwrap().0 += 5;
                }
                idx = nums_and_index.p1(); // reborrow here so earlier p0 borrow succeeds

                // Hasn't refreshed yet
                assert_eq!(idx.lookup(&Number(20)).len(), 1);
                assert_eq!(idx.lookup(&Number(25)).len(), 0);

                idx.refresh();
                assert_eq!(idx.lookup(&Number(20)).len(), 0);
                assert_eq!(idx.lookup(&Number(25)).len(), 1);
            };

        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(manual_refresh_system)
            .run();
    }

    fn remover(n: usize) -> impl Fn(Index<Number>, Commands) {
        move |mut idx: Index<Number>, mut commands: Commands| {
            for entity in idx.lookup(&Number(n)).into_iter() {
                commands.get_entity(entity).unwrap().remove::<Number>();
            }
        }
    }

    fn despawner(n: usize) -> impl Fn(Index<Number>, Commands) {
        move |mut idx: Index<Number>, mut commands: Commands| {
            for entity in idx.lookup(&Number(n)).into_iter() {
                commands.get_entity(entity).unwrap().despawn();
            }
        }
    }

    fn next_frame(world: &mut World) {
        world.clear_trackers();
    }

    #[test]
    fn test_removal_detection() {
        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(checker(20, 1).in_base_set(CoreSet::PreUpdate))
            .add_system(remover(20).in_base_set(CoreSet::Update))
            .add_system(next_frame.in_base_set(CoreSet::PostUpdate))
            .add_system(
                remover(30)
                    .after(next_frame)
                    .in_base_set(CoreSet::PostUpdate),
            )
            // Detect component removed this earlier this frame
            .add_system(checker(30, 0).in_base_set(CoreSet::Last))
            // Detect component removed after we ran last stage
            .add_system(checker(20, 0).in_base_set(CoreSet::Last))
            .run();
    }

    #[test]
    fn test_despawn_detection() {
        App::new()
            .add_startup_system(add_some_numbers)
            .add_system(checker(20, 1).in_base_set(CoreSet::PreUpdate))
            .add_system(despawner(20).in_base_set(CoreSet::Update))
            .add_system(next_frame.in_base_set(CoreSet::PostUpdate))
            .add_system(
                despawner(30)
                    .after(next_frame)
                    .in_base_set(CoreSet::PostUpdate),
            )
            // Detect component removed this earlier this frame
            .add_system(checker(30, 0).in_base_set(CoreSet::Last))
            // Detect component removed after we ran last stage
            .add_system(checker(20, 0).in_base_set(CoreSet::Last))
            .run();
    }
}
