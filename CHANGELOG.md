
# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [0.6.0] - 2024-12-01

Bevy version updated to `0.14`.

### Changed
- Type signature of `IndexStorage::removal_observer` updated to match
  [bevy's change to the `Observer` type](https://github.com/bevyengine/bevy/pull/15151).

## [0.5.0] - 2024-07-04

Bevy version updated to `0.14`.

### Added
- Added derives for `Eq`, `PartialEq`, `Debug`, `Copy`, and `Clone` to
  `UniquenessError`.

### Changed
- `HashmapStorage` now uses Observers instead of `RemovedComponents` to
  know when to clean up stale entries.
- `IndexRefreshPolicy` has been changed from a trait to an enum because
  the switch to Observers should eliminate the need for complex
  refresh policy configurations.
- The `RefereshPolicy` associated type of `IndexInfo` is now a constant
  called `REFRESH_POLICY`.
- Calling `lookup` directly on a storage resource no longer refreshes the
  storage if refresh policy is `WhenUsed`. This was moved up to `Index`'s
  `refresh` method.

### Removed
- Removed concrete implementations of the old `IndexRefreshPolicy` trait
  such as `ConservativeRefreshPolicy`.

## [0.4.1] - 2024-04-20

### Added
- Added `reflect` crate feature with `Reflect` derives for storage types.

## [0.4.0] - 2024-02-17

Bevy version updated to `0.13`.

### Changed
- `Index::lookup` now returns an `impl Iterator<Item=Entity>` instead of a
  `HashSet<Entity>` to avoid unnecessary allocations.
- `Index::refresh` is now a no-op if it was previously called during this `Tick`.
  `Index::force_refresh` can now be used to refresh the index unconditionally.

### Added
- Added `Index::force_refresh` (see above).
- Added `Index::lookup_single` and `Index::single` (panicking version) for
  cases when a lookup is expected to return only one `Entity`.
- Added the `RefereshPolicy` associated type to the `IndexInfo` trait to allow
  specifying storage refresh behavior.
  - Also added the `IndexRefreshPolicy` trait to support this, as well as 5
    concrete policies that can be used, such as `ConservativeRefreshPolicy`.
