use bevy::ecs::all_tuples;
use bevy::ecs::query::{QueryFilter, ReadOnlyQueryData};
use bevy::ecs::system::ReadOnlySystemParam;
use bevy::prelude::{Changed, Component, RemovedComponents};

pub trait ComponentTuple {
    type Refs<'a>: for<'r> ReadOnlyQueryData<Item<'r> = Self::Refs<'r>>;
    type ChangedFilter: QueryFilter;
    type Removed<'w, 's>: for<'a, 'b> ReadOnlySystemParam<Item<'a, 'b> = Self::Removed<'a, 'b>>
        + RemovedComponentIter;
}

impl<C: Component> ComponentTuple for &C {
    type Refs<'r> = &'r C;
    type ChangedFilter = Changed<C>;
    type Removed<'w, 's> = (RemovedComponents<'w, 's, C>,);
}

macro_rules! impl_component_tuple {
    ($($C:ident),*) => {
        impl<$($C: bevy::ecs::component::Component),*> ComponentTuple for ($($C,)*) {
            type Refs<'r> = ($(&'r $C,)*);
            type ChangedFilter = bevy::prelude::Or<($( bevy::ecs::query::Changed<$C>,)*)>;
            type Removed<'w, 's> = ($(bevy::ecs::removal_detection::RemovedComponents<'w, 's, $C>,)*);
        }
    }
}
all_tuples!(impl_component_tuple, 1, 15, C);

pub trait RemovedComponentIter {
    fn read_all(&mut self) -> impl Iterator<Item = bevy::prelude::Entity>;
}

// Based on
// https://stackoverflow.com/questions/66396814/generating-tuple-indices-based-on-macro-rules-repetition-expansion
macro_rules! tuple_items {
    (@ $tpl:ident, (), ($($i:tt)*), [$($result:expr),*], $func:ident) => {
        [$($result),*]
    };
    (@ $tpl:ident, ($C0:ident $($C:ident)*), ($idx:tt $($i:tt)*), [$($result:expr),*], $func:ident) => {
        tuple_items!(
            @ $tpl, ($($C)*), ($($i)*), [$($result,)* $tpl.$idx.$func()], $func
        )
    };
    ($tpl:ident, ($($C:ident)*), $func:ident) => {
        tuple_items!(@ $tpl, ($($C)*), (0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15), [], $func)
    };
}

macro_rules! impl_removed_tuple_iter {
    ($($C:ident),*) => {
        impl<'w, 's, $($C: bevy::ecs::component::Component),*> RemovedComponentIter for
                ($(bevy::ecs::removal_detection::RemovedComponents<'w, 's, $C>,)*) {
            fn read_all(&mut self) -> impl Iterator<Item = bevy::prelude::Entity> {
                tuple_items!(self, ($($C)*), read).into_iter().flatten()
            }
        }
    }
}
all_tuples!(impl_removed_tuple_iter, 1, 15, C);
