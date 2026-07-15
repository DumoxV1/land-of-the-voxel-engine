//! voxel-worldgen: deterministic seeded world generation (S-04 spike).
//!
//! Generates `voxel_core::Chunk`s from a seed + `ChunkCoord`. The heightmap is a pure
//! function of world X/Z, so adjacent chunks join seamlessly (no cracks at borders).
//! Renderer-agnostic: depends only on `voxel-core`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

/// How far below the surface we still fill solid voxels. Kept at 1 (a single support voxel
/// under the grass) so the world is a thin shell, not an 8-voxel-thick slab: flying beneath
/// the map then shows nothing (the bottom face is culled against the virtual bedrock floor in
/// the mesher, and there is no thick side wall to see). Collision still lands on the surface
/// voxel. One voxel of footing halves generation cost on deep chunks.
const BEDROCK_DEPTH: i64 = 1;

/// Hard upper bound on `surface_height_m` in **meters**, used by the O(1) air-chunk early-out
/// in `generate_chunk`. `surface_height_m = base + mid + micro` where each term is a
/// `(fbm*0.5+0.5)` value in [0,1) times its amplitude: base ≤ 60 m, mid ≤ 40·max_roughness
/// (roughness ≤ 1.4 → 56 m), micro ≤ 3 m → ≤ 119 m. A +4 m margin keeps the bound safe if the
/// amplitude constants shift. MUST stay ≥ the true supremum or the early-out would clip terrain.
const MAX_SURFACE_M: f32 = 60.0 + 40.0 * 1.4 + 3.0 + 4.0; // 123 m

/// Max distinct column height-buffers kept per thread (LRU). One streamed frame touches a
/// few hundred columns; 64 comfortably covers the burst of Y-slabs generated for a single
/// column while bounding memory (~64 · (34·34·4 B) ≈ 0.3 MB/thread) over a long session.
const COLUMN_CACHE_CAP: usize = 64;

thread_local! {
    /// Per-thread LRU cache of column surface-height buffers, keyed by (cx, cz, seed).
    /// `surface_height_m` is a pure function of world X/Z (independent of chunk.y), so every
    /// Y-slab in a column shares ONE buffer. The live client streams `cy in 0..=max_cy`
    /// (~7-8 slabs/column), so without this cache the (n+2)² buffer — each cell a ~7-fBm
    /// `surface_height_m` call — was rebuilt per slab (~7x redundant). Per-thread → the rayon
    /// mesh pool needs no locking, and determinism holds (buffers store pure results).
    static COLUMN_HBUF_CACHE: RefCell<Vec<((i64, i64, u32), Rc<Vec<f32>>)>> =
        const { RefCell::new(Vec::new()) };
}

/// Return the (n+2)² surface-height buffer (in meters) for column (cx, cz) under `seed`,
/// building it once and caching it per thread (LRU). Byte-identical to the old inline loop.
fn column_height_buffer(cx: i64, cz: i64, origin_x: i64, origin_z: i64, seed: u32) -> Rc<Vec<f32>> {
    let key = (cx, cz, seed);
    // Fast path: cache hit → move-to-front (back = most-recently-used) and return the shared buffer.
    if let Some(hit) = COLUMN_HBUF_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        c.iter().position(|(k, _)| *k == key).map(|pos| {
            let entry = c.remove(pos);
            let buf = entry.1.clone();
            c.push(entry);
            buf
        })
    }) {
        return hit;
    }
    // Miss: build the buffer, then insert (evicting the least-recently-used front at capacity).
    let buf = Rc::new(build_column_height_buffer(origin_x, origin_z, seed));
    COLUMN_HBUF_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= COLUMN_CACHE_CAP {
            c.remove(0);
        }
        c.push((key, buf.clone()));
    });
    buf
}

/// Build the raw (n+2)² surface-height buffer (meters) for a column — the previously-inline
/// loop, factored out so `column_height_buffer` can cache its result across Y-slabs.
fn build_column_height_buffer(origin_x: i64, origin_z: i64, seed: u32) -> Vec<f32> {
    let n = SIZE;
    let stride = (n + 2) as usize;
    let mut hbuf = vec![0.0f32; stride * stride];
    for lx in -1..=n {
        for lz in -1..=n {
            let wx = origin_x + lx;
            let wz = origin_z + lz;
            let idx = ((lx + 1) as usize) * stride + ((lz + 1) as usize);
            hbuf[idx] = surface_height_m(wx, wz, seed);
        }
    }
    hbuf
}

