//! S-08 player-controller tests (strict TDD — written before implementation, run RED first).
//! References `voxel_player::{Player, PlayerController, Input}`, which may not yet exist.

use voxel_core::coords::ChunkCoord;
use voxel_core::palette::MaterialId;
use voxel_player::{Input, Player, PlayerController};
use voxel_world::World;

/// Build a flat world: a solid floor of stone (material 3) at world-Y = 0, rest is air.
/// NOTE: setting any voxel generates the chunk's seeded terrain (which fills air up to ~Y20),
/// so we explicitly clear everything above the floor to get a controlled, flat test world.
fn flat_world() -> World {
    let mut w = World::new(0);
    for x in -8..32 {
        for z in -8..32 {
            // Floor.
            w.set_voxel(voxel_core::coords::WorldVoxel::new(x, 0, z), MaterialId::from(3));
            // Clear anything above the floor (overrides generated terrain).
            for y in 1..32 {
                w.set_voxel(voxel_core::coords::WorldVoxel::new(x, y, z), MaterialId::from(0));
            }
        }
    }
    w
}

#[test]
fn step_forward_moves_player() {
    let mut world = flat_world();
    let mut player = Player::new([8.0, 2.0, 8.0]);
    let before = player.pos;
    // Face +X (yaw = 0 means looking toward +X) and walk forward.
    player.yaw = 0.0;
    let mut ctrl = PlayerController::new();
    ctrl.step(&mut world, &mut player, Input::forward(), 0.1);
    assert!(
        (player.pos[0] - before[0]).abs() > 1e-3,
        "forward step must move the player along +X"
    );
    assert!(
        (player.pos[2] - before[2]).abs() < 1e-3,
        "forward (+X yaw) must not move along Z"
    );
}

#[test]
fn collision_blocks_movement() {
    let mut world = flat_world();
    // Drop a wall of stone at x = 10 directly ahead of the player (x=8, facing +X).
    for y in 1..6 {
        for z in 7..10 {
            world.set_voxel(voxel_core::coords::WorldVoxel::new(10, y, z), MaterialId::from(3));
        }
    }
    let mut player = Player::new([8.0, 2.0, 8.0]);
    player.yaw = 0.0; // face +X
    let mut ctrl = PlayerController::new();
    // Many forward steps; must not pass x=10 (player half-width 0.3 keeps it just below 10).
    for _ in 0..50 {
        ctrl.step(&mut world, &mut player, Input::forward(), 0.1);
    }
    assert!(
        player.pos[0] < 9.8,
        "collision must stop the player before the wall at x=10 (closest stable x≈9.7), got x={}",
        player.pos[0]
    );
}

#[test]
fn gravity_makes_player_fall_and_rest_on_ground() {
    let mut world = flat_world();
    // Player starts in the air above the floor (y=0). Should fall and rest on top of it.
    let mut player = Player::new([8.0, 5.0, 8.0]);
    let mut ctrl = PlayerController::new();
    for _ in 0..200 {
        ctrl.step(&mut world, &mut player, Input::none(), 0.05);
    }
    // Floor top is the top of the y=0 voxel (voxel fills [0,1)), so the player's feet rest at
    // y=1.0 and its center at ~1.9 (half-height 0.9).
    assert!(
        player.pos[1] > 1.0 && player.pos[1] < 2.5,
        "player must rest on the floor (feet on top of the y=0 voxel -> center ~1.9), got y={}",
        player.pos[1]
    );
    assert!(player.on_ground, "player must be flagged on_ground");
}

#[test]
fn per_axis_slide_along_wall() {
    let mut world = flat_world();
    // A wall ahead at x=10 (broad in Z so the player cannot slip past within the test).
    for y in 1..6 {
        for z in 7..40 {
            world.set_voxel(voxel_core::coords::WorldVoxel::new(10, y, z), MaterialId::from(3));
        }
    }
    let mut player = Player::new([8.0, 2.0, 10.0]);
    player.yaw = 0.0; // face +X
    let mut ctrl = PlayerController::new();
    // Hold forward (into wall) + right (slide along Z). X must be blocked, Z must advance.
    let mut input = Input::forward();
    input.right = true;
    let before_z = player.pos[2];
    for _ in 0..50 {
        ctrl.step(&mut world, &mut player, input, 0.1);
    }
    assert!(player.pos[0] < 9.8, "X blocked by wall (closest stable x≈9.7)");
    assert!(
        (player.pos[2] - before_z).abs() > 0.5,
        "Z should slide along the wall, got dz={}",
        player.pos[2] - before_z
    );
}
