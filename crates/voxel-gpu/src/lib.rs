//! voxel-gpu: wgpu renderer spike (S-10). Renders voxel meshes on the GPU (Vulkan).
//!
//! - `probe`: offscreen feasibility probe (colored triangle) — proves wgpu works on the host.
//! - `renderer`: real voxel renderer (greedy-mesh triangles -> GPU, Lay of the Land shading).

pub mod probe;
pub mod renderer;

/// Mijlpaal 3 (P3): non-blocking chunk meshing.
///
/// Worldgen + greedy-meshing are pure CPU functions of `(ChunkCoord, seed)`:
/// `voxel_worldgen::generate_chunk` then `voxel_mesher::greedy_mesh`. They never touch the
/// GPU, so they can run on a rayon pool and stream the finished `Vec<Triangle>` back to the
/// render thread through a channel. See `docs/milestone3-rayon-meshing.md`.
use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M};
use voxel_mesher::{Triangle, Vec3};

/// A finished mesh produced off-thread, tagged with the generation it was requested for.
#[derive(Debug, Clone)]
pub struct MeshResult {
    pub coord: ChunkCoord,
    pub gen: u64,
    pub tris: Vec<Triangle>,
}

/// Convert a chunk-local mesh (vertices in voxel units) into canonical GPU world meters.
pub fn mesh_chunk_world_meters(chunk: &Chunk) -> Vec<Triangle> {
    let origin = [
        chunk.coord.x as f32 * CHUNK_SIZE as f32,
        chunk.coord.y as f32 * CHUNK_SIZE as f32,
        chunk.coord.z as f32 * CHUNK_SIZE as f32,
    ];
    let to_world = |p: Vec3| {
        Vec3::new(
            (origin[0] + p.x) * VOXEL_SIZE_M,
            (origin[1] + p.y) * VOXEL_SIZE_M,
            (origin[2] + p.z) * VOXEL_SIZE_M,
        )
    };
    voxel_mesher::greedy_mesh(chunk)
        .into_iter()
        .map(|t| Triangle {
            a: to_world(t.a),
            b: to_world(t.b),
            c: to_world(t.c),
            ..t
        })
        .collect()
}

/// Eye height in renderer coordinates (meters) from a terrain height in voxel units.
#[inline]
pub fn spawn_eye_y_m(top_voxel: i64, eye_clearance_voxels: i64) -> f32 {
    (top_voxel + eye_clearance_voxels) as f32 * VOXEL_SIZE_M
}

/// Build a dedicated rayon pool that keeps one core free for the render thread.
pub fn mesh_pool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().saturating_sub(1).max(1))
        .build()
        .expect("rayon mesh pool")
}

/// Spawn an off-thread generate+mesh for `coord`, sending the result on `tx`.
/// CPU-only: must never touch the wgpu Device/Queue.
pub fn spawn_mesh(
    pool: &rayon::ThreadPool,
    tx: &crossbeam_channel::Sender<MeshResult>,
    coord: ChunkCoord,
    gen: u64,
    seed: u32,
) {
    let tx = tx.clone();
    pool.spawn(move || {
        let chunk = voxel_worldgen::generate_chunk(coord, seed);
        let tris = mesh_chunk_world_meters(&chunk);
        let _ = tx.send(MeshResult { coord, gen, tris });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_core::coords::ChunkCoord;

    #[test]
    fn mesh_chunk_offthread_streams_result() {
        // P3 proof: a chunk is generated+meshed on a rayon pool and arrives via the channel
        // without blocking the calling thread.
        let pool = mesh_pool();
        let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
        spawn_mesh(&pool, &tx, ChunkCoord::new(3, 0, 5), 1, 7);
        let r = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("mesh result should arrive off-thread");
        assert_eq!(r.coord, ChunkCoord::new(3, 0, 5));
        assert_eq!(r.gen, 1);
        assert!(!r.tris.is_empty(), "generated chunk must produce triangles");
    }

    #[test]
    fn streamed_mesh_is_in_chunk_world_meters() {
        // Chunk (2,0,3) spans x=8..12 m and z=12..16 m at 12.5 cm/voxel.
        // The current bug leaves every mesh in local 0..32 voxel coordinates.
        let pool = mesh_pool();
        let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
        let coord = ChunkCoord::new(2, 0, 3);
        spawn_mesh(&pool, &tx, coord, 1, 7);
        let r = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("mesh must arrive");
        let positions = r.tris.iter().flat_map(|t| [t.a, t.b, t.c]);
        for p in positions {
            assert!(
                (8.0..=12.0).contains(&p.x),
                "x={} outside chunk world meters",
                p.x
            );
            assert!(
                (12.0..=16.0).contains(&p.z),
                "z={} outside chunk world meters",
                p.z
            );
            assert!(
                (0.0..=4.0).contains(&p.y),
                "y={} outside chunk world meters",
                p.y
            );
        }
    }

    /// Reproduces the client's "never go white" invariant: a freshly-requested chunk must
    /// eventually land in the cache so the frame has tris to draw. The worker is async, so we
    /// wait briefly for it (mirrors the real frame loop, which retries every frame until the
    /// mesh arrives — the client's sync fallback covers frame 1).
    #[test]
    fn drained_mesh_lands_in_cache_after_one_frame() {
        use std::collections::HashMap;
        use std::time::Duration;
        let pool = mesh_pool();
        let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
        let coord = ChunkCoord::new(2, 0, 2);
        let gen = 1u64;
        spawn_mesh(&pool, &tx, coord, gen, 7);

        let mut cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();
        let mut requested_gen: HashMap<ChunkCoord, u64> = HashMap::new();
        requested_gen.insert(coord, gen);

        // Wait for the worker and drain it into the cache (mirrors: every frame the client
        // retries try_recv until the mesh arrives; here we prove it eventually lands).
        let r = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("mesh must arrive off-thread");
        if requested_gen.get(&r.coord).copied() == Some(r.gen) {
            cache.insert(r.coord, r.tris);
        }
        assert!(
            cache.contains_key(&coord),
            "drained mesh must land in the cache so the frame draws"
        );
        assert!(!cache[&coord].is_empty());
    }
}
