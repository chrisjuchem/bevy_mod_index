use crate::refresh_policy::{refresh_index_system, IndexRefreshPolicy};
use crate::storage::IndexStorage;
use bevy::ecs::archetype::Archetype;
use bevy::ecs::component::Tick;
use bevy::ecs::system::{ReadOnlySystemParam, StaticSystemParam, SystemMeta, SystemParam};
use bevy::ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy::prelude::*;
use bevy::utils::HashSet;
use std::hash::Hash;

/// Implement this trait on your own types to specify how an [`Index`] should behave.
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
    /// The [`IndexRefreshPolicy`] to use to automatically refresh the index.
    type RefreshPolicy: IndexRefreshPolicy;

    /// The function used by [`Index::lookup`] to determine the value of a component.
    ///
    /// The values returned by this function are typically cached by the storage, so
    /// this should always return the same value given equal [`Component`]s.
    fn value(c: &Self::Component) -> Self::Value;
}

/// A [`SystemParam`] that allows you to lookup [`Component`]s that match a certain value.
pub struct Index<'w, 's, I: IndexInfo + 'static> {
    storage: ResMut<'w, I::Storage>,
    refresh_data:
        StaticSystemParam<'w, 's, <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>>,
}

// todo impl deref instead? need to move storage?
impl<'w, 's, I: IndexInfo> Index<'w, 's, I> {
    /// Get all of the entities with relevant components that evaluate to the given value
    /// using [`I::value`][`IndexInfo::value`].
    pub fn lookup(&mut self, val: &I::Value) -> HashSet<Entity> {
        self.storage.lookup(val, &mut self.refresh_data)
    }

    /// Refresh the underlying [`IndexStorage`] for this index if it hasn't already been refreshed
    /// this [`Tick`].
    ///
    /// Note: 1 [`Tick`] = 1 system, not 1 frame.
    ///
    /// This may or may not be necessary to call manually depending on the particular [`IndexRefreshPolicy`] used.
    pub fn refresh(&mut self) {
        self.storage.refresh(&mut self.refresh_data)
    }

    /// Unconditionally refresh the underlying [`IndexStorage`] for this index.
    ///
    /// This must be called before the index will reflect changes made earlier in the same system.
    pub fn force_refresh(&mut self) {
        self.storage.force_refresh(&mut self.refresh_data)
    }
}

