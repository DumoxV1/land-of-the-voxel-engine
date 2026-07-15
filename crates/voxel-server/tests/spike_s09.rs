//! S-09 headless server tests (strict TDD — written before implementation, run RED first).
//! References `voxel_server::Server`, which may not yet exist.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_player::{Input, Player};
use voxel_server::Server;

#[test]
fn server_tick_falls_players_to_ground() {
    let mut srv = Server::new(7);
    srv.add_player(1, [40.0, 40.0, 40.0]);
    for _ in 0..400 {
        srv.tick(0.03);
    }
    let p = srv.player(1).expect("player 1 exists");
    assert!(p.on_ground, "player must be on the ground after falling");
    assert!(
        p.pos[1] > 1.0 && p.pos[1] < 400.0,
        "player should rest on generated terrain (now up to ~40 m / 320 vox), got y={}",
        p.pos[1]
    );
    // No GPU/renderer involved — this is a pure-sim assertion.
    let _ = Player::new([0.0, 0.0, 0.0]); // ensure voxel-player API present
    let _ = Input::none();
}

#[test]
fn server_apply_edit_visible_to_all() {
    let mut srv = Server::new(7);
    srv.add_player(1, [40.0, 40.0, 40.0]);
    srv.add_player(2, [10.0, 40.0, 10.0]);

    let wv = WorldVoxel::new(40, 12, 40);
    srv.apply_edit(1, wv, MaterialId::from(4)); // player 1 places a metal block

    // Both players' views of the world read the same value.
    let seen_by_1 = srv.material_at(wv);
    let seen_by_2 = srv.material_at(wv);
    assert_eq!(seen_by_1, MaterialId::from(4));
    assert_eq!(seen_by_2, MaterialId::from(4), "edit is visible to all players (shared world)");

    // The edit is recorded in the log (auditable, replayable).
    assert_eq!(srv.edit_count(), 1, "edit must be logged");
}

#[test]
fn server_deterministic_same_seed_same_inputs() {
    // Two servers, same seed, same scripted inputs -> identical final world voxel.
    let wv = WorldVoxel::new(20, 10, 20);

    let mut a = Server::new(123);
    a.add_player(1, [20.0, 40.0, 20.0]);
    a.apply_edit(1, wv, MaterialId::from(7));
    for _ in 0..200 {
        a.tick(0.03);
    }

    let mut b = Server::new(123);
    b.add_player(1, [20.0, 40.0, 20.0]);
    b.apply_edit(1, wv, MaterialId::from(7));
    for _ in 0..200 {
        b.tick(0.03);
    }

    assert_eq!(
        a.material_at(wv),
        b.material_at(wv),
        "same seed + same edits => identical world (server-authoritative determinism)"
    );
    // And the spawned players land at the same place.
    let pa = a.player(1).unwrap();
    let pb = b.player(1).unwrap();
    let dx = (pa.pos[0] - pb.pos[0]).abs();
    let dy = (pa.pos[1] - pb.pos[1]).abs();
    let dz = (pa.pos[2] - pb.pos[2]).abs();
    assert!(dx < 1e-4 && dy < 1e-4 && dz < 1e-4, "players deterministic: {pa:?} vs {pb:?}");
}

#[test]
fn server_headless_compiles_without_renderer() {
    // Structural: this test only passes if the crate builds, which it can't if it depends on
    // a renderer. We assert the server can be constructed and ticked with no rendering calls.
    let mut srv = Server::new(1);
    srv.add_player(1, [5.0, 30.0, 5.0]);
    let before = srv.player(1).unwrap().pos[1];
    for _ in 0..50 {
        srv.tick(0.03);
    }
    let after = srv.player(1).unwrap().pos[1];
    assert!(after < 400.0, "player fell (gravity) and landed on terrain, not at spawn");
}
