# bevy_mod_index

[![](https://img.shields.io/crates/v/bevy_mod_index)](https://crates.io/crates/bevy_mod_index)
[![](https://docs.rs/bevy_mod_index/badge.svg)](https://docs.rs/bevy_mod_index/latest/bevy_mod_index)
[![](https://img.shields.io/crates/d/bevy_mod_index)](https://crates.io/crates/bevy_mod_index)
[![](https://img.shields.io/badge/Bevy%20version-v0.12.x-orange)](https://crates.io/crates/bevy/0.12.0)
[![](https://img.shields.io/github/license/chrisjuchem/bevy_mod_index?color=blue)](https://github.com/chrisjuchem/bevy_mod_index/blob/main/LICENSE)
[![](https://img.shields.io/github/stars/chrisjuchem/bevy_mod_index?color=green)](https://github.com/chrisjuchem/bevy_mod_index/stargazers)

A Rust crate that allows efficient querying for components by their values in
the game engine [Bevy].

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
- a function for calculating that value for a component, and
- how to store the relationship between an entity and the value calculated from 
  its appropriate component.
```rust
struct NearOrigin {}
impl IndexInfo for NearOrigin {
  type Component = Transform;
  type Value = bool;
  type Storage = HashmapStorage<Self>;

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

## Implementations
`HashmapStorage` uses a custom `SystemParam` that updates the index whenever it is used.
The update is done by using a query to loop over all components, and only reading the actual
data/re-computing the index value when a component is changed since the last update. If the
index is not used, it will not update, even if its system runs, which can be useful if you
only need up-to-date data in certain circumstances (e.g. when the mouse is clicked) to save
re-computing values for rapidly changing data.

`NoStorage`, as the name implies, does not store any index data. Instead, it loops over all
data each time it is queried, computing the `value` function for each component, exactly like
the first `move_living_players` example above. This option allows you to use the index API
without incurring as much overhead as `HashmapStorage` (though still more than directly looping
over all components yourself)

## Compatability
| Bevy Version | `bevy_mod_index` Version |
|--------------|--------------------------|
| 0.13         | 0.4.0                    |
| 0.12         | 0.3.0                    |
| 0.11         | 0.2.0                    |
| 0.10         | 0.1.0                    |

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
- `lookup` returned entities which no longer exist/no longer have the relevant component.
  - Currently, detection of removed entities and components relies on `RemovedComponents`,
    which only has a 2-frame buffer. If no systems that use your index run within a frame
    of a component or entity being removed, it will be missed. This means that run conditions
    should generally be avoided, but including the `Index` in the condition may alleviate the
    issue (though I have not tested this).

## Future work
- Docs
- Return an iterator of matching `Entity`s instead of a `HashSet`.
- Cleanup removed components and despawned entities without needing to run every frame.
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
