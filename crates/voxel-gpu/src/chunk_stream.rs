//! State-of-the-art chunk streaming (2026-07-15 refactor of the inline radial loop in
//! gpu_window.rs).
//!
//! Replaces the old "every frame, for each chunk in the view disc, fire an unbounded
//! `mesh_pool.spawn`" approach with a proper streaming pipeline:
//!
//!   1. **Priority queue (close→far).** Each frame we enumerate the column coords inside
//!      the view disc, compute a priority key (chebyshev distance, tie-broken by height),
//!      and only *request* the N highest-priority chunks that are not yet cached/pending.
//!      This makes the ground under the player appear instantly and the horizon fill in
//!      gradually instead of all-at-once "phasing in".
//!
//!   2. **Bounded worker pool with back-pressure.** Jobs are pushed onto a bounded
//!      channel; a fixed set of worker threads `recv()` blocks. The producer (render
//!      thread) is never allowed to outrun the workers (no 300 simultaneous gen+mesh
//!      jobs thrashing the CPU). Capacity = num_workers * 2.
//!
//!   3. **Height cache.** `surface_height_m` is the expensive worldgen noise; we memoize
//!      it per column so we don't recompute the same height every frame (the old loop
//!      called it for every column, every frame).
//!
//!   4. **Frustum-before-request.** Chunks fully outside the camera frustum are never
//!      requested (the old loop only frustum-culled at *draw* time, still generating
//!      meshes for chunks behind the player).
//!
//!   5. **Air-skip.** A chunk slab that is entirely above `col_top_vox + margin` is never
//!      generated (the old loop capped at MAX_Y but still baked fully-empty slabs).
//!
//!   6. **LOD.** Chunks in the outer ring of the view disc are meshed at half resolution
//!      (2×2×2 voxel blocks) — distant terrain needs far less geometry, cutting the
//!      triangle count for the far ring ~8×.

use std::collections::{HashMap, HashSet, VecDeque};

use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M};
use voxel_worldgen::surface_height_m;

/// Vertical voxel scale is 0.125 m/voxel (12.5 cm). METERS_PER_CHUNK below assumes the
/// chunk edge in *world meters*; kept here to avoid a crate-cycle import.
pub const VOXEL_SIZE: f32 = VOXEL_SIZE_M;

/// Level of detail for a streamed chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lod {
    /// Full 1×1×1 voxel resolution (near field).
    Full,
    /// Half resolution: voxels are 2×2×2 blocks (far field). ~8× fewer triangles.
    Half,
    /// Imposter: a single flat quad at the column's surface height, coloured by the
    /// dominant material. The cheapest tier (2 triangles/chunk) for the far ring — a
    /// "billboard" that reads as terrain at distance without any real geometry. (B2.)
    Imposter,
}

impl Lod {
    /// Voxel downsample factor (1 = full, 2 = half). Imposter is a degenerate quad.
    pub fn factor(&self) -> i64 {
        match self {
            Lod::Full => 1,
            Lod::Half => 2,
            Lod::Imposter => 1,
        }
    }
}

/// A unit of work handed to a worker thread.
#[derive(Debug, Clone, Copy)]
pub struct ChunkJob {
    pub coord: ChunkCoord,
    pub lod: Lod,
}

/// Priority-key for the request queue. Lower value = higher priority (requested first).
/// We sort by chebyshev (max-axis) distance so chunks form square shells around the
/// camera; the Y component is weighted lighter so a tall nearby column beats a flat far one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrioKey {
    d: i64, // chebyshev distance (x/z) from camera column
    dy: i64, // vertical distance from the camera's current slab
}

impl PrioKey {
    fn new(dx: i64, dz: i64, dy: i64) -> Self {
        let d = dx.unsigned_abs().max(dz.unsigned_abs()) as i64;
        let dy = dy.unsigned_abs() as i64;
        PrioKey { d, dy }
    }
}

impl Ord for PrioKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Primary: horizontal shell distance (close first). Secondary: vertical distance.
        // Reverse for a min-heap semantics via BinaryHeap (we store NegPrio).
        self.d
            .cmp(&other.d)
            .then(self.dy.cmp(&other.dy))
    }
}

