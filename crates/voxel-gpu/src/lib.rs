//! voxel-gpu: wgpu renderer spike (S-10). Renders voxel meshes on the GPU (Vulkan).
//!
//! - `probe`: offscreen feasibility probe (colored triangle) — proves wgpu works on the host.
//! - `renderer`: real voxel renderer (greedy-mesh triangles -> GPU, Lay of the Land shading).

pub mod probe;
pub mod renderer;
pub mod cache;
pub mod chunk_stream;

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
/// `lod` downsamples the chunk before meshing: `Lod::Half` collapses every 2×2×2 voxel
/// block into a single 2×-scale voxel (distant chunks need far less geometry).
pub fn mesh_chunk_world_meters(chunk: &Chunk, lod: crate::chunk_stream::Lod) -> Vec<Triangle> {
    // A1 (2026-07-15): an all-AIR chunk (every streamed chunk above the surface or below the
    // bedrock line) meshes to nothing. Skip the full greedy sweep (~196k neighbour probes +
    // 6 mask allocations per chunk) — the render loop discards an empty mesh anyway.
    if chunk.is_empty() {
        return Vec::new();
    }
    // LOD: downsample to 2x blocks first, then mesh the coarse chunk at 2x world scale.
    let (mesh_chunk, voxel_scale) = match lod {
        crate::chunk_stream::Lod::Full => (chunk.clone(), VOXEL_SIZE_M),
        crate::chunk_stream::Lod::Half => {
            let half = downsample_chunk_2x(chunk);
            // Each coarse voxel spans 2 fine voxels = 2 * VOXEL_SIZE_M in world meters.
            (half, VOXEL_SIZE_M * 2.0)
        }
    };
    let origin = [
        mesh_chunk.coord.x as f32 * CHUNK_SIZE as f32 * (voxel_scale / VOXEL_SIZE_M),
        mesh_chunk.coord.y as f32 * CHUNK_SIZE as f32 * (voxel_scale / VOXEL_SIZE_M),
        mesh_chunk.coord.z as f32 * CHUNK_SIZE as f32 * (voxel_scale / VOXEL_SIZE_M),
    ];
    let to_world = |p: Vec3| {
        Vec3::new(
            (origin[0] + p.x) * voxel_scale,
            (origin[1] + p.y) * voxel_scale,
            (origin[2] + p.z) * voxel_scale,
        )
    };
    voxel_mesher::greedy_mesh(&mesh_chunk)
        .into_iter()
        .map(|t| Triangle {
            a: to_world(t.a),
            b: to_world(t.b),
            c: to_world(t.c),
            ..t
        })
        .collect()
}

/// Downsample a CHUNK_SIZE³ chunk into a (CHUNK_SIZE/2)³ chunk where each 2×2×2 voxel
/// block becomes one coarse voxel. The coarse voxel keeps the **topmost non-AIR** fine
/// material in the block (the visible surface), or AIR if the whole block is empty. This
/// preserves the silhouette/surface for distant LOD meshes while cutting volume 8×.
fn downsample_chunk_2x(chunk: &Chunk) -> Chunk {
    use voxel_core::coords::{LocalVoxel, CHUNK_SIZE};
    let half = (CHUNK_SIZE / 2) as i32;
    let mut out = Chunk::uniform(chunk.coord, voxel_core::palette::MaterialId::from(0u8));
    for bx in 0..half {
        for by in 0..half {
            for bz in 0..half {
                // Pick the topmost non-AIR voxel in this 2x2x2 block.
                let mut mat = voxel_core::palette::MaterialId::from(0u8);
                'blk: for dy in (0..2).rev() {
                    for dx in 0..2 {
                        for dz in 0..2 {
                            let fx = (bx * 2 + dx) as u8;
                            let fy = (by * 2 + dy) as u8;
                            let fz = (bz * 2 + dz) as u8;
                            let m = chunk.get(LocalVoxel::new(fx, fy, fz));
                            if m != voxel_core::palette::MaterialId::from(0u8) {
                                mat = m;
                                break 'blk;
                            }
                        }
                    }
                }
                if mat != voxel_core::palette::MaterialId::from(0u8) {
                    out.set(LocalVoxel::new(bx as u8, by as u8, bz as u8), mat);
                }
            }
        }
    }
    out
}

/// Eye height in renderer coordinates (meters) from a terrain height in voxel units.
#[inline]
pub fn spawn_eye_y_m(top_voxel: i64, eye_clearance_voxels: i64) -> f32 {
    (top_voxel + eye_clearance_voxels) as f32 * VOXEL_SIZE_M
}

