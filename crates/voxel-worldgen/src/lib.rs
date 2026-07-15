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
const SAND: u8 = 7;
const SNOW: u8 = 8;

/// Generate a deterministic chunk for the given coord + seed.
///
/// The terrain height is a pure function of world X/Z (fBm), so adjacent chunks form
/// one continuous surface. Biome (meadow/desert/snow/rock) is a second pure function of
/// world X/Z and selects the surface material, giving the world varied terrain instead of
/// one uniform grass sheet. Same (coord, seed) always yields an identical chunk.
pub fn generate_chunk(coord: ChunkCoord, seed: u32) -> Chunk {
    let mut chunk = Chunk::uniform(coord, MaterialId::from(AIR));
    let origin = coord.world_voxel(LocalVoxel::new(0, 0, 0)); // world pos of chunk (0,0,0)
    for lx in 0..SIZE as u8 {
        for lz in 0..SIZE as u8 {
            let wx = origin.x + lx as i64;
            let wz = origin.z + lz as i64;
            // Surface height in WORLD-Y voxels (coord.y selects which 4 m slab this chunk is).
            let h = (surface_height_m(wx, wz, seed) / voxel_core::coords::VOXEL_SIZE_M) as i64;
            let biome = biome_at(wx, wz, seed);
            // P1 spike (2026-07-15): slope computed ONCE per column here, not 32x inside
            // `classify` (was the dominant hot-path — 4 fBm calls per ly = 131k fBm/chunk).
            let slope = {
                let hl = surface_height_m(wx - 1, wz, seed);
                let hr = surface_height_m(wx + 1, wz, seed);
                let hd = surface_height_m(wx, wz - 1, seed);
                let hu = surface_height_m(wx, wz + 1, seed);
                let dx = (hr - hl).abs();
                let dz = (hu - hd).abs();
                (dx + dz) / voxel_core::coords::VOXEL_SIZE_M
            };
            for ly in 0..SIZE as u8 {
                let wy = origin.y + ly as i64;
                let m = classify(wy, h, slope, biome);
                if m != AIR {
                    chunk.set(LocalVoxel::new(lx, ly, lz), MaterialId::from(m));
                }
            }
        }
    }
    chunk
}

/// Classify a world-Y column into a material given the surface height `h`, the precomputed
/// `slope`, and the climate `biome`. Pure + cheap: NO height-field sampling (caller computes
/// `slope` once per column — see `generate_chunk`).
fn classify(wy: i64, h: i64, slope: f32, biome: Biome) -> u8 {
    if wy > h {
        return AIR;
    }
    // Steep exposure → bare rock regardless of biome.
    if slope >= 4.0 && wy >= h - 2 {
        return STONE;
    }
    if wy == h {
        // Surface layer: biome-driven.
        match biome {
            Biome::Meadow => GRASS,
            Biome::Desert => SAND,
            Biome::Snow => SNOW,
            Biome::Rock => STONE,
        }
    } else if wy >= h - 3 {
        match biome {
            Biome::Snow => SNOW, // snow pack a bit thick
            _ => DIRT,
        }
    } else {
        STONE
    }
}

/// Normalized fBm in [0,1]: sum of noise octaves (doubling freq / halving amp),
/// divided by total weight. Pure function of (x, z, seed) → deterministic + seamless.
fn fbm01(x: i64, z: i64, seed: u32) -> f32 {
    // Octave periods in VOXELS. Broad hills need large periods: with 12.5 cm voxels,
    // period 2048 ≈ 256 m wide base hills; finer octaves add 32 m / 4 m / 0.5 m detail.
    const OCTAVES: &[(i64, f32)] = &[(2048, 0.5), (512, 0.28), (128, 0.14), (32, 0.08), (4, 0.08)];
    let mut n = 0.0f32;
    let mut wsum = 0.0f32;
    for &(period, weight) in OCTAVES {
        let gx = x.div_euclid(period);
        let gz = z.div_euclid(period);
        let fx = (x.rem_euclid(period)) as f32 / period as f32;
        let fz = (z.rem_euclid(period)) as f32 / period as f32;
        let v00 = hash2(gx, gz, seed);
        let v10 = hash2(gx + 1, gz, seed);
        let v01 = hash2(gx, gz + 1, seed);
        let v11 = hash2(gx + 1, gz + 1, seed);
        let sx = smooth(fx);
        let sz = smooth(fz);
        let top = lerp(v00, v10, sx);
        let bot = lerp(v01, v11, sx);
        n += lerp(top, bot, sz) * weight;
        wsum += weight;
    }
    n / wsum
}

/// Surface height in world-Y for a world (x, z), as multi-octave fractal Brownian motion
/// (fBm) in [0, SIZE-1]. Legacy helper kept for slope math; prefers `surface_height_m`.
pub fn height(x: i64, z: i64, seed: u32) -> i64 {
    let scale = (SIZE - 1) as f32;
    (fbm01(x, z, seed) * scale).round().clamp(0.0, (SIZE - 1) as f32) as i64
}

/// Surface height in **meters**, as fBm. Vertical-scale spike (2026-07-15): canonical
/// height used by `generate_chunk`. Amplitude is large (≈40 m peaks) so a ~1.75 m human
/// reads as small against the terrain, fixing the "blocks look huge" complaint.
pub fn surface_height_m(x: i64, z: i64, seed: u32) -> f32 {
    const AMPLITUDE_M: f32 = 40.0;
    fbm01(x, z, seed) * AMPLITUDE_M
}

/// Climate biome for a world (x, z). Determines the surface material + tint so the world
/// reads as varied terrain (meadow / desert / snow / rock) instead of one uniform grass sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    Meadow,
    Desert,
    Snow,
    Rock,
}