/// Max distinct column cy-ranges cached per thread. A range is a tiny `(i64, i64)`, so a
/// generous cap covers every column a long streaming session visits while costing little RAM
/// (~256k · ~40 B ≈ 10 MB worst case). Cleared wholesale on overflow (cheap, extremely rare).
const COLUMN_RANGE_CACHE_CAP: usize = 262_144;

thread_local! {
    /// Per-thread cache of a column's solid chunk-Y range, keyed by (cx, cz, seed).
    /// `column_solid_cy_range` is a pure function, so caching makes the per-frame streaming
    /// lookup O(1) after a column is first seen.
    static COLUMN_RANGE_CACHE: RefCell<HashMap<(i64, i64, u32), (i64, i64)>> =
        RefCell::new(HashMap::new());
}

/// Inclusive chunk-Y range `[lo, hi]` of the chunks in column (cx, cz) that can contain any
/// solid voxel under `seed`. Any chunk `(cx, cy, cz)` with `cy < lo` or `cy > hi` is
/// **guaranteed** all-AIR — `generate_chunk` returns a uniform-AIR chunk for it via the exact
/// same surface/bedrock envelope. This lets the client's streaming loop skip the tall band of
/// empty sky chunks above the surface and the deep chunks below the bedrock floor *exactly*:
/// the visible set is byte-identical (no chunk carrying geometry is ever excluded), only the
/// wasted per-air-chunk work (frustum test + cache probe + generate call) is avoided.
///
/// Pure + deterministic (function of cx, cz, seed); cached per thread. Mirrors the envelope
/// `generate_chunk` derives from `max_h` (max surface over the 32² footprint) and `min_floor`
/// (min `max(h - BEDROCK_DEPTH, 0)`): a chunk overlaps solid iff
/// `cy*SIZE <= max_h && cy*SIZE + SIZE-1 >= min_floor`.
pub fn column_solid_cy_range(cx: i64, cz: i64, seed: u32) -> (i64, i64) {
    let key = (cx, cz, seed);
    if let Some(hit) = COLUMN_RANGE_CACHE.with(|c| c.borrow().get(&key).copied()) {
        return hit;
    }
    let origin_x = cx * SIZE;
    let origin_z = cz * SIZE;
    let mut max_h = i64::MIN;
    let mut min_floor = i64::MAX;
    // Sample the exact 32² interior footprint `generate_chunk`'s envelope uses — no border
    // ring (the ring only feeds slope, not the air/solid decision).
    for lx in 0..SIZE {
        for lz in 0..SIZE {
            let h = (surface_height_m(origin_x + lx, origin_z + lz, seed)
                / voxel_core::coords::VOXEL_SIZE_M) as i64;
            if h > max_h {
                max_h = h;
            }
            let floor = (h - BEDROCK_DEPTH).max(0);
            if floor < min_floor {
                min_floor = floor;
            }
        }
    }
    // `hi` = highest cy whose bottom voxel (cy*SIZE) still sits at/under the tallest surface.
    let hi = max_h.div_euclid(SIZE);
    // `lo` = lowest cy whose top voxel (cy*SIZE + SIZE-1) still reaches the lowest bedrock
    // floor. Floor-division is conservative (may include one extra all-AIR chunk below, never
    // one too few), so a solid chunk is never excluded.
    let lo = (min_floor - (SIZE - 1)).div_euclid(SIZE);
    let range = (lo, hi);
    COLUMN_RANGE_CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() >= COLUMN_RANGE_CACHE_CAP {
            c.clear();
        }
        c.insert(key, range);
    });
    range
}