/// Pure first-person free-fly step. `dt` is the frame delta in **seconds** so movement
/// speed is frame-rate independent (the same world distance per second regardless of FPS).
/// `speed` is in world-meters/second. `keys` is a bitmask: bit0=W, bit1=S, bit2=D, bit3=A.
/// Returns the new eye position. Kept pure + public so the live client and unit tests share
/// the exact same integration (no per-frame drift, no "super fast at high FPS" bug).
pub fn free_fly_step(
    eye: [f32; 3],
    yaw: f32,
    pitch: f32,
    dt: f32,
    speed: f32,
    keys: u8,
) -> [f32; 3] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let forward = [cy * cp, sp, sy * cp];
    let right = [cy, 0.0, sy];
    let mut e = eye;
    if keys & 1 != 0 {
        e[0] += forward[0] * speed * dt;
        e[1] += forward[1] * speed * dt;
        e[2] += forward[2] * speed * dt;
    }
    if keys & 2 != 0 {
        e[0] -= forward[0] * speed * dt;
        e[1] -= forward[1] * speed * dt;
        e[2] -= forward[2] * speed * dt;
    }
    if keys & 4 != 0 {
        e[0] += right[0] * speed * dt;
        e[2] += right[2] * speed * dt;
    }
    if keys & 8 != 0 {
        e[0] -= right[0] * speed * dt;
        e[2] -= right[2] * speed * dt;
    }
    e
}

/// Build a dedicated rayon pool that keeps one core free for the render thread.
pub fn mesh_pool() -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus::get().saturating_sub(1).max(1))
        .build()
        .expect("rayon mesh pool")
}

