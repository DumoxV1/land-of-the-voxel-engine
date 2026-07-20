//! I1 (live edit) raycast-tests — TDD: geschreven vóór `raycast_voxel` bestond (RED),
//! daarna bijgesteld naar realistische assertions (World is niet leeg: terrain vult de
//! ondergrond, dus we testen op "raakt iets lager" i.p.v. een specifieke geplaatste voxel).
//! Tests de Amanatides-Woo voxel-DDA tegen een `World`.

use voxel_core::coords::WorldVoxel;
use voxel_edit::raycast_voxel;
use voxel_world::World;

#[test]
fn raycast_hits_terrain_below() {
    // Schiet van zeer hoog (625 m, boven elke surface die max ~477 m is) omlaag → raakt terrain.
    let mut world = World::new(1);
    let origin = WorldVoxel::new(0, 5000, 0);
    let dir = [0.0f32, -1.0, 0.0];
    let hit = raycast_voxel(&mut world, origin, dir, 10000.0);
    assert!(hit.is_some(), "ray omlaag moet terrain raken");
    let (h, _n) = hit.unwrap();
    assert!(h.y < origin.y, "hit moet lager dan de start zijn");
    assert!(world.material_at(h).0 != 0, "hit-voxel moet solid zijn");
}

#[test]
fn raycast_misses_in_empty_high_sky() {
    // Ver boven elke terrain (max ~477 m = 3816 vox) → geen solid binnen bereik.
    let mut world = World::new(2);
    let origin = WorldVoxel::new(0, 100_000, 0);
    let dir = [0.0f32, -1.0, 0.0];
    let hit = raycast_voxel(&mut world, origin, dir, 10.0);
    assert!(hit.is_none(), "ray in lege hoge lucht raakt niets");
}

#[test]
fn raycast_normal_points_back_at_origin() {
    // Hit van boven (omlaag) → normal moet +Y wijzen (terug naar de ray-origin).
    let mut world = World::new(3);
    let origin = WorldVoxel::new(0, 5000, 0);
    let dir = [0.0f32, -1.0, 0.0];
    let (_h, n) = raycast_voxel(&mut world, origin, dir, 10000.0).unwrap();
    assert_eq!(n, WorldVoxel::new(0, 1, 0), "normal wijst terug omhoog (+Y)");
}
