//! Integration tests for S-01 `voxel-core` (strict TDD — written before implementation).
//! These reference the public API that must exist. Running `cargo test` before
//! implementation yields a compile failure (RED). After minimal implementation they go GREEN.

use voxel_core::coords::{ChunkCoord, LocalVoxel, WorldVoxel, CHUNK_SIZE};
use voxel_core::chunk::{Chunk, ChunkState};
use voxel_core::edit::{Edit, EditId};
use voxel_core::palette::MaterialId;
use voxel_core::serialize::round_trip;

#[test]
fn world_to_chunk_to_local_roundtrip_positive() {
    // (x, y, z) within a single chunk maps back to itself.
    let w = WorldVoxel::new(40, 5, -12);
    let chunk = ChunkCoord::from_world(w);
    let local = LocalVoxel::from_world(w);
    let rebuilt = chunk.world_voxel(local);
    assert_eq!(w, rebuilt, "positive roundtrip must be exact");
}

#[test]
fn world_to_chunk_correct_for_negative_euclidean() {
    // Negative coordinates must use Euclidean division so the local index stays in [0, CHUNK_SIZE).
    // e.g. world x = -1 -> chunk x = -1, local x = CHUNK_SIZE-1 (not 0 with truncating div).
    let w = WorldVoxel::new(-1, -1, -1);
    let chunk = ChunkCoord::from_world(w);
    let local = LocalVoxel::from_world(w);
    assert!(local.x < CHUNK_SIZE as u8 && local.y < CHUNK_SIZE as u8 && local.z < CHUNK_SIZE as u8,
        "local indices must stay in range for negatives");
    assert_eq!(chunk.world_voxel(local), w, "negative roundtrip must be exact");
    // Explicit Euclidean check: a voxel just below a chunk boundary.
    let w2 = WorldVoxel::new(-CHUNK_SIZE as i64, 0, 0);
    let c2 = ChunkCoord::from_world(w2);
    assert_eq!(c2.x, -1, "world x = -CHUNK_SIZE belongs to chunk -1");
    let l2 = LocalVoxel::from_world(w2);
    assert_eq!(l2.x, 0, "local x at boundary is 0");
}

#[test]
fn uniform_chunk_get_set_and_state() {
    let mat: MaterialId = 3u8.into();
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat);
    assert_eq!(chunk.state(), ChunkState::Uniform);
    assert_eq!(chunk.get(LocalVoxel::new(0, 0, 0)), mat);
    let new_mat: MaterialId = 7u8.into();
    chunk.set(LocalVoxel::new(1, 2, 3), new_mat);
    assert_eq!(chunk.get(LocalVoxel::new(1, 2, 3)), new_mat);
    // One differing voxel -> two distinct materials (0 and 7) -> PalettePacked (<=16 materials).
    assert_eq!(chunk.state(), ChunkState::PalettePacked);
}

#[test]
fn edit_idempotence_duplicate_rejected() {
    // Applying the same edit (same pos + revision) twice must not advance revision twice.
    let first = Edit::new(
        WorldVoxel::new(1, 2, 3),
        MaterialId::from(0u8),
        MaterialId::from(1u8),
        EditId::new(1),
    );
    let duplicate = first;
    assert!(first.conflicts_with(&duplicate).is_none(),
        "identical edit at same revision is idempotent (no conflict)");
    let later = Edit::new(
        WorldVoxel::new(1, 2, 3),
        MaterialId::from(0u8),
        MaterialId::from(2u8),
        EditId::new(2),
    );
    assert!(first.conflicts_with(&later).is_some(),
        "different new value at same position is a conflict");
}

#[test]
fn serialize_round_trip_byte_stable() {
    let mat: MaterialId = 5u8.into();
    let mut chunk = Chunk::uniform(ChunkCoord::new(2, -1, 0), mat);
    chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(9u8));
    let bytes = chunk.to_bytes();
    let restored = Chunk::from_bytes(&bytes).expect("deserialize must succeed");
    assert_eq!(chunk, restored, "deserialized chunk equals original");
    // Byte-stable: re-serializing yields identical bytes.
    assert_eq!(bytes, restored.to_bytes(), "serialization must be byte-stable");
    // round_trip helper contract.
    let rt = round_trip(&bytes);
    assert_eq!(rt, bytes, "round_trip must preserve bytes");
}

#[cfg(feature = "proptest")]
mod property {
    use super::*;
    use proptest::prelude::*;
    use voxel_core::coords::CHUNK_SIZE;

    proptest! {
        #[test]
        fn world_chunk_local_roundtrip(x in -1_000_000i64..1_000_000, y in -1_000_000i64..1_000_000, z in -1_000_000i64..1_000_000) {
            let w = WorldVoxel::new(x, y, z);
            let chunk = ChunkCoord::from_world(w);
            let local = LocalVoxel::from_world(w);
            prop_assert!(local.x < CHUNK_SIZE as u8 && local.y < CHUNK_SIZE as u8 && local.z < CHUNK_SIZE as u8);
            prop_assert_eq!(chunk.world_voxel(local), w);
        }
    }
}
