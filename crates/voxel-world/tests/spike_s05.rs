//! S-05 world-store tests (strict TDD — written before implementation, run RED first).
//! References `voxel_world::{World, get_or_generate, set_voxel, chunk_at, dirty_chunks`, which
//! may not yet exist. Running before implementation yields compile errors (RED).

use voxel_core::coords::{ChunkCoord, LocalVoxel, WorldVoxel};
use voxel_core::palette::MaterialId;
use voxel_world::World;

#[test]
fn get_or_generate_caches_and_is_deterministic() {
    let mut w = World::new(123);
    let c = ChunkCoord::new(2, 0, -1);
    let a = w.get_or_generate(c);
    let b = w.get_or_generate(c);
    assert_eq!(a, b, "same coord+seed must yield identical cached chunk");
    assert_eq!(a.to_bytes(), b.to_bytes(), "cached chunk must be byte-identical");
}

#[test]
fn set_voxel_writes_to_correct_chunk_and_persists() {
    let mut w = World::new(5);
    // World voxel at chunk (0,0,0) local (10,20,10).
    let wv = WorldVoxel::new(10, 20, 10);
    w.set_voxel(wv, MaterialId::from(4));
    // The edit must show up in the owning chunk.
    let chunk = w.chunk_at(ChunkCoord::from_world(wv));
    assert_eq!(
        chunk.get(LocalVoxel::from_world(wv)),
        MaterialId::from(4),
        "set_voxel must write into the correct chunk at the right local voxel"
    );
    // Re-generating/fetching the same coord must NOT overwrite the edit.
    let chunk2 = w.get_or_generate(ChunkCoord::from_world(wv));
    assert_eq!(
        chunk2.get(LocalVoxel::from_world(wv)),
        MaterialId::from(4),
        "edits must survive get_or_generate (no overwrite by generation)"
    );
}

#[test]
fn adjacent_chunks_join_without_cracks() {
    // Two adjacent chunks in the same World must have the same border-height continuity as S-04.
    let mut w = World::new(7);
    let a = w.get_or_generate(ChunkCoord::new(0, 0, 0));
    let b = w.get_or_generate(ChunkCoord::new(1, 0, 0));

    let top = |chunk: &voxel_core::chunk::Chunk, lx: u8, lz: u8| -> i32 {
        for ly in (0..32u8).rev() {
            if chunk.get(LocalVoxel::new(lx, ly, lz)) != MaterialId::from(0) {
                return ly as i32;
            }
        }
        -1
    };

    for z in 0..32u8 {
        let ha = top(&a, 31, z); // world X = 31
        let hb = top(&b, 0, z); // world X = 32
        let intra_a = (top(&a, 31, z) - top(&a, 30, z)).abs();
        let intra_b = (top(&b, 1, z) - top(&b, 0, z)).abs();
        let border = (ha - hb).abs();
        assert!(
            border <= intra_a.max(intra_b).max(1),
            "world border step (X31->X32, z={z}) must not exceed intra-chunk steps"
        );
    }
}

#[test]
fn set_voxel_marks_chunk_dirty() {
    let mut w = World::new(9);
    let wv = WorldVoxel::new(5, 5, 5);
    let coord = ChunkCoord::from_world(wv);
    w.set_voxel(wv, MaterialId::from(8));
    let dirty = w.dirty_chunks();
    assert!(dirty.contains(&coord), "set_voxel must mark the owning chunk dirty");
}