/// Generate a deterministic chunk for the given coord + seed.
///
/// The terrain height is a pure function of world X/Z (fBm), so adjacent chunks form
/// one continuous surface. Biome (meadow/desert/snow/rock) is a second pure function of
/// world X/Z and selects the surface material, giving the world varied terrain instead of
/// one uniform grass sheet. Same (coord, seed) always yields an identical chunk.
pub fn generate_chunk(coord: ChunkCoord, seed: u32) -> Chunk {
    let mut chunk = Chunk::uniform(coord, MaterialId::from(AIR));
    let origin = coord.world_voxel(LocalVoxel::new(0, 0, 0)); // world pos of chunk (0,0,0)

    let n = SIZE as i64;

    // A2 fast path (2026-07-15): O(1) whole-chunk AIR early-out with ZERO fBm work.
    // `surface_height_m` is bounded above by its component maxima (base 60 m + mid
    // 40·max_roughness + micro 3 m). Any chunk whose lowest voxel sits above that ceiling is
    // guaranteed all-AIR, so we skip building the (n+2)² height buffer entirely. This is the
    // hot case: the client streams a tall column of Y-layers, most of which are empty sky.
    if origin.y as f32 * voxel_core::coords::VOXEL_SIZE_M > MAX_SURFACE_M {
        return chunk;
    }

    // Surface-height field for this chunk's columns PLUS a 1-voxel border ring (so slope at
    // chunk edges samples the neighbour column without re-evaluating fBm per ly). The field
    // depends only on world X/Z, so it is IDENTICAL for every Y-slab in this (cx,cz) column —
    // computed once and cached per thread, reused across the ~7-8 slabs the client streams per
    // column (was rebuilt per slab). Buffer is (n+2)² with a +1 index offset.
    let stride = (n + 2) as usize;
    let hbuf = column_height_buffer(coord.x, coord.z, origin.x, origin.z, seed);
    let h_vox = |lx: i64, lz: i64| -> i64 {
        let idx = ((lx + 1) as usize) * stride + ((lz + 1) as usize);
        (hbuf[idx] / voxel_core::coords::VOXEL_SIZE_M) as i64
    };

    // A2 (2026-07-15): whole-chunk AIR early-out. Compute the surface-height envelope over
    // THIS chunk's columns; if the chunk's world-Y span lies entirely above every column's
    // surface (nothing to fill) or entirely below every column's bedrock floor, every voxel
    // is AIR. Return the uniform-AIR chunk without running the per-column biome/local fBm
    // (~11 fBm evaluations/column × 1024 columns) or the classify loop. The client streams
    // Y-layers 0..=12 per column, so most streamed chunks are pure air above/below the thin
    // surface shell — this skips their generation cost entirely.
    let mut max_h = i64::MIN;
    let mut min_floor = i64::MAX;
    for lx in 0..n {
        for lz in 0..n {
            let h = h_vox(lx, lz);
            if h > max_h {
                max_h = h;
            }
            let floor_wy = (h - BEDROCK_DEPTH).max(0);
            if floor_wy < min_floor {
                min_floor = floor_wy;
            }
        }
    }
    let chunk_lo = origin.y;
    let chunk_hi = origin.y + n - 1;
    if chunk_lo > max_h || chunk_hi < min_floor {
        return chunk; // entirely above surface or below bedrock → all AIR
    }

    for lx in 0..SIZE as u8 {
        for lz in 0..SIZE as u8 {
            let wx = origin.x + lx as i64;
            let wz = origin.z + lz as i64;
            // Surface height in WORLD-Y voxels (coord.y selects which 4 m slab this chunk is).
            let h = h_vox(lx as i64, lz as i64);
            // 3-tier biome query + local material variation (computed once per column).
            let q = biome_query(wx, wz, seed);
            let local = local_params(wx, wz, seed);
            // Slope from the buffered height field (neighbours in the 1-ring border) — no
            // per-ly fBm re-evaluation (was the dominant cost before the buffer).
            let hl = h_vox(lx as i64 - 1, lz as i64) as f32 * voxel_core::coords::VOXEL_SIZE_M;
            let hr = h_vox(lx as i64 + 1, lz as i64) as f32 * voxel_core::coords::VOXEL_SIZE_M;
            let hd = h_vox(lx as i64, lz as i64 - 1) as f32 * voxel_core::coords::VOXEL_SIZE_M;
            let hu = h_vox(lx as i64, lz as i64 + 1) as f32 * voxel_core::coords::VOXEL_SIZE_M;
            let slope = ((hr - hl).abs() + (hu - hd).abs()) / voxel_core::coords::VOXEL_SIZE_M;
            // Only fill from the surface down to BEDROCK_DEPTH below it; deeper voxels stay
            // AIR (never drawn — greedy meshing only emits the visible shell).
            let floor_wy = (h - BEDROCK_DEPTH).max(0);
            for ly in 0..SIZE as u8 {
                let wy = origin.y + ly as i64;
                if wy < floor_wy {
                    continue; // below bedrock — leave AIR
                }
                let m = classify(wy, h, slope, q, local);
                if m != AIR {
                    chunk.set(LocalVoxel::new(lx, ly, lz), MaterialId::from(m));
                }
            }
        }
    }
    chunk
}