#[doc(hidden)]
pub struct IndexFetchState<'w, 's, I: IndexInfo + 'static> {
    storage_state: <ResMut<'w, I::Storage> as SystemParam>::State,
    refresh_data_state: <StaticSystemParam<
        'w,
        's,
        <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>,
    > as SystemParam>::State,
}
unsafe impl<'w, 's, I> SystemParam for Index<'w, 's, I>
where
    I: IndexInfo + 'static,
{
    type State = IndexFetchState<'static, 'static, I>;
    type Item<'_w, '_s> = Index<'_w, '_s, I>;
    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        if !world.contains_resource::<I::Storage>() {
            world.init_resource::<I::Storage>();
            if I::RefreshPolicy::REFRESH_EVERY_FRAME {
                let label = I::RefreshPolicy::schedule();
                world
                    .resource_mut::<Schedules>()
                    .get_mut(label.clone())
                    .expect(&format!("Can't find schedule `{label:?}`."))
                    .add_systems(refresh_index_system::<I>);
            }
        }
        IndexFetchState {
            storage_state: <ResMut<'w, I::Storage> as SystemParam>::init_state(world, system_meta),
            refresh_data_state: <StaticSystemParam<
                'w,
                's,
                <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>,
            > as SystemParam>::init_state(world, system_meta),
        }
    }
    fn new_archetype(state: &mut Self::State, archetype: &Archetype, system_meta: &mut SystemMeta) {
        <ResMut<'w, I::Storage> as SystemParam>::new_archetype(
            &mut state.storage_state,
            archetype,
            system_meta,
        );
        <StaticSystemParam<'w, 's, <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>> as SystemParam>::new_archetype(
            &mut state.refresh_data_state,
            archetype,
            system_meta,
        );
    }
    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <ResMut<'w, I::Storage> as SystemParam>::apply(
            &mut state.storage_state,
            system_meta,
            world,
        );
        <StaticSystemParam<'w, 's, <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>> as SystemParam>::apply(
            &mut state.refresh_data_state,
            system_meta,
            world,
        );
    }
    unsafe fn get_param<'w2, 's2>(
        state: &'s2 mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w2>,
        change_tick: Tick,
    ) -> Self::Item<'w2, 's2> {
        let mut idx = Index {
            storage: <ResMut<'w, I::Storage>>::get_param(
                &mut state.storage_state,
                system_meta,
                world,
                change_tick,
            ),
            refresh_data: <StaticSystemParam<
                'w,
                's,
                <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>,
            > as SystemParam>::get_param(
                &mut state.refresh_data_state,
                system_meta,
                world,
                change_tick,
            ),
        };
        if I::RefreshPolicy::REFRESH_WHEN_RUN {
            idx.refresh()
        }
        idx
    }
}
unsafe impl<'w, 's, I: IndexInfo + 'static> ReadOnlySystemParam for Index<'w, 's, I>
where
    ResMut<'w, I::Storage>: ReadOnlySystemParam,
    StaticSystemParam<'w, 's, <I::Storage as IndexStorage<I>>::RefreshData<'static, 'static>>:
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
        type RefreshPolicy = ConservativeRefreshPolicy;

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
            .add_systems(Startup, add_some_numbers)
            .add_systems(Update, checker(10, 2))
            .add_systems(Update, checker(20, 1))
            .add_systems(Update, checker(30, 1))
            .add_systems(Update, checker(40, 0))
            .run();
    }

    #[test]
    fn test_changing_values() {
        App::new()
            .add_systems(Startup, add_some_numbers)
            .add_systems(PreUpdate, checker(10, 2))
            .add_systems(PreUpdate, checker(20, 1))
            .add_systems(PreUpdate, checker(30, 1))
            .add_systems(Update, adder_all(5))
            .add_systems(PostUpdate, checker(10, 0))
            .add_systems(PostUpdate, checker(20, 0))
            .add_systems(PostUpdate, checker(30, 0))
            .add_systems(PostUpdate, checker(15, 2))
            .add_systems(PostUpdate, checker(25, 1))
            .add_systems(PostUpdate, checker(35, 1))
            .run();
    }

    #[test]
    fn test_changing_with_index() {
        App::new()
            .add_systems(Startup, add_some_numbers)
            .add_systems(PreUpdate, checker(10, 2))
            .add_systems(PreUpdate, checker(20, 1))
            .add_systems(Update, adder_some(10, 10))
            .add_systems(PostUpdate, checker(10, 0))
            .add_systems(PostUpdate, checker(20, 3))
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

                // already refreshed once this frame, need to use force.
                idx.refresh();
                assert_eq!(idx.lookup(&Number(20)).len(), 1);
                assert_eq!(idx.lookup(&Number(25)).len(), 0);

                idx.force_refresh();
                assert_eq!(idx.lookup(&Number(20)).len(), 0);
                assert_eq!(idx.lookup(&Number(25)).len(), 1);
            };

        App::new()
            .add_systems(Startup, add_some_numbers)
            .add_systems(Update, manual_refresh_system)
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
            .add_systems(Startup, add_some_numbers)
            .add_systems(PreUpdate, checker(20, 1))
            .add_systems(PreUpdate, checker(30, 1))
            .add_systems(Update, remover(20))
            .add_systems(PostUpdate, (next_frame, remover(30)).chain())
            // Detect component removed this earlier this frame
            .add_systems(Last, checker(30, 0))
            // Detect component removed after we ran last stage
            .add_systems(Last, checker(20, 0))
            .run();
    }

    #[test]
    fn test_despawn_detection() {
        App::new()
            .add_systems(Startup, add_some_numbers)
            .add_systems(PreUpdate, checker(20, 1))
            .add_systems(PreUpdate, checker(30, 1))
            .add_systems(Update, despawner(20))
            .add_systems(PostUpdate, (next_frame, despawner(30)).chain())
            // Detect component removed this earlier this frame
            .add_systems(Last, checker(30, 0))
            // Detect component removed after we ran last stage
            .add_systems(Last, checker(20, 0))
            .run();
    }

    #[test]
    fn test_despawn_detection_2_frames() {
        let mut app = App::new();
        app.add_systems(Startup, add_some_numbers)
            .add_systems(PostStartup, checker(20, 1))
            .add_systems(PostStartup, checker(30, 1));

        app.add_systems(Update, despawner(20));
        app.update();

        // Clear update schedule
        app.world
            .resource_mut::<Schedules>()
            .insert(Schedule::new(Update));
        app.update();

        app.add_systems(Update, despawner(30))
            // Detect component removed this earlier this frame
            .add_systems(Last, checker(30, 0))
            // Detect component removed multiple frames ago stage
            .add_systems(Last, checker(20, 0));
        app.update();
    }
}
