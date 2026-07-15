//! S-08 player-controller tests (strict TDD). The controller works in VOXEL units
//! (1 voxel = 0.125 m); positions below are expressed in voxels. 1 m = 8 voxels.

use voxel_core::coords::{WorldVoxel, VOXEL_SIZE_M};
use voxel_core::palette::MaterialId;
use voxel_player::{Input, Player, PlayerController};
use voxel_world::World;

/// Build a flat world: a solid floor of stone (material 3) at world-Y = 0, rest is air.
/// NOTE: setting any voxel generates the chunk's seeded terrain (which fills air up to ~Y20),
/// so we explicitly clear everything above the floor to get a controlled, flat test world.
fn flat_world() -> World {
    let mut w = World::new(0);
    for x in -64..256 {
        for z in -64..256 {
            w.set_voxel(WorldVoxel::new(x, 0, z), MaterialId::from(3));
            for y in 1..256 {
                w.set_voxel(WorldVoxel::new(x, y, z), MaterialId::from(0));
            }
        }
    }
    w
}

#[test]
fn step_forward_moves_player() {
    let mut world = flat_world();
    // Player at voxel (64, 16, 64) — well above the floor (voxel y=0).
    let mut player = Player::new([64.0, 16.0, 64.0]);
    let before = player.pos;
    // Face +X (yaw = 0 means looking toward +X) and walk forward.
    player.yaw = 0.0;
    let mut ctrl = PlayerController::new();
    ctrl.step(&mut world, &mut player, Input::forward(), 0.1);
    assert!(
        (player.pos[0] - before[0]).abs() > 1e-3,
        "forward step must move the player along +X (voxels)"
    );
    assert!(
        (player.pos[2] - before[2]).abs() < 1e-3,
        "forward (+X yaw) must not move along Z"
    );
}

#[test]
fn collision_blocks_movement() {
    let mut world = flat_world();
    // Drop a wall of stone at x = 80 (10 m) directly ahead of the player (x=64, facing +X).
    for y in 1..48 {
        for z in 56..80 {
            world.set_voxel(WorldVoxel::new(80, y, z), MaterialId::from(3));
        }
    }
    let mut player = Player::new([64.0, 16.0, 64.0]);
    player.yaw = 0.0; // face +X
    let mut ctrl = PlayerController::new();
    // Many forward steps; must not pass x=80 (half-width keeps it just below 80).
    for _ in 0..50 {
        ctrl.step(&mut world, &mut player, Input::forward(), 0.1);
    }
    assert!(
        player.pos[0] < 79.0,
        "collision must stop the player before the wall at x=80, got x={}",
        player.pos[0]
    );
}

#[test]
fn gravity_makes_player_fall_and_rest_on_ground() {
    let mut world = flat_world();
    // Player starts in the air above the floor (y=0). Should fall and rest on top of it.
    let mut player = Player::new([64.0, 40.0, 64.0]);
    let mut ctrl = PlayerController::new();
    for _ in 0..200 {
        ctrl.step(&mut world, &mut player, Input::none(), 0.05);
    }
    // Floor top is the top of the y=0 voxel (voxel fills [0,1)), so the player's feet rest at
    // y=1.0 voxel and its center at ~15.2 vox (half-height 7.6). Express in voxels.
    let expected_center_vox = (1.0 + voxel_player::HALF[1]);
    assert!(
        player.pos[1] > expected_center_vox - 1.0
            && player.pos[1] < expected_center_vox + 1.0,
        "player must rest on the floor (feet on top of the y=0 voxel -> center ~{:.1}), got y={}",
        expected_center_vox,
        player.pos[1]
    );
    assert!(player.on_ground, "player must be flagged on_ground");
}

