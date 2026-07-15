//! S-04 worldgen tests (strict TDD — written before implementation, run RED first).
//! References `voxel_worldgen::generate_chunk`, which may not yet exist. Running before
//! implementation yields compile errors (RED). After minimal implementation they go GREEN.

use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, VOXEL_SIZE_M};
use voxel_core::palette::MaterialId;
use voxel_worldgen::{generate_chunk, surface_height_m};

const CHUNK_SIZE: u8 = 32;

#[test]
fn deterministic_same_seed_same_chunk() {
    // Determinism: same seed + coord must yield a byte-identical chunk across calls.
    let c = ChunkCoord::new(3, 0, -2);
    let a = generate_chunk(c, 12345);
    let b = generate_chunk(c, 12345);
    assert_eq!(a, b, "generate_chunk must be deterministic for a fixed seed+coord");
    // And round-trips through serialization identically.
    assert_eq!(a.to_bytes(), b.to_bytes(), "serialized deterministic chunk must match");
}

#[test]
fn different_seed_different_chunk() {
    // Different seeds should produce different terrain for the same coord. Scan Y until we
    // find a chunk that actually carries terrain for seed 0 (the deep (0,0,0) slab is empty
    // below bedrock for every seed, so a fixed coord would make the test meaningless).
    let mut surface = None;
    for cy in 0..16i64 {
        let c = ChunkCoord::new(0, cy, 0);
        // Compare against an empty chunk with the SAME coord, so a genuinely empty chunk
        // (Uniform AIR, no solid voxels) is detected as empty regardless of coord.
        let empty = Chunk::uniform(c, MaterialId::from(0));
        if generate_chunk(c, 0) != empty {
            surface = Some(c);
            break;
        }
    }
    let c = surface.expect("expected a non-empty surface chunk for seed 0");
    let a = generate_chunk(c, 0);
    let b = generate_chunk(c, 999);
    assert_ne!(a, b, "different seeds should yield different chunks");
}

#[test]
fn chunk_boundary_continuous() {
    // Adjacent chunks must join seamlessly: the heightmap is a pure function of world X/Z, so
    // the step in surface height across a chunk border must be no larger than the step *within*
    // a chunk. (Chunk A local x=31 is world X=31; chunk B local x=0 is world X=32 — they are
    // neighbouring world columns, not the same one, so they legitimately differ.)
    let ca = ChunkCoord::new(0, 0, 0);
    let cb = ChunkCoord::new(1, 0, 0);
    let a = generate_chunk(ca, 7);
    let b = generate_chunk(cb, 7);

    // Surface height (world-Y of the topmost solid voxel) for a (local_x, local_z) in a chunk.
    let top = |chunk: &Chunk, lx: u8, lz: u8| -> i32 {
        for ly in (0..CHUNK_SIZE).rev() {
            if chunk.get(voxel_core::coords::LocalVoxel::new(lx, ly, lz)) != MaterialId::from(0) {
                return ly as i32;
            }
        }
        -1
    };

    for z in 0..CHUNK_SIZE {
        let ha = top(&a, 31, z); // world X = 31 (last column of A)
        let hb = top(&b, 0, z); // world X = 32 (first column of B)
        let intra_a = (top(&a, 31, z) - top(&a, 30, z)).abs(); // step within A (X 30->31)
        let intra_b = (top(&b, 1, z) - top(&b, 0, z)).abs(); // step within B (X 32->33)
        let border = (ha - hb).abs(); // step across the chunk border (X 31->32)
        assert!(
            border <= intra_a.max(intra_b).max(1),
            "height step across chunk border (X31->X32, z={z}) must not exceed intra-chunk steps"
        );
    }
}

#[test]
fn non_empty_chunk() {
    // A generated chunk ON THE SURFACE should contain at least one solid voxel (no
    // empty-world regressions). BEDROCK_DEPTH truncates deep chunks to AIR, so we must
    // sample the chunk that actually contains the terrain surface, not a deep one.
    let seed = 0u32;
    let h_vox = (surface_height_m(0, 0, seed) / VOXEL_SIZE_M) as i64;
    let surface_cy = h_vox / 32;
    let chunk = generate_chunk(ChunkCoord::new(0, surface_cy, 0), seed);
    let mut solid = 0u32;
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                if chunk.get(voxel_core::coords::LocalVoxel::new(x, y, z)) != MaterialId::from(0) {
                    solid += 1;
                }
            }
        }
    }
    assert!(solid > 0, "surface chunk must contain solid voxels");
}

#[test]
fn material_layers_are_sane() {
    // Topmost solid voxel in a column is a valid surface material (grass/sand/snow/stone);
    // air(0) is above it; the column below is solid. Biome selects the exact surface mat.
    // Sample the chunk that actually contains the surface (BEDROCK truncates deep chunks).
    let seed = 42u32;
    let h_vox = (surface_height_m(2, 2, seed) / VOXEL_SIZE_M) as i64;
    let surface_cy = h_vox / 32;
    let chunk = generate_chunk(ChunkCoord::new(2, surface_cy, 2), seed);
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let mut top = None;
            for y in (0..CHUNK_SIZE).rev() {
                let m = chunk.get(voxel_core::coords::LocalVoxel::new(x, y, z));
                if m != MaterialId::from(0) {
                    top = Some((y, m));
                    break;
                }
            }
            if let Some((ty, tm)) = top {
                let id = tm.as_u8();
                assert!(
                    matches!(id, 2 | 7 | 8 | 3),
                    "top solid voxel must be a surface material (grass/sand/snow/stone), got {id} at x={x} z={z}"
                );
                // Directly above the top must be air.
                if ty + 1 < CHUNK_SIZE {
                    assert_eq!(
                        chunk.get(voxel_core::coords::LocalVoxel::new(x, ty + 1, z)),
                        MaterialId::from(0),
                        "air must sit above the surface at x={x} z={z}"
                    );
                }
            }
        }
    }
}
