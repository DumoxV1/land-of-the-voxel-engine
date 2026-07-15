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
            let h = height(wx, wz, seed); // surface height in world-Y (fBm)
            let biome = biome_at(wx, wz, seed);
            for ly in 0..SIZE as u8 {
                let wy = origin.y + ly as i64;
                let m = classify(wy, h, wx, wz, seed, biome);
                if m != AIR {
                    chunk.set(LocalVoxel::new(lx, ly, lz), MaterialId::from(m));
                }
            }
        }
    }
    chunk
}

/// Classify a world-Y column into a material given the surface height `h`, the world X/Z
/// (for slope), the seed, and the climate `biome`.
///
/// Surface material follows the biome (grass/sand/snow), with bare rock on steep slopes and
/// stone beneath the topsoil.
fn classify(wy: i64, h: i64, wx: i64, wz: i64, seed: u32, biome: Biome) -> u8 {
    if wy > h {
        return AIR;
    }
    // Slope estimate via central differences of the height field (in voxels).
    let slope = {
        let hl = height(wx - 1, wz, seed);
        let hr = height(wx + 1, wz, seed);
        let hd = height(wx, wz - 1, seed);
        let hu = height(wx, wz + 1, seed);
        let dx = (hr - hl).abs();
        let dz = (hu - hd).abs();
        (dx + dz) as f32
    };
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

/// Surface height in world-Y for a world (x, z), as multi-octave fractal Brownian motion
/// (fBm) in [0, SIZE-1].
///
/// Sums several noise octaves at doubling frequency / halving amplitude, giving large hills
/// plus fine detail (fractal relief) instead of one smooth value-noise layer. Pure function
/// of (x, z, seed) → deterministic and seamless across chunk borders.
pub fn height(x: i64, z: i64, seed: u32) -> i64 {
    let scale = (SIZE - 1) as f32;
    // Octaves: (grid period in voxels, amplitude weight). Larger period = broader hills.
    const OCTAVES: &[(i64, f32)] = &[(64, 0.5), (16, 0.28), (4, 0.14), (2, 0.08)];
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
    (n / wsum * scale).round().clamp(0.0, (SIZE - 1) as f32) as i64
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

    /// Terrain must have multi-scale (fractal) relief: large hills AND fine detail, not a
    /// single noise scale. RED until `height` uses fBm: the old 1-layer code has one scale
    /// (N=8), so decimating to every 16th sample loses no range (coarse == full range).
    #[test]
    fn terrain_has_fractal_relief() {
        let seed = 7u32;
        let range_full = {
            let mut mn = i64::MAX;
            let mut mx = i64::MIN;
            for i in 0..=2048 {
                let h = height(i, i / 3, seed);
                mn = mn.min(h);
                mx = mx.max(h);
            }
            mx - mn
        };
        let range_coarse = {
            let mut mn = i64::MAX;
            let mut mx = i64::MIN;
            for i in 0..=128 {
                let h = height(i * 16, (i * 16) / 3, seed);
                mn = mn.min(h);
                mx = mx.max(h);
            }
            mx - mn
        };
        // Large low-frequency hills must exist...
        assert!(
            range_coarse >= 8,
            "terrain lacks large-scale hills: coarse range = {range_coarse}"
        );
        // ...AND fine detail must add range beyond what the coarse sample captures
        // (fBm's high-frequency octaves). On the old single-scale code this delta is ~0.
        assert!(
            (range_full - range_coarse) >= 3,
            "terrain lacks fine-scale (fractal) detail: full range {range_full} - coarse {range_coarse} < 3"
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
}


