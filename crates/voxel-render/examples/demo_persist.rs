//! S-07 demo: generate a world, apply edits, SAVE, LOAD into a fresh world, and render it.
//! Proves the full persistence chain (seed + edit log) reproduces the edited world.
//!
//! Run with: cargo run --example demo_persist -p voxel-render
//! Produces: crates/voxel-render/demo_persist.png

use voxel_core::coords::{ChunkCoord, LocalVoxel, WorldVoxel};
use voxel_core::palette::MaterialId;
use voxel_edit::EditTool;
use voxel_persist::{load_world, save_world};
use voxel_render::{Camera, render_world, BACKGROUND};
use voxel_world::World;

fn main() {
    // 1) Build a 2x2 world with seed and a few edits (a small tower of metal).
    let mut world = World::new(7);
    let mut tool = EditTool::new();
    let tower = WorldVoxel::new(40, 0, 40);
    let tcoord = ChunkCoord::from_world(tower);
    let local = LocalVoxel::from_world(tower);
    let generated = world.get_or_generate(tcoord);
    let mut top = 0i64;
    for ly in (0..32u8).rev() {
        if generated.get(LocalVoxel::new(local.x, ly, local.z)) != MaterialId::from(0) {
            top = ly as i64;
            break;
        }
    }
    for dy in 1..=4i64 {
        tool.place(&mut world, WorldVoxel::new(40, top + dy, 40), MaterialId::from(4), 1, dy as u64);
    }
    let log = tool.into_log();

    // 2) SAVE to a temp file.
    let path = std::env::temp_dir().join("lotve_demo_persist.bin");
    save_world(&world, &log, &path).expect("save");

    // 3) LOAD into a fresh world (simulates server restart).
    let (mut loaded, loaded_log) = load_world(&path).expect("load");
    assert_eq!(loaded_log.len(), log.len(), "loaded log keeps all edits");

    // 4) Render the loaded world (2x2 chunks + restored tower).
    let mut chunks = Vec::new();
    for cx in 0..2i64 {
        for cz in 0..2i64 {
            let coord = ChunkCoord::new(cx, 0, cz);
            chunks.push((coord, loaded.get_or_generate(coord)));
        }
    }
    let cam = Camera::new(35.0, 35.0, 90.0, 50.0);
    let img = render_world(&chunks, &cam, 384, 384);

    let out = "crates/voxel-render/demo_persist.png";
    img.save(out).expect("write demo_persist.png");
    let non_bg = img.pixels().filter(|p| p.0 != BACKGROUND).count();
    println!("wrote {out}: {}x{} px, {} non-background pixels (persisted + reloaded)", img.width(), img.height(), non_bg);
}