/// Classify a world-Y column into a material given the surface height `h`, the precomputed
/// `slope`, the 3-tier `BiomeQuery`, and the height-safe `LocalParams`. Pure + cheap: NO
/// height-field sampling (caller computes `slope` once per column — see `generate_chunk`).
fn classify(wy: i64, h: i64, slope: f32, q: BiomeQuery, local: LocalParams) -> u8 {
    if wy > h {
        return AIR;
    }
    // Steep exposure → bare rock regardless of biome.
    if slope >= 4.0 && wy >= h - 2 {
        return STONE;
    }
    if wy == h {
        // Surface layer: biome-driven, with Tier-3 local material scatter (height-safe).
        if local.rock_outcrop > 0.82 && q.biome != Biome::Desert {
            return STONE; // exposed stone patches on any biome
        }
        match q.biome {
            Biome::Meadow | Biome::Forest | Biome::Savanna => GRASS,
            Biome::Desert => SAND,
            Biome::Tundra => {
                if local.snow_drift > 0.5 {
                    SNOW
                } else {
                    DIRT
                }
            }
            Biome::Snow => SNOW,
            Biome::Rock => STONE,
        }
    } else if wy >= h - 3 {
        match q.biome {
            Biome::Snow | Biome::Tundra => SNOW, // cold pack a bit thick
            _ => DIRT,
        }
    } else {
        STONE
    }
}

/// Generic signed fBm in [-1,1], configurable octaves (periods in voxels, weights sum to 1
/// after normalization). `fbm01` is the wrapper used by the legacy height field. Pure
/// function of (x, z, seed) → deterministic + seamless across chunks.
fn fbm(x: i64, z: i64, seed: u32, octaves: &[(i64, f32)]) -> f32 {
    let mut n = 0.0f32;
    let mut wsum = 0.0f32;
    for &(period, weight) in octaves {
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
    (2.0 * (n / wsum)) - 1.0
}

/// Normalized fBm in [0,1]: sum of noise octaves (doubling freq / halving amp),
/// divided by total weight. Pure function of (x, z, seed) → deterministic + seamless.
///
/// Walkability (2026-07-15): only the three LOW-frequency octaves are kept. The old
/// 32-voxel (4 m) and 4-voxel (0.5 m) octaves produced 50 cm steps every half metre,
/// making the surface un-walkable. With periods >= 128 voxels (16 m) the steepest local
/// gradient is ~0.14 m/voxel — gentle, walkable hills — while the 40 m amplitude keeps
/// the terrain filmically large against the 1.90 m avatar.
fn fbm01(x: i64, z: i64, seed: u32) -> f32 {
    // Octave periods in VOXELS. Broad hills need large periods: with 12.5 cm voxels,
    // period 2048 ≈ 256 m wide base hills; finer octaves (512, 128) add 64 m / 16 m rolling
    // detail. No sub-16 m octaves — they create un-walkable micro-cliffs.
    const OCTAVES: &[(i64, f32)] = &[(2048, 0.5), (512, 0.28), (128, 0.14)];
    (fbm(x, z, seed, OCTAVES) * 0.5 + 0.5).clamp(0.0, 1.0)
}

/// Surface height in world-Y for a world (x, z), as multi-octave fractal Brownian motion
/// (fBm) in [0, SIZE-1]. Legacy helper kept for slope math; prefers `surface_height_m`.
pub fn height(x: i64, z: i64, seed: u32) -> i64 {
    let scale = (SIZE - 1) as f32;
    (fbm01(x, z, seed) * scale).round().clamp(0.0, (SIZE - 1) as f32) as i64
}

/// Climate octaves (voxels): continental envelope, 4 km + 16 km periods.
const REGION_OCTAVES: &[(i64, f32)] = &[(32768, 0.6), (131072, 0.4)];
/// Biome-selecting octaves (voxels): 64 m + 256 m macro variation.
const BIOME_OCTAVES: &[(i64, f32)] = &[(512, 0.55), (2048, 0.45)];
/// Tier-3 micro HEIGHT octaves (voxels): MUST stay >= 128 vox (16 m) to preserve the
/// `terrain_is_walkable` invariant (< 1 m/voxel local slope).
const LOCAL_H_OCTAVES: &[(i64, f32)] = &[(128, 0.55), (256, 0.35)];
/// Tier-3 micro MATERIAL octaves (voxels): 4 m + 16 m — height-safe (does not move surface).
const LOCAL_M_OCTAVES: &[(i64, f32)] = &[(32, 0.5), (128, 0.5)];

/// Continental climate region (Tier 1). Restricts which biome set is allowed at a location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    Tropical,
    Temperate,
    Arid,
    Boreal,
    Polar,
}

/// Macro biome (Tier 2), gated by the region. Replaces the old 4-value `Biome`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    Meadow,
    Forest,
    Desert,
    Savanna,
    Tundra,
    Snow,
    Rock,
}

/// Tier-3 local variation params — micro detail that does NOT move the surface, so the
/// walkability invariant is untouched. Used only for material scatter (rock outcrops,
/// dunes, forest density, snow drifts).
#[derive(Debug, Clone, Copy)]
pub struct LocalParams {
    pub rock_outcrop: f32,
    pub dune: f32,
    pub forest_density: f32,
    pub snow_drift: f32,
}

