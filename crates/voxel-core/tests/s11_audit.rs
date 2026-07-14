//! S-11 audit-hardening tests (strict TDD — RED before fixes).
//!
//! Audit findings C-01: malformed palette-packed payloads (nibble >= palette_len) must
//! return `Err`, not deserialize into a chunk that panics on `get`. Chunk coords must
//! round-trip at full i64 range (the audit found silent i32 truncation).

use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, LocalVoxel, CHUNK_SIZE};
use voxel_core::palette::MaterialId;

fn packed_chunk() -> Chunk {
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
    // Two materials -> PalettePacked with palette [0, 5].
    chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(5u8));
    chunk
}

/// A payload whose packed nibbles index past the palette must be rejected with Err —
/// never accepted and left to panic later in `Chunk::get`.
#[test]
fn malformed_nibble_out_of_palette_range_is_err() {
    let chunk = packed_chunk();
    let mut bytes = chunk.to_bytes();
    // Header: version(1) + i64 coords(24) + state(1) + uniform(1) = 27, then palette_len(1).
    let plen = bytes[27] as usize;
    assert!(plen < 16, "test setup: palette should be small");
    // Corrupt the first packed byte: both nibbles = 15, far beyond palette_len.
    bytes[27 + 1 + plen] = 0xFF;
    let res = Chunk::from_bytes(&bytes);
    assert!(
        res.is_err(),
        "packed nibble >= palette_len must be a deserialize Err, got {res:?}"
    );
}

/// Truncated dense body must be rejected (regression guard for length checks).
#[test]
fn truncated_dense_body_is_err() {
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
    // Force dense: more than 16 distinct materials.
    for i in 0..CHUNK_SIZE.min(20) {
        chunk.set(
            LocalVoxel::new(i as u8, 0, 0),
            MaterialId::from((i + 1) as u8),
        );
    }
    let mut bytes = chunk.to_bytes();
    bytes.truncate(bytes.len() - 7);
    assert!(Chunk::from_bytes(&bytes).is_err());
}

/// Chunk coordinates beyond i32 range must round-trip exactly (not silently truncate).
#[test]
fn chunk_coord_i64_round_trips() {
    let big = 1i64 << 40;
    let chunk = Chunk::uniform(ChunkCoord::new(big, -big, big + 3), MaterialId::from(2u8));
    let bytes = chunk.to_bytes();
    let restored = Chunk::from_bytes(&bytes).expect("deserialize");
    assert_eq!(
        restored.coord,
        ChunkCoord::new(big, -big, big + 3),
        "chunk coords must round-trip at full i64 range"
    );
    assert_eq!(bytes, restored.to_bytes(), "byte-stable at i64 coords");
}
