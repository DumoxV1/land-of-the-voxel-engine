//! S-11 audit-hardening tests (strict TDD — RED before fixes).
//!
//! Audit findings P-01: unbounded fall speed tunnels through thin floors (substep
//! displacement > 1 voxel), and `resolve_floor_y` samples only one column while the
//! AABB (0.6 wide) can overlap four.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_player::{Input, Player, PlayerController};
use voxel_world::World;

/// Build a controlled world: air everywhere in the test region, with a floor as specified.
fn empty_region(w: &mut World, x0: i64, x1: i64, y0: i64, y1: i64, z0: i64, z1: i64) {
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                w.set_voxel(WorldVoxel::new(x, y, z), MaterialId::from(0));
            }
        }
    }
}

/// Falling from very high onto a 1-voxel-thick plateau must land on it, not tunnel through.
#[test]
fn high_fall_lands_on_thin_floor() {
    let mut world = World::new(0);
    // Clear a tall shaft and put a single-thick floor at y = 10.
    empty_region(&mut world, 6, 12, 0, 32, 6, 12);
    for x in 6..12 {
        for z in 6..12 {
            world.set_voxel(WorldVoxel::new(x, 10, z), MaterialId::from(3));
        }
    }
    let mut player = Player::new([8.5, 500.0, 8.5]);
    let mut ctrl = PlayerController::new();
    // Fall for 30 simulated seconds (plenty to reach terminal velocity and land).
    for _ in 0..300 {
        ctrl.step(&mut world, &mut player, Input::none(), 0.1);
    }
    assert!(
        player.on_ground,
        "player must come to rest, pos = {:?}",
        player.pos
    );
    let feet = player.pos[1] - 0.9; // HALF[1]
    assert!(
        (feet - 11.0).abs() < 0.1,
        "feet must rest on top of the y=10 floor (y=11), got feet={feet} pos={:?}",
        player.pos
    );
}

/// Standing on a block edge: the AABB overlaps a neighbouring column with a higher floor.
/// `resolve_floor_y` must use the highest solid top under the whole footprint, not just
/// the column under the player's center.
#[test]
fn floor_resolve_covers_full_footprint() {
    let mut world = World::new(0);
    empty_region(&mut world, 6, 14, 0, 32, 6, 14);
    // Low floor everywhere at y=4; a higher step at x=10..12 at y=6.
    for x in 6..14 {
        for z in 6..14 {
            world.set_voxel(WorldVoxel::new(x, 4, z), MaterialId::from(3));
        }
    }
    for x in 10..12 {
        for z in 6..14 {
            world.set_voxel(WorldVoxel::new(x, 6, z), MaterialId::from(3));
        }
    }
    // Player center in column x=9 but hitbox (half-width 0.3) overlapping x=10 (the step).
    let mut player = Player::new([9.9, 12.0, 8.5]);
    let mut ctrl = PlayerController::new();
    for _ in 0..100 {
        ctrl.step(&mut world, &mut player, Input::none(), 0.1);
    }
    assert!(player.on_ground, "must land, pos = {:?}", player.pos);
    let feet = player.pos[1] - 0.9;
    // The hitbox overlaps the y=6 step, so feet must rest on its top (y=7) —
    // resting at y=5 means the resolver ignored the overlapped column (audit bug).
    assert!(
        (feet - 7.0).abs() < 0.1,
        "feet must rest on the step top y=7 (footprint overlaps it), got feet={feet} pos={:?}",
        player.pos
    );
}