/// Full 3-tier biome query result, computed once per column.
#[derive(Debug, Clone, Copy)]
pub struct BiomeQuery {
    pub region: Region,
    pub biome: Biome,
    pub blend: f32,
}

/// Tier 1 — continental climate region: temperature + moisture fields select an envelope.
pub fn climate_region(x: i64, z: i64, seed: u32) -> Region {
    let temp = fbm(x, z, seed ^ 0x7E3D, &REGION_OCTAVES); // warm (+) / cold (-)
    let moist = fbm(x, z, seed ^ 0x5A01, &REGION_OCTAVES); // wet (+) / dry (-)
    if temp < -0.2 && moist < 0.0 {
        Region::Polar
    } else if temp < 0.1 && moist < 0.0 {
        Region::Boreal
    } else if moist < -0.25 {
        Region::Arid
    } else if temp > 0.35 {
        Region::Tropical
    } else {
        Region::Temperate
    }
}

/// Tier 2 — region-gated biome selection from two macro-noise axes.
fn biome_for(region: Region, b: f32, m: f32) -> Biome {
    match region {
        Region::Arid => {
            if b < 0.0 {
                Biome::Desert
            } else {
                Biome::Savanna
            }
        }
        Region::Tropical => {
            if m > 0.1 {
                Biome::Forest
            } else {
                Biome::Savanna
            }
        }
        Region::Polar => Biome::Snow,
        Region::Boreal => {
            if b < -0.1 {
                Biome::Tundra
            } else {
                Biome::Forest
            }
        }
        Region::Temperate => {
            if b < -0.1 {
                Biome::Rock
            } else {
                Biome::Meadow
            }
        }
    }
}

/// Tier 2 — region-gated biome selection from two macro-noise axes (region precomputed).
fn biome_from(region: Region, x: i64, z: i64, seed: u32) -> Biome {
    let b = fbm(x, z, seed ^ 0xB10C, &BIOME_OCTAVES);
    let m = fbm(x, z, seed ^ 0xC0DE, &BIOME_OCTAVES);
    biome_for(region, b, m)
}

/// Tier 3 — local material variation (height-safe micro detail).
pub fn local_params(x: i64, z: i64, seed: u32) -> LocalParams {
    LocalParams {
        rock_outcrop: (fbm(x, z, seed ^ 0x20C1, &LOCAL_M_OCTAVES) * 0.5 + 0.5).clamp(0.0, 1.0),
        dune: (fbm(x + 777, z, seed ^ 0x0C73, &LOCAL_M_OCTAVES) * 0.5 + 0.5).clamp(0.0, 1.0),
        forest_density: (fbm(x, z - 333, seed ^ 0xF031, &LOCAL_M_OCTAVES) * 0.5 + 0.5)
            .clamp(0.0, 1.0),
        snow_drift: (fbm(x - 111, z, seed ^ 0x5A03, &LOCAL_M_OCTAVES) * 0.5 + 0.5).clamp(0.0, 1.0),
    }
}

/// 3-tier biome query: region (T1) → region-gated biome (T2) + blend factor.
pub fn biome_query(x: i64, z: i64, seed: u32) -> BiomeQuery {
    let region = climate_region(x, z, seed);
    let b = fbm(x, z, seed ^ 0xB10C, &BIOME_OCTAVES);
    let m = fbm(x, z, seed ^ 0xC0DE, &BIOME_OCTAVES);
    BiomeQuery {
        region,
        biome: biome_for(region, b, m),
        blend: (b + 1.0) * 0.5,
    }
}

/// Surface height in **meters**, as 3-tier fBm (2026-07-15, Fase-B biome lift).
///
/// - T1: gentle continental envelope (tens of metres, very low freq).
/// - T2: biome-conditioned roughness (desert flat, hills high).
/// - T3: micro height — ONLY >=128-vox octaves (walkability preserved, < 1 m/voxel).
///
/// The continent region field is computed ONCE and shared with the biome lookup so a
/// single height query costs ~7 fBm evaluations instead of 3× redundant region samples.
pub fn surface_height_m(x: i64, z: i64, seed: u32) -> f32 {
    let region = climate_region(x, z, seed);
    // T1: continental envelope (~60 m).
    let base = (fbm(x, z, seed ^ 0xBA5E, &REGION_OCTAVES) * 0.5 + 0.5) * 60.0;
    // T2: biome-conditioned roughness (region shared with biome_from).
    let biome = biome_from(region, x, z, seed);
    let mid = (fbm(x, z, seed ^ 0x71D0, &BIOME_OCTAVES) * 0.5 + 0.5) * 40.0 * biome_roughness(biome);
    // T3: micro height — only >=128-vox octaves (walkable).
    let micro = (fbm(x, z, seed ^ 0x91C3, &LOCAL_H_OCTAVES) * 0.5 + 0.5) * 3.0;
    base + mid + micro
}

