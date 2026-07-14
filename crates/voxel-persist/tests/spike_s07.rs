//! S-07 persistence tests (strict TDD — written before implementation, run RED first).
//! References `voxel_persist::{save_world, load_world, PersistError}`, which may not yet exist.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_edit::EditTool;
use voxel_world::World;

#[test]
fn save_then_load_reproduces_world() {
    // Build a world with seed + edits, save, load into a fresh process-shaped state, compare.
    let mut world = World::new(21);
    let mut tool = EditTool::new();
    let spots = [
        WorldVoxel::new(4, 5, 6),
        WorldVoxel::new(12, 1, 9),
        WorldVoxel::new(25, 7, 2),
    ];
    for (i, wv) in spots.iter().enumerate() {
        tool.place(&mut world, *wv, MaterialId::from((i + 1) as u8 * 3), 1, i as u64);
    }
    let log = tool.into_log();

    let path = std::env::temp_dir().join("voxel_persist_test.bin");
    voxel_persist::save_world(&world, &log, &path).expect("save must succeed");

    let (mut loaded_world, loaded_log) = voxel_persist::load_world(&path).expect("load must succeed");

    for wv in spots.iter() {
        let orig = world
            .get_or_generate(voxel_core::coords::ChunkCoord::from_world(*wv))
            .get(voxel_core::coords::LocalVoxel::from_world(*wv));
        let got = loaded_world
            .get_or_generate(voxel_core::coords::ChunkCoord::from_world(*wv))
            .get(voxel_core::coords::LocalVoxel::from_world(*wv));
        assert_eq!(orig, got, "loaded world must match original at {wv:?}");
    }
    // Also confirm the base terrain still matches (deterministic regeneration).
    let probe = WorldVoxel::new(1, 1, 1);
    let orig_base = world
        .get_or_generate(voxel_core::coords::ChunkCoord::from_world(probe))
        .get(voxel_core::coords::LocalVoxel::from_world(probe));
    let loaded_base = loaded_world
        .get_or_generate(voxel_core::coords::ChunkCoord::from_world(probe))
        .get(voxel_core::coords::LocalVoxel::from_world(probe));
    assert_eq!(orig_base, loaded_base, "base terrain must regenerate identically");

    let _ = loaded_log; // checked below
}

#[test]
fn loaded_log_contains_all_edits() {
    let mut world = World::new(33);
    let mut tool = EditTool::new();
    let spots = [WorldVoxel::new(2, 2, 2), WorldVoxel::new(9, 3, 4)];
    for (i, wv) in spots.iter().enumerate() {
        tool.place(&mut world, *wv, MaterialId::from((i + 2) as u8), 1, i as u64);
    }
    let log = tool.into_log();

    let path = std::env::temp_dir().join("voxel_persist_log_test.bin");
    voxel_persist::save_world(&world, &log, &path).unwrap();
    let (_, loaded_log) = voxel_persist::load_world(&path).unwrap();

    assert_eq!(loaded_log.len(), 2, "loaded log must keep all edits");
    let revs: Vec<u64> = loaded_log.edits().iter().map(|e| e.revision).collect();
    assert_eq!(revs, vec![0, 1], "revisions must be 0,1 after round-trip");
    // The recorded `new` values must survive.
    let news: Vec<u8> = loaded_log.edits().iter().map(|e| e.new.0).collect();
    assert_eq!(news, vec![2, 3], "edit new-values must survive round-trip");
}

#[test]
fn corrupt_input_returns_error() {
    let path = std::env::temp_dir().join("voxel_persist_corrupt.bin");
    // Write garbage (wrong magic, too short).
    std::fs::write(&path, b"NOPE_not_a_voxel_save").unwrap();
    let res = voxel_persist::load_world(&path);
    assert!(res.is_err(), "corrupt file must return Err, not panic");
    // Also: a too-short valid-magic file.
    std::fs::write(&path, b"VWL1").unwrap();
    let res2 = voxel_persist::load_world(&path);
    assert!(res2.is_err(), "truncated file must return Err, not panic");
}
