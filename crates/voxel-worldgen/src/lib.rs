//! voxel-worldgen: deterministic seeded world generation (S-04 spike).
//!
//! Generates `voxel_core::Chunk`s from a seed + `ChunkCoord`. The heightmap is a pure
//! function of world X/Z, so adjacent chunks join seamlessly (no cracks at borders).
//! Renderer-agnostic: depends only on `voxel-core`.

use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, LocalVoxel};
use voxel_core::palette::MaterialId;

/// Side length of a chunk in voxels (mirrors `voxel_core::coords::CHUNK_SIZE`).
const SIZE: i64 = voxel_core::coords::CHUNK_SIZE;

/// Material indices used by the generator.
const AIR: u8 = 0;
const DIRT: u8 = 1;
const GRASS: u8 = 2;
const STONE: u8 = 3;

/// Generate a deterministic chunk for the given coord + seed.
///
/// The terrain height is a pure function of world X/Z (value noise), so adjacent chunks form
/// one continuous surface. Same (coord, seed) always yields an identical chunk.
pub fn generate_chunk(coord: ChunkCoord, seed: u32) -> Chunk {
    let mut chunk = Chunk::uniform(coord, MaterialId::from(AIR));
    let origin = coord.world_voxel(LocalVoxel::new(0, 0, 0)); // world pos of chunk (0,0,0)
    for lx in 0..SIZE as u8 {
        for lz in 0..SIZE as u8 {
            let wx = origin.x + lx as i64;
            let wz = origin.z + lz as i64;
            let h = height(wx, wz, seed); // surface height in world-Y
            for ly in 0..SIZE as u8 {
                let wy = origin.y + ly as i64;
                let m = classify(wy, h);
                if m != AIR {
                    chunk.set(LocalVoxel::new(lx, ly, lz), MaterialId::from(m));
                }
            }
        }
    }
    chunk
}

/// Classify a world-Y column into a material given the surface height `h` (world-Y).
fn classify(wy: i64, h: i64) -> u8 {
    if wy > h {
        AIR
    } else if wy == h {
        GRASS
    } else if wy >= h - 3 {
        DIRT
    } else {
        STONE
    }
}

/// Surface height in world-Y for a world (x, z), in [0, SIZE-1].
///
/// 2D value noise: hash a coarse integer grid, bilinearly interpolate, scale to chunk height.
/// Pure function of (x, z, seed) → deterministic and seamless across chunk borders.
fn height(x: i64, z: i64, seed: u32) -> i64 {
    const N: i64 = 8; // noise grid period in voxels
    let scale = (SIZE - 1) as f32;
    let gx = x.div_euclid(N);
    let gz = z.div_euclid(N);
    let fx = (x.rem_euclid(N)) as f32 / (N as f32);
    let fz = (z.rem_euclid(N)) as f32 / (N as f32);

    let v00 = hash2(gx, gz, seed);
    let v10 = hash2(gx + 1, gz, seed);
    let v01 = hash2(gx, gz + 1, seed);
    let v11 = hash2(gx + 1, gz + 1, seed);

    let sx = smooth(fx);
    let sz = smooth(fz);
    let top = lerp(v00, v10, sx);
    let bot = lerp(v01, v11, sx);
    let n = lerp(top, bot, sz); // in [0,1]
    (n * scale).round().clamp(0.0, (SIZE - 1) as f32) as i64
}

/// Deterministic 2D hash → [0,1).
fn hash2(x: i64, z: i64, seed: u32) -> f32 {
    let mut h: u32 = seed ^ 0x9E37_9B9;
    h = h.wrapping_add((x as u32).wrapping_mul(0x85EB_CA6B));
    h = h.wrapping_add((z as u32).wrapping_mul(0xC2B2_AE35));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / 16_777_216.0 // [0,1)
}

#[inline]
fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
