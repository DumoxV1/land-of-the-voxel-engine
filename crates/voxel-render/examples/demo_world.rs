//! S-05 demo: render a small multi-chunk world (generated + edited) to a PNG artifact.
//!
//! Run with: cargo run --example demo_world -p voxel-render
//! Produces: crates/voxel-render/demo_world.png

use voxel_core::coords::{ChunkCoord, LocalVoxel, WorldVoxel};
use voxel_core::palette::MaterialId;
use voxel_render::{Camera, render_world, BACKGROUND};
use voxel_world::World;

fn main() {
    let mut world = World::new(7);

    // Generate a 2x2 block of chunks (x,z in {0,1}).
    let mut chunks = Vec::new();
    for cx in 0..2i64 {
        for cz in 0..2i64 {
            let coord = ChunkCoord::new(cx, 0, cz);
            chunks.push((coord, world.get_or_generate(coord)));
        }
    }

    // Player edit: a small metal tower at world (40, *, 40) — placed ON TOP of the terrain.
    let tower_wx = 40i64;
    let tower_wz = 40i64;
    let tower_coord = ChunkCoord::from_world(WorldVoxel::new(tower_wx, 0, tower_wz));
    // Measure the surface height (topmost solid voxel) at that world column.
    let chunk = world.get_or_generate(tower_coord);
    let local = LocalVoxel::from_world(WorldVoxel::new(tower_wx, 0, tower_wz));
    let mut top = 0i64;
    for ly in (0..32u8).rev() {
        if chunk.get(LocalVoxel::new(local.x, ly, local.z)) != MaterialId::from(0) {
            top = ly as i64;
            break;
        }
    }
    // Build a 4-voxel tower above the surface.
    for dy in 1..=4i64 {
        world.set_voxel(
            WorldVoxel::new(tower_wx, top + dy, tower_wz),
            MaterialId::from(4),
        );
    }

    // Re-fetch the edited chunk so the tower shows up in the rendered set.
    let edited = world.get_or_generate(tower_coord);
    // Replace the placeholder for this coord with the edited version.
    for slot in chunks.iter_mut() {
        if slot.0 == tower_coord {
            slot.1 = edited.clone();
        }
    }

    let cam = Camera::new(35.0, 35.0, 90.0, 50.0);
    let img = render_world(&chunks, &cam, 384, 384);

    let path = "crates/voxel-render/demo_world.png";
    img.save(path).expect("write demo_world.png");
    let non_bg = img.pixels().filter(|p| p.0 != BACKGROUND).count();
    println!("wrote {path}: {}x{} px, {} non-background pixels", img.width(), img.height(), non_bg);
}
