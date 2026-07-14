//! S-03 demo: render a small voxel scene to a PNG artifact for visual inspection.
//!
//! Run with: cargo run --example demo -p voxel-render
//! Produces: crates/voxel-render/demo.png

use voxel_core::chunk::Chunk;
use voxel_core::coords::ChunkCoord;
use voxel_core::palette::MaterialId;
use voxel_render::{Camera, render_scene, BACKGROUND};

fn main() {
    // Build a small demo chunk: a solid base layer (material 2 = grass) with a couple of
    // "columns" (material 1 = dirt/stone) and a single bright voxel (material 4 = metal).
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0));

    // Ground slab: y = 0..4 over the full 32x32 footprint.
    for x in 0..32u8 {
        for z in 0..32u8 {
            for y in 0..4u8 {
                chunk.set(
                    voxel_core::coords::LocalVoxel::new(x, y, z),
                    MaterialId::from(2),
                );
            }
        }
    }
    // Two pillars.
    for (px, pz) in [(8u8, 8u8), (22u8, 20u8)] {
        for y in 4..16u8 {
            chunk.set(
                voxel_core::coords::LocalVoxel::new(px, y, pz),
                MaterialId::from(1),
            );
            chunk.set(
                voxel_core::coords::LocalVoxel::new(px + 1, y, pz),
                MaterialId::from(1),
            );
        }
    }
    // A beacon voxel on top of one pillar.
    chunk.set(
        voxel_core::coords::LocalVoxel::new(9, 16, 8),
        MaterialId::from(4),
    );

    // Orbit the camera a little above the horizon for a 3/4 view.
    let cam = Camera::new(40.0, 28.0, 70.0, 50.0);
    let img = render_scene(&chunk, &cam, 256, 256);

    // Fill the rest of the canvas with the background (img is already sized; just save).
    let path = "crates/voxel-render/demo.png";
    img.save(path).expect("write demo.png");
    let non_bg = img.pixels().filter(|p| p.0 != BACKGROUND).count();
    println!("wrote {path}: {}x{} px, {} non-background pixels", img.width(), img.height(), non_bg);
}
