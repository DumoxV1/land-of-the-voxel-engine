//! S-04 demo: render a deterministically generated worldgen chunk to a PNG artifact.
//!
//! Run with: cargo run --example demo_worldgen -p voxel-render
//! Produces: crates/voxel-render/demo_worldgen.png

use voxel_core::coords::ChunkCoord;
use voxel_render::{Camera, render_scene, BACKGROUND};
use voxel_worldgen::generate_chunk;

fn main() {
    // Generate a deterministic terrain chunk from a seed (no hand-built voxels).
    let chunk = generate_chunk(ChunkCoord::new(0, 0, 0), 7);

    // Orbit camera for a 3/4 view of the generated surface.
    let cam = Camera::new(40.0, 32.0, 72.0, 50.0);
    let img = render_scene(&chunk, &cam, 320, 320);

    let path = "crates/voxel-render/demo_worldgen.png";
    img.save(path).expect("write demo_worldgen.png");
    let non_bg = img.pixels().filter(|p| p.0 != BACKGROUND).count();
    println!("wrote {path}: {}x{} px, {} non-background pixels", img.width(), img.height(), non_bg);
}
