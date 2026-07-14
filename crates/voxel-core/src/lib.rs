//! voxel-core: minimal, renderer-agnostic voxel coordinate + storage crate (S-01).
//!
//! This crate deliberately has NO dependency on any renderer (godot/bevy/wgpu) — see ADR-0002.
//! It provides integer world coordinates with Euclidean division, a chunk abstraction,
//! a material palette, edit events with idempotence, and byte-stable serialization.

pub mod coords;
pub mod chunk;
pub mod palette;
pub mod edit;
pub mod serialize;

pub use coords::{ChunkCoord, LocalVoxel, WorldVoxel, CHUNK_SIZE};
pub use chunk::{Chunk, ChunkState};
pub use palette::MaterialId;
pub use edit::{Edit, EditId};
