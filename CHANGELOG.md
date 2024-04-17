
# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

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