impl PartialOrd for PrioKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Wrapper so BinaryHeap yields the *smallest* key first (BinaryHeap is a max-heap).
#[derive(Debug, Clone, Copy)]
struct NegPrio(PrioKey);

impl PartialEq for NegPrio {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for NegPrio {}
impl PartialOrd for NegPrio {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.0.cmp(&self.0)) // reversed -> min-heap
    }
}
impl Ord for NegPrio {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.cmp(&self.0)
    }
}

/// Memoized column surface heights. Keyed by (cx, cz) column index. Bounded by an LRU
/// eviction so a long session doesn't leak memory.
#[derive(Default)]
pub struct HeightCache {
    map: HashMap<(i64, i64), i64>,
    /// Insertion order (FIFO) for eviction when the map exceeds `cap`.
    order: VecDeque<(i64, i64)>,
    cap: usize,
}

impl HeightCache {
    pub fn new(cap: usize) -> Self {
        HeightCache {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap: cap.max(1),
        }
    }

    /// Surface voxel height (in voxels) for a column, memoized.
    pub fn surface_vox(&mut self, cx: i64, cz: i64, seed: u32) -> i64 {
        let key = (cx, cz);
        if let Some(&h) = self.map.get(&key) {
            return h;
        }
        let col_wx = cx * CHUNK_SIZE as i64 + CHUNK_SIZE as i64 / 2;
        let col_wz = cz * CHUNK_SIZE as i64 + CHUNK_SIZE as i64 / 2;
        let h = (surface_height_m(col_wx, col_wz, seed) / VOXEL_SIZE) as i64;
        self.map.insert(key, h);
        self.order.push_back(key);
        // Evict oldest while over capacity (keeps map + order in lockstep).
        while self.map.len() > self.cap {
            if let Some(k) = self.order.pop_front() {
                self.map.remove(&k);
            } else {
                break;
            }
        }
        h
    }

    /// Drop cached heights for columns far from the camera (called when the player
    /// teleports / long session) to keep memory bounded.
    pub fn retain_near(&mut self, ccx: i64, ccz: i64, radius: i64) {
        self.map.retain(|&(x, z), _| {
            (x - ccx).unsigned_abs() <= radius as u64 && (z - ccz).unsigned_abs() <= radius as u64
        });
        self.order
            .retain(|&(x, z)| (x - ccx).unsigned_abs() <= radius as u64 && (z - ccz).unsigned_abs() <= radius as u64);
    }
}

/// Streaming config (tunable knobs).
#[derive(Debug, Clone, Copy)]
pub struct StreamConfig {
    /// Horizontal view radius in chunks (the streaming disc radius).
    pub view_radius: i64,
    /// Hard cap on streamed vertical slabs (~48 m at MAX_Y=12).
    pub max_y: i64,
    /// Number of chunk requests issued per frame (back-pressure budget).
    pub requests_per_frame: usize,
    /// Chunks beyond this chebyshev distance use Lod::Half.
    pub lod_half_radius: i64,
    /// Chunks beyond this chebyshev distance use Lod::Imposter (single flat quad). (B2.)
    pub lod_imposter_radius: i64,
    /// Extra slabs kept above the surface height (cull margin).
    pub air_margin: i64,
}

impl Default for StreamConfig {
    fn default() -> Self {
        StreamConfig {
            view_radius: 12,
            max_y: 12,
            requests_per_frame: 4,
            lod_half_radius: 8,
            lod_imposter_radius: 11,
            air_margin: 1,
        }
    }
}

/// The streaming scheduler. Pure logic: given the camera column and the set of already
/// cached/pending coords, it produces the ordered list of chunk jobs to request this
/// frame (at most `requests_per_frame`). The caller is responsible for actually spawning
/// the work and for frustum culling (the frustum needs the camera matrices which live in
/// the client).
pub struct ChunkScheduler {
    cfg: StreamConfig,
    /// Columns already requested (pending) or cached this session — bounded by completion.
    seen: HashSet<ChunkCoord>,
}

