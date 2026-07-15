//! CPU-side LRU mesh cache + view-distance-aware GPU residency policy.
//!
//! The world is deterministic (fBm + biome), so voxel data is cheap to regenerate — the
//! expensive part is meshing. We therefore cache *meshes*, not raw voxels, and evict by
//! "least recently visible" so flying around a 150 km² world never blows RAM and never
//! thrashes the GPU VBO pool. (Inspired by VoxelBee Devlog #5's two-tier cache idea, but
//! applied at mesh level since we use a polygon pipeline, not SVO raycasting.)

use std::collections::HashMap;
use voxel_core::coords::ChunkCoord;

/// A cached mesh for one chunk, with the frame it was last seen visible.
#[derive(Clone)]
pub struct CachedMesh {
    pub tris: Vec<voxel_mesher::Triangle>,
    pub last_seen: u64,
}

/// LRU mesh cache keyed by chunk coord, evicting least-recently-visible entries when the
/// RAM budget is exceeded. `ram_bytes` estimates `tris.len() * BYTES_PER_TRI`.
pub struct LruMeshCache {
    map: HashMap<ChunkCoord, CachedMesh>,
    /// Max resident entries (defensive cap).
    max_entries: usize,
    /// Max estimated RAM in bytes (e.g. 12 GB). 0 = unbounded.
    max_ram_bytes: u64,
    bytes_per_tri: u64,
    /// Incrementally maintained triangle count. `estimated_ram` is O(1) instead of
    /// re-summing the whole map on every eviction pass (was O(N^2) under pressure).
    total_tris: u64,
}

impl LruMeshCache {
    pub fn new(max_entries: usize, max_ram_bytes: u64) -> Self {
        Self {
            map: HashMap::new(),
            max_entries,
            max_ram_bytes,
            bytes_per_tri: 52, // Triangle = 9 f32 + 3 f32 + u32 material = 52 bytes
            total_tris: 0,
        }
    }

    pub fn get(&self, coord: &ChunkCoord) -> Option<&CachedMesh> {
        self.map.get(coord)
    }

    pub fn contains(&self, coord: &ChunkCoord) -> bool {
        self.map.contains_key(coord)
    }

    /// Insert or refresh a mesh; records `frame` as last-seen. Then evicts if over budget.
    pub fn insert(&mut self, coord: ChunkCoord, tris: Vec<voxel_mesher::Triangle>, frame: u64) {
        let n = tris.len() as u64;
        let entry = CachedMesh { tris, last_seen: frame };
        self.map.insert(coord, entry);
        self.total_tris += n;
        self.evict_if_needed(frame);
    }

