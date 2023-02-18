use crate::index::IndexInfo;
use crate::unique_multimap::UniqueMultiMap;
use bevy::ecs::component::Tick;
use bevy::ecs::system::{StaticSystemParam, SystemChangeTick, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashSet;

pub trait IndexStorage<I: IndexInfo>: Resource + Default {
    type RefreshData<'w, 's>: SystemParam;

    fn get<'w, 's>(
        &mut self,
        val: &I::Value,
        data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>,
    ) -> HashSet<Entity>;
    fn refresh<'w, 's>(&mut self, data: &mut StaticSystemParam<Self::RefreshData<'w, 's>>);
}

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
