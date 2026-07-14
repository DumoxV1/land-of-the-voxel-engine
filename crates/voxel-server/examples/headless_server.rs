//! S-09 demo: run the authoritative server headless (no GPU, no window) for many ticks and
//! print a state summary. This is the proof that the engine has a runnable, GPU-free server.
//!
//! Run with: cargo run --example headless_server -p voxel-server

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_player::Input;
use voxel_server::Server;

fn main() {
    let mut srv = Server::new(7);

    // Spawn 3 players at different spots, all above the terrain.
    srv.add_player(1, [40.0, 40.0, 40.0]);
    srv.add_player(2, [10.0, 40.0, 10.0]);
    srv.add_player(3, [60.0, 40.0, 20.0]);

    // Player 1 walks forward for a while; the others idle (gravity settles them).
    for t in 0..600 {
        if t % 30 == 0 {
            srv.set_input(1, Input::forward());
        } else if t % 30 == 15 {
            srv.set_input(1, Input::none());
        }
        srv.tick(0.03);
    }

    // Player 2 places a beacon block; it is visible to everyone (shared world).
    let beacon = WorldVoxel::new(10, 14, 10);
    srv.apply_edit(2, beacon, MaterialId::from(4));

    println!("=== Land of the Voxel — headless dedicated server ===");
    println!("seed=7, players={}", srv.player_count());
    println!("edits logged: {}", srv.edit_count());
    for id in 1..=3 {
        let p = srv.player(id).expect("player exists");
        println!(
            "  player {id}: pos=({:.1}, {:.1}, {:.1}) on_ground={}",
            p.pos[0], p.pos[1], p.pos[2], p.on_ground
        );
    }
    println!(
        "  beacon @ {:?} material = {} (shared world: all players see this)",
        beacon,
        srv.material_at(beacon).0
    );
    println!("server ran {} ticks headless (no GPU, no renderer).", srv.tick_count());
}
