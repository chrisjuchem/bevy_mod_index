# bevy_mod_index

[![](https://img.shields.io/crates/v/bevy_mod_index)](https://crates.io/crates/bevy_mod_index)
[![](https://docs.rs/bevy_mod_index/badge.svg)](https://docs.rs/bevy_mod_index/latest/bevy_mod_index)
[![](https://img.shields.io/crates/d/bevy_mod_index)](https://crates.io/crates/bevy_mod_index)
[![](https://img.shields.io/badge/Bevy%20version-v0.14.x-orange)](https://crates.io/crates/bevy/0.14.0)
[![](https://img.shields.io/github/license/chrisjuchem/bevy_mod_index?color=blue)](https://github.com/chrisjuchem/bevy_mod_index/blob/main/LICENSE)
[![](https://img.shields.io/github/stars/chrisjuchem/bevy_mod_index?color=green)](https://github.com/chrisjuchem/bevy_mod_index/stargazers)

A Rust crate that allows efficient querying for components by their values in
the game engine [Bevy].

## Compatability
| Bevy Version | `bevy_mod_index` Version |
|--------------|--------------------------|
| 0.14         | 0.5.x                    |
| 0.13         | 0.4.x                    |
| 0.12         | 0.3.0                    |
| 0.11         | 0.2.0                    |
| 0.10         | 0.1.0                    |

### Bevy Release Candidates

I do not publish release candidates for this crate corresponding to Bevy's,
but I do try to keep the main branch up to date with the latest RC. When there
are active Bevy RCs, the table below will include git commits you can use with
each RC version.

| Bevy Version | `bevy_mod_index` SHA |
|--------------|----------------------|
| 0.15.0-rc.1  | `1f79681`            |

## Features
| Feature name | Description                                    |
|--------------|------------------------------------------------|
| `reflect`    | Adds reflect derives to the storage resources. |

## Use Cases
It is quite common to want to write code in a system that only operates on 
components that have a certain value, e.g.:
```rust
fn move_living_players(mut players: Query<&mut Transform, &Player>) {
  for (mut transform, player) in &players {
    if player.is_alive() {
      move_player(transform);
    }
  }
}
```

With an index, we can change the code to:
```rust
fn move_living_players(
  mut transforms: Query<&mut Transform>, 
  player_alive_idx: Index<PlayerAlive>
) {
  for entity in &player_alive_idx.get(true) {
      transforms.get(entity).unwrap().move_player(transform);
  }
}
```

There are a few cases where a change like this may be beneficial:
- If `is_alive` is expensive to calculate, indexes can we can save work by 
  caching the results and only recomputing when the data actually changes.
  - If the component data that the result is calculated from doesn't change 
    often, we can use the cached values across frames.
  - If components tend to change only in the beginning of a frame, and the 
    results are needed multiple times later on, we can use the cached values
    across different systems, (or even the same system if it had been
    calculated multiple times).
- If we don't care too much about performance, indexes can provide a nicer
  API to work with.

Indexes add a non-zero amount of overhead, though, so introducing them can 
make your systems slower. Make sure to profile your systems before and after
introducing indexes if you care about performance.

## Getting Started
First, import the prelude. 
```rust
use bevy_mod_index::prelude::*;
```

Next, implement the `IndexInfo` trait. If your component only needs one index,
you can implement this trait directly on the component. If you need more than 
one, you can use a simple unit struct for each index beyond the first. You can
also use unit structs to give more descriptive names, even if you only need one
index.

You must specify:
- the type of component to be indexed,
- the type of value that you want to be able to use for lookups,
- a function for calculating that value for a component,
- how to store the relationship between an entity and the value calculated from 
  its appropriate component, and
- when the index should refresh itself with the latest data.
```rust
struct NearOrigin {}
impl IndexInfo for NearOrigin {
  type Component = Transform;
  type Value = bool;
  type Storage = HashmapStorage<Self>;
  const REFRESH_POLICY: IndexRefreshPolicy = IndexRefreshPolicy::WhenRun;

  fn value(t: &Transform) -> bool {
    t.translation.length() < 5.0
  }
}
```

Finally, include the `Index` system param in your systems and use it to query
for entities!
```rust
fn count_players_and_enemies_near_spawn(
  players: Query<(), With<(&Player, &Transform)>>,
  enemies: Query<(), With<(&Enemy, &Transform)>>,
  index: Index<NearOrigin>,
) {
  let (mut player_count, mut enemy_count) = (0, 0);
  
  let entities_near_spawn: HashSet<Entity> = index.lookup(true);
  for entity in entities_near_spawn.into_iter() {
    if let Ok(()) = players.get(entity) {
      player_count += 1;
    }
    if let Ok(()) = enemies.get(entity) {
      enemy_count += 1;
    }
  }
  
  println!("There are {} players and {} enemies near spawn!", player_count, enemy_count)
}
```

## Storage Implementations
`HashmapStorage` uses a `Resource` to cache a mapping between `Entity`s and the values computed
from their components. It uses a custom `SystemParam` to fetch the data that it needs to update
itself when needed. This is a good default choice, especially when the number of `Entity`s returned
by a `lookup` is expected to be just a small percentage of those in the entire query.

`NoStorage`, as the name implies, does not store any index data. Instead, it loops over all
data each time it is queried, computing the `value` function for each component, exactly like
the first `move_living_players` example above. This option allows you to use the index API
without incurring as much overhead as `HashmapStorage` (though still more than directly looping
over all components yourself).

## Refresh Policies
Indexes using `HashmapStorage` must be periodically `refresh`ed for them to be able to accurately
reflect the status of components as they are added, changed, and removed. Specifying an
`IndexRefreshPolicy` configures the index to automatically refresh itself for you with one of
several different timings.

`IndexRefreshPolicy::WhenRun` is a good default if you're not sure which refresh policy to use, but
other policies can be found [in the docs](https://docs.rs/bevy_mod_index/latest/bevy_mod_index/refresh_policy/enum.IndexRefreshPolicy.html).

## Reflection
Reflection for the storage resources can be enabled by selecting the optional `reflect` crate
feature. This is mainly useful for inspecting the underlying storage with `bevy-inspector-egui`.

In order for the resources to appear in the inspector, you will need to manually register the
storage for each index, e.g. `app.register_type::<HashmapStorage<NearOrigin>>();` Make sure that
you also derive `Reflect` for your `IndexInfo` type and any associated components/values.

Note: You should not rely on the internal structure of these resources, since they may change across
releases.

## API Stability
Consider the API to be extremely unstable as I experiment with what names and patterns feel
most natural and expressive, and also work on supporting new features.

## Performance
I have not put a lot of effort into optimizing the performance indexes yet. However, I have
done some initial tests under to get a sense of approximately how much overhead they add.

With 1 million entities, while none of the components change frame-to-frame, using the 
component itself as the index value, operation on ~300 entities takes:
 - 2-4x as long as a naive iteration when using `NoStorage`.
 - 3-5x as long as a naive iteration when using `HashmapStorage`.

With the same setup, except that 5% of the entities are updated every frame, performance for
`HashmapStorage` drops to 30-40x as long as naive iteration.

I am currently in the process of adding more concrete benchmarks, and I do have some plans
for changes that will affect performance.

## Get in contact
If you have suggestions for improvements to the API, or ideas about improving performance, 
I'd love to hear them. File an issue, or even better, reach out in the `bevy_mod_index`
`#crate-help` thread on Bevy's [discord].

## Troubleshooting
- `Query<(bevy_ecs::entity::Entity, &bevy_mod_index::index::test::Number, bevy_ecs::query::fetch::ChangeTrackers<bevy_mod_index::index::test::Number>), ()> in system bevy_mod_index::index::test::adder_some::{{closure}} accesses component(s) bevy_mod_index::index::test::Number in a way that conflicts with a previous system parameter. Consider using ``Without<T>`` to create disjoint Queries or merging conflicting Queries into a ``ParamSet``.`
  - Indexes use a read-only query of their components to update the index before it is used.
    If you have a query that mutably access these components in the same system as an `Index`,
    you can [combine them into a `ParamSet`][ParamSet].

## Future work
- Docs
- Option to update the index when components change instead of when the index is used.
  - Naively, requires engine support for custom `DerefMut` hooks, but this would likely
    add overhead even when indexes aren't used. Other solutions may be possible.
    - Perhaps the `Component` derive will one day accept an attribute that enables/disables
      change detection by specifying `&mut T` or `Mut<T>` as the reference type, and we could
      add a third option for `IndexedMut<T>` that would automatically look up all indexes for
      the component in some resource and add the entity to a list to be re-indexed.
      - See https://github.com/bevyengine/bevy/pull/7499 for a draft implementation.
- More storage options besides `HashMap`.
  - Sorted container to allow for querying "nearby" values.
    - 1D data should be simple enough, but would also like to support kd-trees for positions.
- Indexes over more than one `Component`.
- Indexes for subsets of a `Component`
  - Replacing Components with arbitrary queries may cover both of these cases.
- Derive for simple cases of IndexInfo where the component itself is used as the value.

[Bevy]: https://bevyengine.org/
[discord]: https://discord.gg/bevy
[ParamSet]: https://docs.rs/bevy/latest/bevy/ecs/system/struct.ParamSet.html