    /// Mark a chunk as currently visible (refreshes its recency without re-meshing).
    pub fn touch(&mut self, coord: &ChunkCoord, frame: u64) {
        if let Some(e) = self.map.get_mut(coord) {
            e.last_seen = frame;
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// O(1) RAM estimate via the incrementally maintained triangle count.
    fn estimated_ram(&self) -> u64 {
        self.total_tris * self.bytes_per_tri
    }

    /// Drop least-recently-visible entries until within both caps.
    fn evict_if_needed(&mut self, _frame: u64) {
        while self.map.len() > self.max_entries
            || (self.max_ram_bytes > 0 && self.estimated_ram() > self.max_ram_bytes)
        {
            // Find the least-recently-visible coord.
            let victim = self
                .map
                .iter()
                .min_by_key(|(_, e)| e.last_seen)
                .map(|(c, _)| *c);
            match victim {
                Some(c) => {
                    if let Some(e) = self.map.remove(&c) {
                        self.total_tris -= e.tris.len() as u64;
                    }
                }
                None => break,
            }
            // Safety: if we can't shrink further (single entry over ram cap), stop to avoid
            // an infinite loop.
            if self.map.len() <= 1 {
                break;
            }
        }
    }
}

/// GPU residency policy: given candidate chunk coords and the camera eye, choose which chunks
/// may stay resident on the GPU VBO pool, keeping the nearest `max_gpu_chunks` and evicting the
/// rest. Distance is measured center-to-eye in world meters. A simple, deterministic stand-in
/// for VoxelBee's view-LRU with hysteresis (hysteresis added later).
pub fn gpu_resident_set(
    eye: [f32; 3],
    candidates: &[ChunkCoord],
    chunk_m: f32,
    max_gpu_chunks: usize,
) -> Vec<ChunkCoord> {
    let mut scored: Vec<(f32, ChunkCoord)> = candidates
        .iter()
        .map(|c| {
            let cx = (c.x as f32 + 0.5) * chunk_m;
            let cz = (c.z as f32 + 0.5) * chunk_m;
            let d = ((cx - eye[0]).powi(2) + (cz - eye[2]).powi(2)).sqrt();
            (d, *c)
        })
        .collect();
    scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    scored.truncate(max_gpu_chunks);
    scored.into_iter().map(|(_, c)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_core::coords::ChunkCoord;
    use voxel_mesher::{Triangle, Vec3};

    fn tri() -> Triangle {
        // Minimal dummy triangle so the cache has non-empty payloads.
        let v = Vec3::new(0.0, 0.0, 0.0);
        Triangle {
            a: v,
            b: v,
            c: v,
            normal: Vec3::new(0.0, 1.0, 0.0),
            material: voxel_core::palette::MaterialId::from(2),
            ao: [1.0; 3],
        }
    }

    #[test]
    fn lru_mesh_cache_evicts_least_recently_visible() {
        // Realistic flow: each frame visible chunks are touched BEFORE a new chunk is inserted.
        let mut cache = LruMeshCache::new(2, 0); // cap 2 entries
        cache.insert(ChunkCoord::new(0, 0, 0), vec![tri()], 1); // A @1
        cache.insert(ChunkCoord::new(1, 0, 0), vec![tri()], 1); // B @1
        cache.touch(&ChunkCoord::new(0, 0, 0), 2); // A seen again @2
        cache.insert(ChunkCoord::new(2, 0, 0), vec![tri()], 2); // C @2 -> evict oldest (B @1)
        assert!(cache.contains(&ChunkCoord::new(0, 0, 0)), "A must survive (touched @2)");
        assert!(cache.contains(&ChunkCoord::new(2, 0, 0)), "C must survive (@2)");
        assert!(!cache.contains(&ChunkCoord::new(1, 0, 0)), "B (oldest @1) must be evicted");
    }

    #[test]
    fn lru_mesh_cache_ram_budget_evicts() {
        // 1 tri ~ 32 bytes; budget 64 bytes => at most 2 tris => 2 entries of 1 tri each.
        let mut cache = LruMeshCache::new(1000, 64);
        cache.insert(ChunkCoord::new(0, 0, 0), vec![tri()], 1); // A @1
        cache.insert(ChunkCoord::new(1, 0, 0), vec![tri()], 1); // B @1
        cache.touch(&ChunkCoord::new(0, 0, 0), 2); // A @2
        cache.insert(ChunkCoord::new(2, 0, 0), vec![tri()], 5); // C @5 -> RAM 96>64 -> evict B @1
        assert!(cache.len() <= 2, "RAM budget must evict down to <=2 entries, got {}", cache.len());
        assert!(!cache.contains(&ChunkCoord::new(1, 0, 0)), "oldest (B @1) must be evicted under RAM cap");
    }

    #[test]
    fn view_lru_vbo_keeps_near_chunks() {
        let eye = [0.0, 3.0, 0.0];
        let near = ChunkCoord::new(0, 0, 0); // ~0 m
        let mid = ChunkCoord::new(10, 0, 0); // ~40 m
        let far = ChunkCoord::new(50, 0, 0); // ~200 m
        let candidates = vec![far, mid, near];
        let kept = gpu_resident_set(eye, &candidates, 4.0, 2);
        assert!(kept.contains(&near), "nearest chunk must stay on GPU");
        assert!(kept.contains(&mid), "middle chunk must stay on GPU");
        assert!(!kept.contains(&far), "farthest chunk must be evicted from GPU");
    }
}
