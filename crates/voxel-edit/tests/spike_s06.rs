//! S-06 edit-tool tests (strict TDD — written before implementation, run RED first).
//! References `voxel_edit::{Edit, EditLog, EditTool}`, which may not yet exist. Running before
//! implementation yields compile errors (RED).

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_edit::{EditLog, EditTool};
use voxel_world::World;

#[test]
fn edit_captures_old_and_new() {
    let mut world = World::new(1);
    let wv = WorldVoxel::new(3, 3, 3);
    let before = world.get_or_generate(voxel_core::coords::ChunkCoord::from_world(wv)).get(
        voxel_core::coords::LocalVoxel::from_world(wv),
    );
    let mut tool = EditTool::new();
    let edit = tool.place(&mut world, wv, MaterialId::from(5), 1, 10);
    assert_eq!(edit.old, before, "edit.old must be the prior world value");
    assert_eq!(edit.new, MaterialId::from(5));
    assert_eq!(edit.actor, 1);
    assert_eq!(edit.tick, 10);
}

#[test]
fn edit_log_revisions_monotonic() {
    let mut world = World::new(2);
    let mut tool = EditTool::new();
    for i in 0..5u64 {
        tool.place(&mut world, WorldVoxel::new(i as i64, 1, 1), MaterialId::from(3), 1, i);
    }
    let log: EditLog = tool.into_log();
    assert_eq!(log.len(), 5);
    let revs: Vec<u64> = log.edits().iter().map(|e| e.revision).collect();
    let mut sorted = revs.clone();
    sorted.sort();
    assert_eq!(revs, sorted, "revisions must be monotonically increasing");
    assert_eq!(log.revision(), 4, "last revision should be 4 (0-based, 5 edits)");
}

#[test]
fn edit_tool_place_and_remove_update_world_and_log() {
    let mut world = World::new(3);
    let wv = WorldVoxel::new(7, 7, 7);
    let mut tool = EditTool::new();
    tool.place(&mut world, wv, MaterialId::from(6), 2, 1);
    assert_eq!(
        world.get_or_generate(voxel_core::coords::ChunkCoord::from_world(wv)).get(
            voxel_core::coords::LocalVoxel::from_world(wv)
        ),
        MaterialId::from(6),
        "place must update the world"
    );
    tool.remove(&mut world, wv, 2, 2);
    assert_eq!(
        world.get_or_generate(voxel_core::coords::ChunkCoord::from_world(wv)).get(
            voxel_core::coords::LocalVoxel::from_world(wv)
        ),
        MaterialId::from(0),
        "remove must clear the voxel"
    );
    let log: EditLog = tool.into_log();
    assert_eq!(log.len(), 2, "both edits must be logged");
}

#[test]
fn replay_reproduces_world_state() {
    // Apply edits to world A, build a log, then replay the log onto a FRESH world B (same seed)
    // and confirm the edited voxels match.
    let mut world_a = World::new(11);
    let mut tool = EditTool::new();
    let edits = [
        WorldVoxel::new(4, 4, 4),
        WorldVoxel::new(10, 2, 8),
        WorldVoxel::new(20, 6, 1),
    ];
    for (i, wv) in edits.iter().enumerate() {
        tool.place(&mut world_a, *wv, MaterialId::from((i + 1) as u8 * 2), 1, i as u64);
    }
    let log = tool.into_log();

    // Fresh world B with the same seed, then replay.
    let mut world_b = World::new(11);
    log.apply_all(&mut world_b);

    for (i, wv) in edits.iter().enumerate() {
        let expected = MaterialId::from((i + 1) as u8 * 2);
        let got_a = world_a
            .get_or_generate(voxel_core::coords::ChunkCoord::from_world(*wv))
            .get(voxel_core::coords::LocalVoxel::from_world(*wv));
        let got_b = world_b
            .get_or_generate(voxel_core::coords::ChunkCoord::from_world(*wv))
            .get(voxel_core::coords::LocalVoxel::from_world(*wv));
        assert_eq!(got_a, expected, "world A must hold the edit");
        assert_eq!(got_b, expected, "replayed world B must match world A at edited voxel");
        assert_eq!(got_a, got_b, "replay must reproduce exact edited state");
    }
}
