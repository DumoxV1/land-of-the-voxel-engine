//! S-08 demo: drop a player onto generated terrain, simulate a few physics steps, then render
//! the world from the player's camera position. Proves the controller rests the player on the
//! ground and that player-space camera framing works.
//!
//! Run with: cargo run --example demo_player -p voxel-render
//! Produces: crates/voxel-render/demo_player.png

use voxel_core::coords::ChunkCoord;
use voxel_player::{Input, Player, PlayerController};
use voxel_render::{Camera, render_world, BACKGROUND};
use voxel_world::World;

fn main() {
    let mut world = World::new(7);
    // 2x2 chunk block around the origin.
    let coord = ChunkCoord::new(0, 0, 0);
    let _ = world.get_or_generate(coord);
    world.get_or_generate(ChunkCoord::new(1, 0, 0));
    world.get_or_generate(ChunkCoord::new(0, 0, 1));
    world.get_or_generate(ChunkCoord::new(1, 0, 1));

    // Spawn a player above the terrain and let gravity settle it.
    let mut player = Player::new([40.0, 40.0, 40.0]);
    let mut ctrl = PlayerController::new();
    for _ in 0..400 {
        ctrl.step(&mut world, &mut player, Input::none(), 0.03);
    }
    println!(
        "player settled at ({:.1}, {:.1}, {:.1}), on_ground={}",
        player.pos[0], player.pos[1], player.pos[2], player.on_ground
    );

    // Render the 2x2 block from a camera placed at the player's eye.
    let mut chunks = Vec::new();
    for cx in 0..2i64 {
        for cz in 0..2i64 {
            let c = ChunkCoord::new(cx, 0, cz);
            chunks.push((c, world.get_or_generate(c)));
        }
    }
    let eye = [player.pos[0], player.pos[1] + 0.8, player.pos[2] + 6.0];
    let cam = Camera::new(eye[0], eye[1], eye[2], 50.0);
    let img = render_world(&chunks, &cam, 384, 384);
    let out = "crates/voxel-render/demo_player.png";
    img.save(out).expect("write demo_player.png");
    let non_bg = img.pixels().filter(|p| p.0 != BACKGROUND).count();
    println!("wrote {out}: {}x{} px, {} non-background pixels (player-eye view)", img.width(), img.height(), non_bg);
}
