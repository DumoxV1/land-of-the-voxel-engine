//! S-01-hardening tests (strict TDD — written before implementation, run RED first).
//!
//! These exercise the three chunk states required by ADR-0001:
//! - `Uniform`: single shared material, zero storage.
//! - `PalettePacked`: dense voxel grid with <=16 distinct materials, bitpacked 4-bit IDs
//!   (2 voxels per byte) backed by a per-chunk palette (max 16 entries).
//! - `Dense`: dense voxel grid with >16 distinct materials (1 byte per voxel).
//!
//! Running `cargo test` before implementation yields compile/assert failures (RED). After
//! minimal implementation they go GREEN. The public API referenced here may not yet exist.

use voxel_core::chunk::{Chunk, ChunkState};
use voxel_core::coords::ChunkCoord;
use voxel_core::palette::MaterialId;

const CHUNK_SIZE: u8 = 32;

fn mat(v: u8) -> MaterialId {
    MaterialId::from(v)
}

#[test]
fn uniform_state_has_zero_dense_storage() {
    // A uniform chunk must NOT allocate dense/palette storage.
    let chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat(0));
    assert_eq!(chunk.state(), ChunkState::Uniform);
    assert!(chunk.dense_data().is_none(), "uniform chunk holds no dense data");
    assert!(chunk.palette().is_none(), "uniform chunk holds no palette");
}

#[test]
fn palette_packed_state_within_16_materials() {
    // Setting up to 16 distinct materials in a chunk should transition to PalettePacked.
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat(0));
    for i in 0..16u8 {
        // place material i at a distinct voxel.
        let x = i % CHUNK_SIZE;
        let y = (i / CHUNK_SIZE) % CHUNK_SIZE;
        let z = if i >= CHUNK_SIZE { 1 } else { 0 };
        if x == 0 && y == 0 && z == 0 {
            // voxel (0,0,0) is the uniform baseline; skip to avoid overwriting baseline.
            continue;
        }
        chunk.set(
            voxel_core::coords::LocalVoxel::new(x, y, z),
            mat(i),
        );
    }
    assert_eq!(
        chunk.state(),
        ChunkState::PalettePacked,
        "chunk with <=16 distinct materials must be PalettePacked"
    );
    // Palette packed storage is half the size of dense: 16 voxels per byte -> N^3 / 2 bytes.
    let packed = chunk.packed_data().expect("palette-packed chunk exposes packed bytes");
    let n = (CHUNK_SIZE as usize).pow(3);
    assert_eq!(packed.len(), n.div_ceil(2), "4-bit packing uses N^3/2 bytes");
    // A palette must back the packed data.
    let palette = chunk.palette().expect("palette-packed chunk exposes a palette");
    assert!(palette.len() <= 16, "palette never exceeds 16 entries");
}

#[test]
fn bitpacking_round_trips_two_voxels_per_byte() {
    // flat(x,y,z) = x*N*N + y*N + z. So flat 0 = (0,0,0), flat 1 = (0,0,1), flat 2 = (0,0,2).
    // Even flat -> low nibble of its byte; odd flat -> high nibble.
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat(0));
    // voxel (0,0,1) gets material index 1, voxel (0,0,2) gets material index 2.
    chunk.set(voxel_core::coords::LocalVoxel::new(0, 0, 1), mat(1));
    chunk.set(voxel_core::coords::LocalVoxel::new(0, 0, 2), mat(2));
    let packed = chunk.packed_data().expect("packed data present");
    // Byte 0 holds flat 0 (low) and flat 1 (high); byte 1 holds flat 2 (low).
    let first = packed[0];
    let lo = first & 0x0F;
    let hi = (first & 0xF0) >> 4;
    assert_eq!(lo, 0, "voxel flat 0 (0,0,0) is the uniform baseline (index 0)");
    assert_eq!(
        hi, 1,
        "voxel flat 1 (0,0,1) maps to high nibble = palette index of material 1"
    );
    let second = packed[1];
    assert_eq!(
        second & 0x0F, 2,
        "voxel flat 2 (0,0,2) maps to low nibble = palette index of material 2"
    );
    // Reading back must reconstruct the exact materials.
    assert_eq!(chunk.get(voxel_core::coords::LocalVoxel::new(0, 0, 0)), mat(0));
    assert_eq!(chunk.get(voxel_core::coords::LocalVoxel::new(0, 0, 1)), mat(1));
    assert_eq!(chunk.get(voxel_core::coords::LocalVoxel::new(0, 0, 2)), mat(2));
}

#[test]
fn dense_state_when_more_than_16_materials() {
    // Introducing a 17th distinct material must promote the chunk to Dense (1 byte/voxel).
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat(0));
    for i in 0..17u8 {
        let x = i % CHUNK_SIZE;
        let y = (i / CHUNK_SIZE) % CHUNK_SIZE;
        let z = 0u8; // i < 17 here, so the spreading stays on the x line; z is fixed.
        if x == 0 && y == 0 && z == 0 {
            continue;
        }
        chunk.set(voxel_core::coords::LocalVoxel::new(x, y, z), mat(i));
    }
    assert_eq!(
        chunk.state(),
        ChunkState::Dense,
        "chunk with >16 distinct materials must be Dense"
    );
    let dense = chunk.dense_data().expect("dense chunk exposes dense data");
    let n = (CHUNK_SIZE as usize).pow(3);
    assert_eq!(dense.len(), n, "dense storage is one byte per voxel");
}

#[test]
fn palette_packed_serialization_round_trip() {
    // A palette-packed chunk must serialize/deserialize byte-stably and round-trip exactly.
    let mut chunk = Chunk::uniform(ChunkCoord::new(3, -2, 1), mat(0));
    for i in 1..8u8 {
        let x = (i * 3) % CHUNK_SIZE;
        let y = (i * 5) % CHUNK_SIZE;
        let z = (i * 7) % CHUNK_SIZE;
        chunk.set(voxel_core::coords::LocalVoxel::new(x, y, z), mat(i));
    }
    assert_eq!(chunk.state(), ChunkState::PalettePacked);
    let bytes = chunk.to_bytes();
    let restored = Chunk::from_bytes(&bytes).expect("deserialize must succeed");
    assert_eq!(chunk, restored, "deserialized palette-packed chunk equals original");
    assert_eq!(bytes, restored.to_bytes(), "serialization must remain byte-stable");
}

#[test]
fn dense_serialization_round_trip() {
    // A dense chunk must serialize/deserialize byte-stably and round-trip exactly.
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), mat(0));
    for i in 1..20u8 {
        let x = (i * 3) % CHUNK_SIZE;
        let y = (i * 5) % CHUNK_SIZE;
        let z = (i * 7) % CHUNK_SIZE;
        chunk.set(voxel_core::coords::LocalVoxel::new(x, y, z), mat(i));
    }
    assert_eq!(chunk.state(), ChunkState::Dense);
    let bytes = chunk.to_bytes();
    let restored = Chunk::from_bytes(&bytes).expect("deserialize must succeed");
    assert_eq!(chunk, restored, "deserialized dense chunk equals original");
    assert_eq!(bytes, restored.to_bytes(), "serialization must remain byte-stable");
}

#[test]
fn uniform_serialization_round_trip() {
    // Uniform must remain byte-stable and round-trip exactly (regression guard).
    let chunk = Chunk::uniform(ChunkCoord::new(-1, 4, 2), mat(7));
    let bytes = chunk.to_bytes();
    let restored = Chunk::from_bytes(&bytes).expect("deserialize must succeed");
    assert_eq!(chunk, restored);
    assert_eq!(bytes, restored.to_bytes());
}
