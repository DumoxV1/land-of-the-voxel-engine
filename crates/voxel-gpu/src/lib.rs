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
use voxel_core::coords::ChunkCoord;
use voxel_mesher::Triangle;

/// A finished mesh produced off-thread, tagged with the generation it was requested for.
#[derive(Debug, Clone)]
pub struct MeshResult {
    pub coord: ChunkCoord,
    pub gen: u64,
    pub tris: Vec<Triangle>,
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
        let tris = voxel_mesher::greedy_mesh(&chunk);
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
}