#[test]
fn per_axis_slide_along_wall() {
    let mut world = flat_world();
    // A wall ahead at x=80 (broad in Z so the player cannot slip past within the test).
    for y in 1..48 {
        for z in 56..320 {
            world.set_voxel(WorldVoxel::new(80, y, z), MaterialId::from(3));
        }
    }
    // Player at voxel (64, 16, 80) facing +X, wall at x=80.
    let mut player = Player::new([64.0, 16.0, 80.0]);
    player.yaw = 0.0; // face +X
    let mut ctrl = PlayerController::new();
    // Hold forward (into wall) + right (slide along Z). X must be blocked, Z must advance.
    let mut input = Input::forward();
    input.right = true;
    let before_z = player.pos[2];
    for _ in 0..50 {
        ctrl.step(&mut world, &mut player, input, 0.1);
    }
    assert!(player.pos[0] < 79.0, "X blocked by wall at x=80");
    assert!(
        (player.pos[2] - before_z).abs() > 0.5,
        "Z should slide along the wall, got dz={}",
        player.pos[2] - before_z
    );
}

/// A flat world with a 1-voxel-high ledge: floor at Y=0 everywhere, plus a raised
/// plateau (floor at Y=1) for x >= 100. Tests that the avatar can walk *up* a gentle
/// rise instead of being blocked by the 1-voxel step (the "can't walk up a hill" bug).
fn ledge_world() -> World {
    let mut w = flat_world();
    for x in 100..256 {
        for z in -64..256 {
            w.set_voxel(WorldVoxel::new(x, 1, z), MaterialId::from(3));
        }
    }
    w
}

#[test]
fn walks_up_one_voxel_ledge() {
    let mut world = ledge_world();
    // Start on the low floor (feet on Y=0) just before the ledge at x=100, facing +X.
    let mut player = Player::new([96.0, 8.0, 64.0]);
    let mut ctrl = PlayerController::new();
    player.yaw = 0.0;
    let start_y = player.pos[1];
    // Walk forward across the ledge boundary (x=100) long enough to cross it.
    for _ in 0..200 {
        ctrl.step(&mut world, &mut player, Input::forward(), 0.016);
    }
    // Must have advanced into / past the ledge.
    assert!(
        player.pos[0] > 100.0,
        "player should walk up and over the 1-voxel ledge (x={:.1}, stuck below 100)",
        player.pos[0]
    );
    // And the avatar should now be standing on the raised floor (center y ~ 1 + HALF[1]),
    // i.e. noticeably above the start height — proving it stepped up, not floated/jumped.
    assert!(
        player.pos[1] > start_y + 0.5,
        "player should have stepped UP onto the ledge (y went {:.2} -> {:.2})",
        start_y,
        player.pos[1]
    );
}

/// REGRESSION (step-up refactor): horizontal movement while airborne must NOT flag
/// `on_ground`, otherwise the player can jump mid-air (Space while falling). The
/// `on_ground` flag must be owned exclusively by the Y-axis gravity/floor resolution,
/// never by a flat horizontal step.
#[test]
fn horizontal_move_while_airborne_is_not_grounded() {
    let mut world = flat_world();
    // Player truly airborne, 16 voxels above the floor (y=0).
    let mut player = Player::new([64.0, 16.0, 64.0]);
    player.yaw = 0.0;
    let mut ctrl = PlayerController::new();
    // A plain forward step in mid-air.
    ctrl.step(&mut world, &mut player, Input::forward(), 0.1);
    assert!(
        !player.on_ground,
        "horizontal move in mid-air must not set on_ground (would allow mid-air jump)"
    );
    // A jump input while airborne must be ignored (no upward lift).
    let before_y = player.pos[1];
    ctrl.step(
        &mut world,
        &mut player,
        Input {
            forward: true,
            jump: true,
            ..Default::default()
        },
        0.1,
    );
    assert!(
        player.pos[1] <= before_y + 1e-3,
        "jump while airborne must not lift the player (got y={:.2}, was {:.2})",
        player.pos[1],
        before_y
    );
}
