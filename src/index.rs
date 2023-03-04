use crate::storage::IndexStorage;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::system::{ReadOnlySystemParam, StaticSystemParam, SystemMeta, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashSet;
use std::hash::Hash;

/// Implement this trait on your own types to specify how an index should behave.
///
/// If there is a single canonical way to index a [`Component`], you can implement this
/// for that component directly. Otherwise, it is recommended to implement this for a
/// unit struct/enum.
pub trait IndexInfo: Sized + 'static {
    /// The type of component to be indexed.
    type Component: Component;
    /// The type of value to be used when looking up components.
    type Value: Send + Sync + Hash + Eq + Clone;
    /// The type of storage to use for the index.
    type Storage: IndexStorage<Self>;

    /// The function used by [`Index::lookup`] to determine the value of a component.
    ///
    /// The values returned by this function are typically cached by the storage, so
    /// this should always return the same value given equal Components.
    fn value(c: &Self::Component) -> Self::Value;
}

/// A [`SystemParam`] that allows you to lookup [`Component`]s that match a certain value.
pub struct Index<'w, 's, T: IndexInfo + 'static> {
    storage: ResMut<'w, T::Storage>,
    refresh_data:
        StaticSystemParam<'w, 's, <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>>,
}

// todo impl deref instead? need to move storage?
impl<'w, 's, T: IndexInfo> Index<'w, 's, T> {
    /// Get all of the entities with relevant components that evaluate to the given value
    /// using [`T::value`][`IndexInfo::value`].
    pub fn lookup(&mut self, val: &T::Value) -> HashSet<Entity> {
        self.storage.get(val, &mut self.refresh_data)
    }

    /// Refresh the underlying [`IndexStorage`] for this index.
    ///
    /// This may or may not be necessary to call manually depending on the particular [`IndexStorage`] used.
    pub fn refresh(&mut self) {
        self.storage.refresh(&mut self.refresh_data)
    }
}

#[doc(hidden)]
pub struct IndexFetchState<'w, 's, T: IndexInfo + 'static> {
    storage_state: <ResMut<'w, T::Storage> as SystemParam>::State,
    refresh_data_state: <StaticSystemParam<
        'w,
        's,
        <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>,
    > as SystemParam>::State,
}
unsafe impl<'w, 's, T> SystemParam for Index<'w, 's, T>
where
    T: IndexInfo + 'static,
{
    type State = IndexFetchState<'static, 'static, T>;
    type Item<'_w, '_s> = Index<'_w, '_s, T>;
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        world.init_resource::<T::Storage>();
        IndexFetchState {
            storage_state: <ResMut<'w, T::Storage> as SystemParam>::init_state(world, system_meta),
            refresh_data_state: <StaticSystemParam<
                'w,
                's,
                <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>,
            > as SystemParam>::init_state(world, system_meta),
        }
    }
    fn new_archetype(state: &mut Self::State, archetype: &Archetype, system_meta: &mut SystemMeta) {
        <ResMut<'w, T::Storage> as SystemParam>::new_archetype(
            &mut state.storage_state,
            archetype,
            system_meta,
        );
        <StaticSystemParam<'w, 's, <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>> as SystemParam>::new_archetype(
            &mut state.refresh_data_state,
            archetype,
            system_meta,
        );
    }
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <ResMut<'w, T::Storage> as SystemParam>::apply(
            &mut state.storage_state,
            system_meta,
            world,
        );
        <StaticSystemParam<'w, 's, <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>> as SystemParam>::apply(
            &mut state.refresh_data_state,
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
            storage: <ResMut<'w, T::Storage>>::get_param(
                &mut state.storage_state,
                system_meta,
                world,
                change_tick,
            ),
            refresh_data: <StaticSystemParam<
                'w,
                's,
                <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>,
            > as SystemParam>::get_param(
                &mut state.refresh_data_state,
                system_meta,
                world,
                change_tick,
            ),
        }
    }
}
unsafe impl<'w, 's, T: IndexInfo + 'static> ReadOnlySystemParam for Index<'w, 's, T>
where
    ResMut<'w, T::Storage>: ReadOnlySystemParam,
    StaticSystemParam<'w, 's, <T::Storage as IndexStorage<T>>::RefreshData<'static, 'static>>:
        ReadOnlySystemParam,
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
        type Storage = HashmapStorage<Self>;

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