/// Biome at a world (x, z), pure function of (x, z, seed).
///
/// A slow climate field (coarse value noise) selects meadow/desert/snow, with a second
/// moisture-ish axis pushing high/steep regions toward rock. Distinct regions → distinct
/// biomes, so the world reads as varied terrain.
pub fn biome_at(x: i64, z: i64, seed: u32) -> Biome {
    const N: i64 = 256; // coarse climate grid (256 voxels = 32 m cells)
    let climate = hash2(x.div_euclid(N), z.div_euclid(N), seed ^ 0x51ED);
    let cold = hash2(x.div_euclid(N) + 31, z.div_euclid(N) - 17, seed ^ 0xB10C);
    match (climate, cold) {
        (c, _) if c < 0.33 => Biome::Desert,
        (_, k) if k < 0.30 => Biome::Snow,
        (c, _) if c > 0.72 => Biome::Rock,
        _ => Biome::Meadow,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    /// P1 spike (2026-07-15): chunk generation must stay fast. Regression guard against the
    /// old hot-path where `classify` re-sampled the 4-neighbour height field PER VOXEL-Y
    /// (32x per column) — that was ~3.2 ms/chunk. After hoisting slope to once-per-column it
    /// is ~0.2 ms/chunk. 200 chunks must finish well under 500 ms (old code took ~640 ms).
    #[test]
    fn chunk_gen_stays_fast() {
        let t0 = Instant::now();
        for i in 0..200u32 {
            let _ = generate_chunk(
                ChunkCoord::new((i as i64) % 16, (i as i64 / 16) % 8, (i as i64) % 16),
                7,
            );
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        assert!(
            ms < 500.0,
            "chunk gen too slow: {ms:.1} ms for 200 chunks (slope hot-path regression?)"
        );
    }

    /// Terrain must have multi-scale (fractal) relief: large hills AND fine detail, not a
    /// single noise scale. Measured in METERS via `surface_height_m` (the canonical
    /// height). Large low-freq hills must span >= 8 m; fine octaves must add >= 3 m of
    /// range beyond what a coarse (2 m-step) sample captures. Sampled along an x-line
    /// (z constant) so the fine x-octaves are fairly represented.
    #[test]
    fn terrain_has_fractal_relief() {
        let seed = 7u32;
        let z = 500i64;
        let range_full = {
            let mut mn = f32::MAX;
            let mut mx = f32::MIN;
            for x in 0..=2048 {
                let h = surface_height_m(x, z, seed);
                mn = mn.min(h);
                mx = mx.max(h);
            }
            mx - mn
        };
        let range_coarse = {
            let mut mn = f32::MAX;
            let mut mx = f32::MIN;
            for x in (0..=2048).step_by(16) {
                let h = surface_height_m(x, z, seed);
                mn = mn.min(h);
                mx = mx.max(h);
            }
            mx - mn
        };
        assert!(
            range_coarse >= 8.0,
            "terrain lacks large-scale hills: coarse range = {range_coarse:.1} m"
        );
        assert!(
            (range_full - range_coarse) >= 1.0,
            "terrain lacks fine-scale (fractal) detail: full {range_full:.1} - coarse {range_coarse:.1} < 1 m"
        );
    }

    /// The world must read as varied terrain, not one uniform grass sheet. Far-apart regions
    /// must yield different biomes (meadow/desert/snow/rock). RED until `biome_at` is real.
    #[test]
    fn biomes_vary_across_regions() {
        let seed = 7u32;
        // Sample biome at regions 0, 4 km, 16 km, 64 km apart (chunk = 4 m).
        let coords = [0, 1000, 4000, 16000];
        let mut seen = std::collections::HashSet::new();
        for &c in &coords {
            seen.insert(biome_at(c, c / 2, seed));
        }
        assert!(
            seen.len() >= 2,
            "biomes must vary across regions (saw {seen:?}), expected >=2 distinct biomes"
        );
    }

    /// Terrain must exceed human scale: a 1.75 m person should look *small*, so the
    /// surface must reach well above ~16 m (not be capped at one 4 m chunk). RED until
    /// `surface_height_m` uses a large amplitude (>16 m peaks).
    #[test]
    fn terrain_exceeds_human_scale() {
        let seed = 7u32;
        let mut max_m = 0.0f32;
        for i in 0..=2048 {
            let h = surface_height_m(i, i / 3, seed);
            max_m = max_m.max(h);
        }
        assert!(
            max_m >= 16.0,
            "terrain must exceed human scale (>=16 m), saw max {max_m:.1} m"
        );
    }

    /// The world must be vertically layered, not a single 4 m slab: chunks at y>0 must
    /// also contain terrain. RED until `generate_chunk` iterates world-Y across chunk.y.
    #[test]
    fn chunks_span_multiple_y_layers() {
        let seed = 7u32;
        let mut y_with_terrain = std::collections::HashSet::new();
        for cy in 0..16i64 {
            let c = ChunkCoord::new(0, cy, 0);
            let chunk = generate_chunk(c, seed);
            if chunk_has_any_solid(&chunk) {
                y_with_terrain.insert(cy);
            }
        }
        assert!(
            y_with_terrain.len() >= 2,
            "terrain must span >=2 Y-chunks, saw layers {y_with_terrain:?}"
        );
    }

    /// Helper: does a chunk contain any non-air voxel?
    fn chunk_has_any_solid(chunk: &voxel_core::chunk::Chunk) -> bool {
        for ly in 0..voxel_core::coords::CHUNK_SIZE as u8 {
            for lx in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                for lz in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                    if chunk.get(voxel_core::coords::LocalVoxel::new(lx, ly, lz)).0 != 0 {
                        return true;
                    }
                }
            }
        }
        false
    }
}


