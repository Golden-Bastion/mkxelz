//! mkxelz-core: deterministic procedural tile generation.
//!
//! JSON spec + switches + seed  ->  PNG atlas + metadata JSON.
//! Same crate version + same spec + same seed always gives the same pixels.

pub mod autotile;
pub mod color;
pub mod error;
pub mod generate;
pub mod image;
pub mod layers;
pub mod noise;
pub mod presets;
pub mod preview;
pub mod rng;
pub mod spec;

pub use error::{Error, Result};
pub use generate::{generate, Entry, Output};
pub use image::Image;
pub use spec::Spec;

/// Semver of this crate; also written into every exported `tiles.json`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
