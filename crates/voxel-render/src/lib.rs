//! voxel-render: software-raster spike (S-03) to make `voxel_mesher` output visible.
//!
//! Consumes a `voxel_core::Chunk` (meshed via `voxel_mesher::greedy_mesh`), projects it
//! with a simple perspective `Camera`, and rasterizes to a PNG via the pure-Rust `image`
//! crate. No GPU/renderer dependency — this spike stays renderer-agnostic-friendly.
//! The real client-shell/renderer (wgpu -> Vulkan/DX12/Metal, Godot, Bevy) is a Phase-2
//! decision behind equal-priority benchmarks (ADR-0002).

pub mod camera;
pub mod render;

pub use camera::Camera;
pub use render::{render_scene, render_world, BACKGROUND};