/// Spawn an off-thread generate+mesh for `coord`, sending the result on `tx`.
/// CPU-only: must never touch the wgpu Device/Queue. `lod` downsamples distant chunks.
pub fn spawn_mesh(
    pool: &rayon::ThreadPool,
    tx: &crossbeam_channel::Sender<MeshResult>,
    coord: ChunkCoord,
    gen: u64,
    seed: u32,
    lod: crate::chunk_stream::Lod,
) {
    let tx = tx.clone();
    pool.spawn(move || {
        let chunk = voxel_worldgen::generate_chunk(coord, seed);
        let tris = mesh_chunk_world_meters(&chunk, lod);
        let _ = tx.send(MeshResult { coord, gen, tris });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_core::coords::ChunkCoord;
    use voxel_worldgen;

    #[test]
    fn spawn_surface_chunk_meshes_on_frame_one() {
        // White-screen guard (2026-07-15): the very first frame must be able to show
        // terrain. The client targets the surface chunk directly under the spawn column
        // as its frame-1 fallback, so that chunk MUST produce triangles synchronously.
        // Regression: an old placeholder camera eye ([40,50,40]) streamed the wrong
        // column and the fallback selected nothing -> clear-color flash (white screen).
        use voxel_core::coords::{CHUNK_SIZE, VOXEL_SIZE_M};
        let seed = 7u32;
        let cx = 1i64; // spawn column (player on chunk (1,0,1) center)
        let cz = 1i64;
        let col_wx = (cx * CHUNK_SIZE + CHUNK_SIZE / 2) as i64;
        let col_wz = (cz * CHUNK_SIZE + CHUNK_SIZE / 2) as i64;
        let col_top_vox = (voxel_worldgen::surface_height_m(col_wx, col_wz, seed) / VOXEL_SIZE_M) as i64;
        let cy = (col_top_vox / CHUNK_SIZE as i64).clamp(0, 12);
        let coord = ChunkCoord::new(cx, cy, cz);
        let chunk = voxel_worldgen::generate_chunk(coord, seed);
        let tris = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full);
        assert!(
            !tris.is_empty(),
            "spawn surface chunk ({cx},{cy},{cz}) must produce triangles for frame-1 render"
        );
    }

    #[test]
    fn mesh_chunk_offthread_streams_result() {
        // P3 proof: a chunk is generated+meshed on a rayon pool and arrives via the channel
        // without blocking the calling thread.
        let pool = mesh_pool();
        let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
        // Use the chunk that actually contains the terrain surface (BEDROCK truncates deep
        // chunks to AIR, so cy=0 alone would be empty far below the ~26 m surface).
        let cx = 3i64;
        let cz = 5i64;
        let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
            / voxel_core::coords::VOXEL_SIZE_M) as i64
            / 32;
        let coord = ChunkCoord::new(cx, cy, cz);
        spawn_mesh(&pool, &tx, coord, 1, 7, crate::chunk_stream::Lod::Full);
        let r = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("mesh result should arrive off-thread");
        assert_eq!(r.coord, coord);
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
        spawn_mesh(&pool, &tx, coord, 1, 7, crate::chunk_stream::Lod::Full);
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

    #[test]
    fn lod_half_downsamples_to_double_scale() {
        // A solid single voxel at local (0,0,0): Full meshes 1 voxel (12 tris) at 0.125 m,
        // Half meshes 1 coarse voxel (12 tris) but at 0.25 m (2x) world scale. Both must
        // produce a non-empty mesh; the Half mesh's world extent must be ~2x the Full.
        use voxel_core::coords::LocalVoxel;
        use voxel_core::palette::MaterialId;
        let coord = ChunkCoord::new(5, 0, 5);
        let mut full_chunk = Chunk::uniform(coord, MaterialId::from(0u8));
        full_chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(2u8));
        let full = mesh_chunk_world_meters(&full_chunk, crate::chunk_stream::Lod::Full);
        let half = mesh_chunk_world_meters(&full_chunk, crate::chunk_stream::Lod::Half);
        assert_eq!(full.len(), 12, "full-res single voxel = 12 tris");
        assert_eq!(half.len(), 12, "half-res single block = 12 tris (one coarse voxel)");
        // Half mesh lives at 2x world scale -> its max vertex coordinate is ~2x the Full's.
        let full_max = full.iter().flat_map(|t| [t.a, t.b, t.c]).map(|v| v.x).fold(0.0f32, f32::max);
        let half_max = half.iter().flat_map(|t| [t.a, t.b, t.c]).map(|v| v.x).fold(0.0f32, f32::max);
        assert!(
            half_max > full_max * 1.5,
            "half-res mesh must be ~2x larger in world meters (full_max={full_max}, half_max={half_max})"
        );
    }

    /// Movement must be frame-rate independent: the same key held for the same wall-clock
    /// time must travel the same world distance regardless of how many frames elapse. This
    /// catches the "super fast at high FPS" bug where speed was added per-frame (no dt).
    #[test]
    fn free_fly_speed_is_frame_rate_independent() {
        let eye0 = [0.0, 3.88, 0.0];
        let yaw = -std::f32::consts::FRAC_PI_2; // look down -Z
        let pitch = -0.4;
        let speed = 8.0; // m/s
        let len = |a: [f32; 3], b: [f32; 3]| {
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        };
        // 1 second of W held, integrated in 1 big step vs 600 small steps (600 FPS).
        let one_big = free_fly_step(eye0, yaw, pitch, 1.0, speed, 1);
        let mut e = eye0;
        for _ in 0..600 {
            e = free_fly_step(e, yaw, pitch, 1.0 / 600.0, speed, 1);
        }
        let d_big = len(one_big, eye0);
        let d_small = len(e, eye0);
        let rel = (d_big - d_small).abs() / d_big.max(1e-6);
        assert!(
            rel < 1e-3,
            "frame-rate dependent movement: 1-step={d_big:.4} 600-step={d_small:.4} (rel={rel:.4})"
        );
        // Absolute distance over 1 s equals speed (8 m/s) * |forward| (=1) = 8 m,
        // independent of pitch (forward is a unit vector). Not per-frame*600.
        assert!(
            (d_big - speed).abs() < 1e-2,
            "W for 1 s at 8 m/s should move ~{speed} m, moved {d_big:.4} m"
        );
    }

    /// Negative chunk coordinates must yield real terrain, not be skipped. The client used to
    /// `continue` on `cx < 0 || cz < 0`, which made flying into negative space produce zero
    /// triangles → white screen. This proves negative chunks generate + mesh normally.
    #[test]
    fn negative_chunk_coords_yield_nonempty_mesh() {
        for &(cx, cz) in &[(-1, -1), (-5, 3), (2, -4)] {
            // The chunk that contains the terrain surface for this (cx,cz) — BEDROCK
            // truncates deep chunks to AIR, so we must target the surface chunk, not cy=0.
            let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
                / voxel_core::coords::VOXEL_SIZE_M) as i64
                / 32;
            let coord = ChunkCoord::new(cx, cy, cz);
            let chunk = voxel_worldgen::generate_chunk(coord, 7);
            let tris = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full);
            assert!(
                !tris.is_empty(),
                "negative chunk {cx},{cz} must produce terrain, not be skipped"
            );
        }
    }
    /// eventually land in the cache so the frame has tris to draw. The worker is async, so we
    /// wait briefly for it (mirrors the real frame loop, which retries every frame until the
    /// mesh arrives — the client's sync fallback covers frame 1).
    #[test]
    fn drained_mesh_lands_in_cache_after_one_frame() {
        use std::collections::HashMap;
        use std::time::Duration;
        let pool = mesh_pool();
        let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
        let coord = {
            let cx = 2i64;
            let cz = 2i64;
            let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
                / voxel_core::coords::VOXEL_SIZE_M) as i64
                / 32;
            ChunkCoord::new(cx, cy, cz)
        };
        let gen = 1u64;
        spawn_mesh(&pool, &tx, coord, gen, 7, crate::chunk_stream::Lod::Full);

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