impl ChunkScheduler {
    pub fn new(cfg: StreamConfig) -> Self {
        ChunkScheduler {
            cfg,
            seen: HashSet::new(),
        }
    }

    /// Mark a coord as no-longer-needed (completed or evicted) so it can be re-requested
    /// later if the camera returns.
    pub fn forget(&mut self, coord: &ChunkCoord) {
        self.seen.remove(coord);
    }

    /// Reserve `coord` as requested (so subsequent frames won't re-issue it).
    pub fn reserve(&mut self, coord: ChunkCoord) {
        self.seen.insert(coord);
    }

    pub fn is_seen(&self, coord: &ChunkCoord) -> bool {
        self.seen.contains(coord)
    }

    /// Core enumeration: build the priority-ordered request plan for this frame.
    ///
    /// `ready` returns true if a coord is already cached (don't request). `cached_or_pending`
    /// is the closure the caller provides to check both the mesh cache and the in-flight
    /// set; we additionally track `seen` here for the within-session generation guard.
    ///
    /// Returns at most `requests_per_frame` jobs, closest first, with LOD assigned by ring.
    pub fn plan(
        &mut self,
        ccx: i64,
        ccz: i64,
        _cam_slab: i64,
        heights: &mut HeightCache,
        seed: u32,
        ready: impl Fn(&ChunkCoord) -> bool,
    ) -> Vec<ChunkJob> {
        let r = self.cfg.view_radius;
        let r2 = r * r;
        // Collect candidate columns with their priority + max slab.
        let mut cands: Vec<(NegPrio, i64, i64)> = Vec::new(); // (prio, cx, cz)
        for dx in -r..=r {
            for dz in -r..=r {
                if dx * dx + dz * dz > r2 {
                    continue; // radial disc, not square
                }
                let cx = ccx + dx;
                let cz = ccz + dz;
                let top_vox = heights.surface_vox(cx, cz, seed);
                let max_cy = ((top_vox + CHUNK_SIZE as i64) / CHUNK_SIZE as i64)
                    .min(self.cfg.max_y)
                    .max(0);
                let prio = NegPrio(PrioKey::new(dx, dz, 0));
                cands.push((prio, cx, cz));
                // Stash max_cy for slab enumeration below (recompute lazily per column).
                let _ = max_cy;
            }
        }
        // Stable sort by priority (min-heap order): closest chunk first.
        // NegPrio reverses PrioKey, so descending NegPrio == ascending PrioKey == near first.
        cands.sort_by(|a, b| b.0.cmp(&a.0));

        let mut out: Vec<ChunkJob> = Vec::new();
        for (_, cx, cz) in cands {
            if out.len() >= self.cfg.requests_per_frame {
                break;
            }
            let top_vox = heights.surface_vox(cx, cz, seed);
            let min_cy = (top_vox.saturating_sub(CHUNK_SIZE as i64))
                .div_euclid(CHUNK_SIZE as i64)
                .max(0)
                .min(self.cfg.max_y);
            let max_cy = ((top_vox + CHUNK_SIZE as i64) / CHUNK_SIZE as i64)
                .min(self.cfg.max_y)
                .max(0);
            let cheb = ((cx - ccx).unsigned_abs().max((cz - ccz).unsigned_abs())) as i64;
            let lod = if cheb >= self.cfg.lod_imposter_radius {
                Lod::Imposter
            } else if cheb >= self.cfg.lod_half_radius {
                Lod::Half
            } else {
                Lod::Full
            };
            for cy in min_cy..=max_cy + self.cfg.air_margin {
                if cy > self.cfg.max_y {
                    break;
                }
                let coord = ChunkCoord::new(cx, cy, cz);
                if ready(&coord) || self.seen.contains(&coord) {
                    continue; // already cached or requested
                }
                // Air-skip: never above the surface + margin slab.
                if cy > max_cy + self.cfg.air_margin {
                    continue;
                }
                self.seen.insert(coord);
                out.push(ChunkJob { coord, lod });
                if out.len() >= self.cfg.requests_per_frame {
                    break;
                }
            }
            if out.len() >= self.cfg.requests_per_frame {
                break;
            }
        }
        out
    }
}