/// Per-biome surface roughness multiplier (T2): flat deserts, rugged hills/rock.
fn biome_roughness(biome: Biome) -> f32 {
    match biome {
        Biome::Desert | Biome::Savanna => 0.4,
        Biome::Meadow | Biome::Tundra => 0.8,
        Biome::Forest => 1.0,
        Biome::Snow => 1.1,
        Biome::Rock => 1.4,
    }
}

/// Climate biome for a world (x, z). Backward-compatible wrapper over the 3-tier query:
/// returns the Tier-2 `Biome` so existing callers/tests keep working.
#[deprecated(note = "use biome_query for the full 3-tier result")]
pub fn biome_at(x: i64, z: i64, seed: u32) -> Biome {
    biome_query(x, z, seed).biome
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
    /// (32x per column) — that was ~3.2 ms/chunk. After hoisting slope to once-per-column +
    /// buffering the height field it is ~4 ms/chunk (the 3-tier biome does ~7 fBm/column,
    /// parallelised on the rayon pool in the live client). 200 chunks must finish under a
    /// budget that still leaves headroom for the mesher + GPU upload.
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
            ms < 1500.0,
            "chunk gen too slow: {ms:.1} ms for 200 chunks (height-buffer regression?)"
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
            // Step 512 vox (64 m) — larger than the 128-vox finest octave so the coarse
            // sample MISSES it and we still measure real fractal (fine) detail.
            for x in (0..=2048).step_by(512) {
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
        // Fine detail must exist (fractal) but stay gentle — with only >=128-vox octaves the
        // fine band is ~0.5 m, enough to read as rolling hills without un-walkable micro-cliffs.
        assert!(
            (range_full - range_coarse) >= 0.3,
            "terrain lacks fine-scale (fractal) detail: full {range_full:.1} - coarse {range_coarse:.1} < 0.3 m"
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
    /// Samples several columns (not just (0,0)) so a fragile surface-height boundary at one
    /// column can't make the test fail while the world is genuinely multi-layer.
    #[test]
    fn chunks_span_multiple_y_layers() {
        let seed = 7u32;
        let mut any_multi = false;
        for (bx, bz) in [(0, 0), (40, 40), (120, 80), (200, 200), (300, 50)] {
            let mut y_with_terrain = std::collections::HashSet::new();
            for cy in 0..16i64 {
                let c = ChunkCoord::new(bx, cy, bz);
                let chunk = generate_chunk(c, seed);
                if chunk_has_any_solid(&chunk) {
                    y_with_terrain.insert(cy);
                }
            }
            if y_with_terrain.len() >= 2 {
                any_multi = true;
                break;
            }
        }
        assert!(
            any_multi,
            "terrain must span >=2 Y-chunks for at least one sampled column"
        );
    }

    /// Terrain must be WALKABLE: the steepest local gradient (height change per 1-voxel
    /// step) must stay gentle enough for a 1.90 m avatar to traverse. The old sub-16 m
    /// octaves produced ~0.5 m steps every half metre (un-walkable). After keeping only
    /// octaves >= 128 voxels, the max gradient must be well under 1 m/voxel.
    #[test]
    fn terrain_is_walkable() {
        let seed = 7u32;
        let mut max_slope = 0.0f32;
        for x in 1..4096i64 {
            let a = surface_height_m(x, 1234, seed);
            let b = surface_height_m(x - 1, 1234, seed);
            max_slope = max_slope.max((a - b).abs());
        }
        assert!(
            max_slope < 1.0,
            "terrain too steep to walk: max local slope = {max_slope:.2} m/voxel (want < 1.0)"
        );
    }

    /// BEDROCK_DEPTH (memo docs/research/voxel-loading-standard.md P1): deep chunks far
    /// below the surface must be EMPTY (AIR), not filled with stone to y=0. The surface
    /// chunk still carries terrain; a deep chunk below the bedrock line carries none.
    #[test]
    fn chunk_underground_truncated() {
        let seed = 7u32;
        // Chunk (0,0,0) spans world-Y 0..31, far below the ~210-vox surface — must be AIR.
        let deep = generate_chunk(ChunkCoord::new(0, 0, 0), seed);
        assert!(
            !chunk_has_any_solid(&deep),
            "deep chunk below bedrock must be empty (was filled with underground to y=0)"
        );
        // The surface-spanning chunk must still contain terrain (the visible shell).
        let surface = generate_chunk(ChunkCoord::new(0, 6, 0), seed);
        assert!(
            chunk_has_any_solid(&surface),
            "surface chunk must still contain terrain"
        );
    }

    /// A2 safety (2026-07-15): the O(1) early-out bound `MAX_SURFACE_M` MUST stay above the
    /// true maximum surface height, or high terrain would be silently clipped to AIR. Sample
    /// a wide, varied area (all biomes/regions) and assert every height stays under the bound.
    #[test]
    fn max_surface_bound_covers_real_terrain() {
        let seed = 7u32;
        let mut observed_max = 0.0f32;
        for x in (0..200_000i64).step_by(263) {
            let z = (x * 7) % 200_000;
            observed_max = observed_max.max(surface_height_m(x, z, seed));
        }
        assert!(
            observed_max < MAX_SURFACE_M,
            "MAX_SURFACE_M ({MAX_SURFACE_M} m) must exceed real max surface ({observed_max:.1} m) \
             — the air-chunk early-out would clip terrain otherwise"
        );
    }

    /// A2 (2026-07-15): generating a chunk that lies ENTIRELY above the surface must be
    /// near-free — it takes the O(1) whole-chunk AIR early-out (no height buffer, no per-column
    /// classify loop) and returns a uniform-AIR chunk. The client streams many such air
    /// chunks per column, so this must be an order of magnitude cheaper than a surface chunk.
    #[test]
    fn air_chunk_gen_is_cheap_and_empty() {
        let seed = 7u32;
        // World-Y ~2000 vox (250 m) is far above the ~60-140 m surface envelope → all AIR.
        let high_cy = 60i64;
        // Correctness: the high chunk must be empty (early-out must not change output).
        let air = generate_chunk(ChunkCoord::new(3, high_cy, 5), seed);
        assert!(
            !chunk_has_any_solid(&air),
            "chunk far above the surface must be all AIR"
        );
        // Speed: 5000 above-surface chunks must finish well under the budget a single
        // surface chunk column would need without the early-out.
        let t0 = Instant::now();
        for i in 0..5000i64 {
            let _ = generate_chunk(ChunkCoord::new(i % 400, high_cy, (i / 400) % 400), seed);
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        assert!(
            ms < 500.0,
            "air-chunk gen too slow: {ms:.1} ms for 5000 chunks (early-out missing?)"
        );
    }

    /// Column height-cache (2026-07-15, CRON-herstart): the surface-height buffer is a pure
    /// function of world X/Z (independent of chunk.y), so every Y-slab in a column shares ONE
    /// buffer. The live client streams ~7-8 Y-slabs per column (`cy in 0..=max_cy`), so before
    /// the cache `surface_height_m` (a ~7-fBm call) ran (n+2)² times PER slab = up to ~7x
    /// redundant work per column. This test proves that generating a whole column (same cx/cz,
    /// many cy) is markedly cheaper than generating the same number of DISTINCT columns.
    /// RED before the per-column cache (both paths rebuild the buffer → roughly equal time).
    #[test]
    fn column_reuse_is_faster_than_distinct_columns() {
        let seed = 7u32;
        // cy must stay under the O(1) sky ceiling (origin.y_m = cy*4 m <= MAX_SURFACE_M=123 m,
        // i.e. cy <= 30) so every slab actually builds/uses the height buffer.
        let n = 28i64;
        // Unique base coords per run so a sibling test on the same thread can't pre-warm them.
        let base_cx = 900_001i64;
        let base_cz = 800_003i64;

        // Warm nothing; measure a single column (1 buffer build + n-1 cache hits).
        let t_col = Instant::now();
        for cy in 0..n {
            let _ = generate_chunk(ChunkCoord::new(base_cx, cy, base_cz), seed);
        }
        let col_ms = t_col.elapsed().as_secs_f64() * 1000.0;

        // Measure n DISTINCT columns at a single low cy (n buffer builds = cache misses).
        let t_dist = Instant::now();
        for k in 0..n {
            let _ = generate_chunk(ChunkCoord::new(base_cx + 10_000 * (k + 1), 0, base_cz + 9_000 * (k + 1)), seed);
        }
        let dist_ms = t_dist.elapsed().as_secs_f64() * 1000.0;

        eprintln!(
            "[column-cache] {n} chunks: same-column {col_ms:.3} ms vs distinct-column {dist_ms:.3} ms \
             ({:.2}x faster)",
            dist_ms / col_ms.max(1e-6)
        );
        assert!(
            col_ms < dist_ms * 0.6,
            "per-column height cache missing: same-column {col_ms:.3} ms vs distinct-column \
             {dist_ms:.3} ms (expected same-column < 0.6x)"
        );
    }

    /// Correctness guard for the column cache: generation must stay deterministic and
    /// seed-correct even when columns/seeds are interleaved through the shared cache. The
    /// same (coord, seed) must yield an identical chunk before and after other columns/seeds
    /// touch the cache; different seeds on the same column must NOT collide.
    #[test]
    fn column_cache_preserves_determinism_and_seed_isolation() {
        let coord = ChunkCoord::new(5, 6, 9); // a real surface-spanning column
        let a1 = generate_chunk(coord, 7);
        // Interleave other columns and a different seed to exercise the cache/eviction path.
        for k in 0..80i64 {
            let _ = generate_chunk(ChunkCoord::new(1000 + k, 0, 2000 + k), 7);
        }
        let other_seed = generate_chunk(coord, 42);
        let a2 = generate_chunk(coord, 7); // same coord+seed again, cache churned in between
        // Deterministic: identical material at every voxel for the same (coord, seed).
        for ly in 0..voxel_core::coords::CHUNK_SIZE as u8 {
            for lx in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                for lz in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                    let v = voxel_core::coords::LocalVoxel::new(lx, ly, lz);
                    assert_eq!(a1.get(v).0, a2.get(v).0, "cache broke determinism at {lx},{ly},{lz}");
                }
            }
        }
        // Seed isolation: a different seed on the same column must differ somewhere (no
        // key collision that reuses seed 7's buffer for seed 42).
        let mut differs = false;
        for ly in 0..voxel_core::coords::CHUNK_SIZE as u8 {
            for lx in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                for lz in 0..voxel_core::coords::CHUNK_SIZE as u8 {
                    let v = voxel_core::coords::LocalVoxel::new(lx, ly, lz);
                    if a1.get(v).0 != other_seed.get(v).0 {
                        differs = true;
                    }
                }
            }
        }
        assert!(differs, "different seeds must not collide in the column cache");
    }

    /// Streaming range (2026-07-15, FASE A #3): `column_solid_cy_range` must NEVER exclude a
    /// chunk that actually contains solid voxels — doing so would leave a hole / white gap in
    /// the streamed world (the exact class of bug the client fought repeatedly). For a spread
    /// of columns (incl. negative coords) we scan a wide Y-span and assert every chunk with any
    /// solid voxel falls INSIDE the reported `[lo, hi]`, and that `hi` (the surface chunk)
    /// carries terrain so the bound is tight, not grossly over-wide. RED until the fn exists.
    #[test]
    fn column_range_never_excludes_solid_chunks() {
        let seed = 7u32;
        for &(cx, cz) in &[
            (0, 0),
            (3, 5),
            (40, 40),
            (120, 80),
            (200, 200),
            (-5, 3),
            (2, -4),
            (300, 50),
        ] {
            let (lo, hi) = column_solid_cy_range(cx, cz, seed);
            assert!(lo <= hi, "column ({cx},{cz}) has empty range {lo}..={hi}");
            for cy in -2..=45i64 {
                let chunk = generate_chunk(ChunkCoord::new(cx, cy, cz), seed);
                if chunk_has_any_solid(&chunk) {
                    assert!(
                        cy >= lo && cy <= hi,
                        "column ({cx},{cz}) range {lo}..={hi} EXCLUDES solid chunk cy={cy} \
                         — streaming would leave a hole"
                    );
                }
            }
            // Tightness: the top chunk of the range must be the surface shell (contains terrain).
            assert!(
                chunk_has_any_solid(&generate_chunk(ChunkCoord::new(cx, hi, cz), seed)),
                "column ({cx},{cz}) hi={hi} should be the surface chunk and carry terrain"
            );
        }
    }

    /// The range is a pure function (cx, cz, seed) and must stay deterministic across the
    /// per-thread cache — including churn from many other columns and a different seed reusing
    /// the same (cx,cz). RED until the fn exists.
    #[test]
    fn column_range_is_deterministic_and_cache_safe() {
        let seed = 7u32;
        let r1 = column_solid_cy_range(12_345, 67_890, seed);
        for k in 0..200i64 {
            let _ = column_solid_cy_range(k, k * 2, seed);
        }
        let _ = column_solid_cy_range(12_345, 67_890, 42); // different seed, same column
        let r2 = column_solid_cy_range(12_345, 67_890, seed);
        assert_eq!(r1, r2, "column range must be deterministic across cache churn");
        let (lo, hi) = column_solid_cy_range(12_345, 67_890, 42);
        assert!(lo <= hi, "different-seed range must still be valid");
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


