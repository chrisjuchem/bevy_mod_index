# bevy_mod_index

A Rust crate that allows efficient querying for components by their values in
the game engine [Bevy](https://bevyengine.org/).

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

## Usage
First, import the prelude. 
```rust
use bevy_mod_index::prelude::*;
```

Next, implement the `IndexInfo` trait. If your component only needs one index,
you can implement this trait directly on the component. If you need more than 
one, you can use a simple unit struct for each index beyond the first. You can
also use unit structs to give more descriptive names, even if you only need one
index.

You must specify the type of component being indexed, the type of value that 
you want to ba able to look up components by, and a function for calculating
that value for a given component.
```rust
struct NearOrigin {}
impl IndexInfo for NearOrigin {
  type Component = Transform;
  type Value = bool;

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
  
  let entities_near_spawn: HashSet<Entity> = index.get(true);
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

## Implementation
This implementation uses a custom `SystemParam` that updates the index whenever it is used.
The update is done by using a query to loop over all components, and only reading the actual
data/re-computing the index value when a component is changed since the last update. If the
index is not used, it will not update, even if its system runs, which can be useful if you
only need up-to-date data in certain circumstances (e.g. when the mouse is clicked) to save
re-computing values for rapidly changing data.

This implementation currently requires a [small patch to Bevy][patch] that allows us to 
check if the entities changed since some arbitrary time instead of just since the last time
the system ran.

## Compatability
| Bevy Version                 | `bevy_mod_index` Version |
|------------------------------|--------------------------|
| main ([custom patch][patch]) | 0.1.0                    |

## Troubleshooting
- `Query<(bevy_ecs::entity::Entity, &bevy_mod_index::index::test::Number, bevy_ecs::query::fetch::ChangeTrackers<bevy_mod_index::index::test::Number>), ()> in system bevy_mod_index::index::test::adder_some::{{closure}} accesses component(s) bevy_mod_index::index::test::Number in a way that conflicts with a previous system parameter. Consider using ``Without<T>`` to create disjoint Queries or merging conflicting Queries into a ``ParamSet``.`
  - Indexes use a read-only query of their components to update the index before it is used.
    If you have a query that mutably access these components in the same system as an `Index`,
    you can [combine them into a `ParamSet`][ParamSet] 

## Future work
- Cleanup removed components and despawned entities.
- Option to update the index when components change instead of when the index is used.
  - Naively, requires engine support for custom `DerefMut` hooks, but this would likely
    add overhead even when indexes aren't used. Other solutions may be possible.
    - Perhaps the `Component` derive will one day accept an attribute that enables/disables
      change detection by specifying `&mut T` or `Mut<T>` as the reference type, and we could
      add a third option for `IndexedMut<T>` that would automatically look up all indexes for
      the component in some resource and add the entity to a list to be re-indexed.
- More storage options besides `HashMap`.
  - Sorted container to allow for querying "nearby" values.
    - 1D data should be simple enough, but would also like to support kd-trees for positions.
  - No storage, using the naive loop approach, but with the nicer index API.
- Indexes over more than one `Component`.
- Indexes for subsets of a `Component`
  - Replacing Components with arbitrary queries may cover both of these cases.


[patch]: https://github.com/bevyengine/bevy/compare/main...chrisjuchem:bevy-fork:bevy_mod_index
[ParamSet]: https://docs.rs/bevy/latest/bevy/ecs/system/struct.ParamSet.html
