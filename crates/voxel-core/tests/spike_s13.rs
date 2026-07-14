//! S-13 RED phase: assert the micro-voxel resolution is 12,5 cm (1/8 m)
//! and that 1 km² therefore spans 62.500 chunks (250 per side, 4 m chunks).
//! This test must FAIL to compile before the constants exist (RED).

use voxel_core::coords::{chunk_m_size, CHUNK_SIZE, VOXEL_SIZE_M};

#[test]
fn voxel_is_12_5_cm() {
    // 1 voxel = 0.125 m = 12.5 cm (the user's target band 9.5-13.5 cm).
    assert!((VOXEL_SIZE_M - 0.125).abs() < 1e-6, "voxel must be 0.125 m");
    // Chunk stays 32 voxels -> 4 m (not the old 32 m).
    assert!((chunk_m_size() - 4.0).abs() < 1e-6, "chunk must be 4 m");
}

#[test]
fn one_km2_is_62500_chunks() {
    // 1000 m / 4 m = 250 chunks per side -> 250^2 = 62_500 chunks per km^2.
    let per_side = (1000.0_f32 / chunk_m_size()).round() as i64;
    assert_eq!(per_side, 250, "1 km per side must be 250 chunks");
    let total = per_side * per_side;
    assert_eq!(total, 62_500, "1 km^2 must be 62_500 chunks");
    // Sanity: still a 32^3 voxel chunk internally.
    assert_eq!(CHUNK_SIZE, 32);
}