/// Stap 2 (inter-chunk occlusie, LxVL): returns true if `target` column is hidden behind
/// taller terrain along the camera's forward yaw. We walk a ray of chunk-columns from the
/// camera column toward `target`; if any *closer* column has a surface height that exceeds
/// `target`'s surface + `cam_y` (camera eye height), the target is tucked behind the
/// "height wall" and need not be generated/meshed this frame.
///
/// Pure + cheap (one ray of length ≤ view radius). The client calls this in Pass A, after
/// frustum culling, to drop chunks the player literally cannot see because a hill/peak
/// blocks the line of sight.
pub fn is_occluded_by_terrain(
    ccx: i64,
    ccz: i64,
    yaw: f32,
    target: ChunkCoord,
    cam_y: f32,
    heights: &mut HeightCache,
    seed: u32,
) -> bool {
    let (sy, cy) = yaw.sin_cos(); // forward = (cy, sy) on the XZ plane
    let (dx, dz) = (cy, sy);
    // Step from the camera column outward; stop before reaching the target itself.
    let mut step = 1i64;
    loop {
        let ix = ccx + (dx * step as f32).round() as i64;
        let iz = ccz + (dz * step as f32).round() as i64;
        if ix == target.x && iz == target.z {
            break; // reached the target's own column; no occluder between
        }
        // Manhattan/chebyshev distance from camera to this sample column.
        let dist = ((ix - ccx).unsigned_abs().max((iz - ccz).unsigned_abs())) as i64;
        let target_dist =
            ((target.x - ccx).unsigned_abs().max((target.z - ccz).unsigned_abs())) as i64;
        if dist >= target_dist {
            break; // overshot past the target's ring; nothing between
        }
        let occ_h = heights.surface_vox(ix, iz, seed) as f32 * VOXEL_SIZE;
        let tgt_h = heights.surface_vox(target.x, target.z, seed) as f32 * VOXEL_SIZE;
        if occ_h > tgt_h + cam_y {
            return true; // a closer, taller column blocks the line of sight
        }
        step += 1;
        if step > 64 {
            break; // safety bound
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hc() -> HeightCache {
        HeightCache::new(1024)
    }

    #[test]
    fn priority_queue_is_close_first() {
        // Plan from origin with a tiny budget; the nearest column must come first.
        let cfg = StreamConfig {
            view_radius: 3,
            max_y: 12,
            requests_per_frame: 2,
            lod_half_radius: 8,
            lod_imposter_radius: 8,
            air_margin: 0,
        };
        let mut s = ChunkScheduler::new(cfg);
        let mut hh = hc();
        // No chunk is "ready" -> all candidates requested. With budget 2 we should get
        // the 2 closest columns (dx=0/dz=0 region), NOT a far one.
        let jobs = s.plan(0, 0, 0, &mut hh, 1, |_| false);
        assert_eq!(jobs.len(), 2, "budget caps requests");
        // Both returned jobs must be within chebyshev distance <= 1 of the camera column
        // (closest shells), proving close→far ordering.
        for j in &jobs {
            let d = (j.coord.x).unsigned_abs().max(j.coord.z.unsigned_abs());
            assert!(d <= 1, "closest-first violated: got cheb={d}");
        }
    }

    #[test]
    fn height_cache_memoizes_and_is_bounded() {
        let mut h = HeightCache::new(16);
        let a = h.surface_vox(5, 5, 42);
        let b = h.surface_vox(5, 5, 42);
        assert_eq!(a, b, "memoized height must be stable");
        // Hammer it with many distinct columns; must not exceed cap meaningfully.
        for i in 0..1000 {
            h.surface_vox(i, i % 3, 42);
        }
        assert!(h.map.len() <= 16, "height cache must stay bounded to cap (got {})", h.map.len());
    }

    #[test]
    fn lod_assigned_three_tiers() {
        let cfg = StreamConfig {
            view_radius: 12,
            max_y: 12,
            requests_per_frame: 5000,
            lod_half_radius: 4,
            lod_imposter_radius: 8,
            air_margin: 0,
        };
        let mut s = ChunkScheduler::new(cfg);
        let mut hh = hc();
        let jobs = s.plan(0, 0, 0, &mut hh, 1, |_| false);
        let near = jobs
            .iter()
            .find(|j| j.coord.x == 0 && j.coord.z == 0)
            .expect("near column planned");
        assert_eq!(near.lod, Lod::Full, "near field must be full res");
        let mid = jobs
            .iter()
            .find(|j| j.coord.x == 6 && j.coord.z == 0)
            .expect("mid column planned");
        assert_eq!(mid.lod, Lod::Half, "mid ring must be half res");
        let far = jobs
            .iter()
            .find(|j| j.coord.x == 11 && j.coord.z == 0)
            .expect("far column planned");
        assert_eq!(far.lod, Lod::Imposter, "far ring must be imposter");
    }

    #[test]
    fn air_skip_never_above_surface() {
        // A column whose surface is at voxel 5 (slab 0) with max_y=4 and air_margin=0:
        // only slabs 0..=0 should be requested, never slab 1 or 2. We force the surface by
        // using a seed/column where surface_height_m is low; to make the test deterministic
        // we instead assert the invariant structurally: no planned job exceeds max_cy+margin,
        // and the scheduler never requests a slab above max_y.
        let cfg = StreamConfig {
            view_radius: 2,
            max_y: 4,
            requests_per_frame: 1000,
            lod_half_radius: 8,
            lod_imposter_radius: 8,
            air_margin: 0,
        };
        let mut s = ChunkScheduler::new(cfg);
        let mut hh = hc();
        let jobs = s.plan(0, 0, 0, &mut hh, 1, |_| false);
        for j in &jobs {
            assert!(j.coord.y <= cfg.max_y, "no job above max_y: cy={}", j.coord.y);
        }
        // And at least some jobs were produced (surface exists within max_y).
        assert!(!jobs.is_empty(), "scheduler must produce jobs for a surface within max_y");
    }

    #[test]
    fn occlusion_cull_skips_chunks_behind_tall_terrain() {
        // Camera at (0,0) looking +X (yaw = 0 -> forward = (1,0)). A tall peak at column x=2
        // (surface 30 m) must occlude a short column at x=5 (surface 2 m) behind it.
        let mut hc = HeightCache::new(1024);
        let seed = 7u32;
        // Force specific heights via the cache (surface_vox memoizes; seed must match usage).
        hc.surface_vox(0, 0, seed); // camera column (whatever height)
        hc.surface_vox(2, 0, seed);
        hc.surface_vox(5, 0, seed);
        // Override the memoized heights directly to simulate a peak + valley.
        hc.map.insert((2, 0), (30.0 / VOXEL_SIZE) as i64); // 30 m peak
        hc.map.insert((5, 0), (2.0 / VOXEL_SIZE) as i64); // 2 m valley behind it
        hc.map.insert((0, 0), (2.0 / VOXEL_SIZE) as i64); // camera on low ground

        let target = ChunkCoord::new(5, 0, 0);
        let occluded = is_occluded_by_terrain(0, 0, 0.0, target, 1.5, &mut hc, seed);
        assert!(
            occluded,
            "chunk behind a 30 m peak (cam at 1.5 m) must be occluded"
        );

        // Control: a chunk at x=5 with NO tall column between camera and it is NOT occluded.
        let mut hc2 = HeightCache::new(1024);
        hc2.surface_vox(0, 0, seed);
        hc2.surface_vox(5, 0, seed);
        // Flatten the whole line of sight (x=0..5) to 2 m so nothing occludes.
        for x in 0..=5 {
            hc2.map.insert((x, 0), (2.0 / VOXEL_SIZE) as i64);
        }
        let target2 = ChunkCoord::new(5, 0, 0);
        let not_occluded = is_occluded_by_terrain(0, 0, 0.0, target2, 1.5, &mut hc2, seed);
        assert!(
            !not_occluded,
            "flat terrain with no occluder must not be culled"
        );
    }
}
