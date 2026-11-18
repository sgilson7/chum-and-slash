//! chum_core: every rule of Chum and Slash. No input or output.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
//!
//! `fx`, `geom`, `mesh` and `render` are ported from Protection of the
//! Spheres (MIT, Sam Gilson; LICENSES.md); `rng` and `ticks` from Builder's
//! realtime kit.

pub mod api;
pub mod clip;
pub mod fx;
pub mod geom;
pub mod glossary;
pub mod mesh;
pub mod model;
pub mod page;
pub mod pilot;
pub mod render;
pub mod rng;
pub mod scene;
pub mod ticks;
pub mod tutorial;
pub mod world;
