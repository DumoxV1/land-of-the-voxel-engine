//! voxel-gpu: wgpu renderer spike (S-10). Renders voxel meshes on the GPU (Vulkan).
//!
//! - `probe`: offscreen feasibility probe (colored triangle) — proves wgpu works on the host.
//! - `renderer`: real voxel renderer (greedy-mesh triangles -> GPU, Lay of the Land shading).

pub mod probe;
pub mod renderer;
pub mod cache;
pub mod chunk_stream;
pub mod sunlight;

/// Mijlpaal 3 (P3): non-blocking chunk meshing.
///
/// Worldgen + greedy-meshing are pure CPU functions of `(ChunkCoord, seed)`:
/// `voxel_worldgen::generate_chunk` then `voxel_mesher::greedy_mesh`. They never touch the
/// GPU, so they run on a bounded worker pool (see `gpu_window` / `run_mesh_job`) and stream
/// two-phase messages (`WorkerMsg::Gen` then `WorkerMsg::Mesh`) back to the render thread
/// through a channel. The `Gen` phase ships the raw chunk so player collision can run on
/// freshly streamed terrain before the mesh is ready (A3: collision-first).
use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, LocalVoxel, VOXEL_SIZE_M};
use voxel_mesher::{Triangle, Vec3};

/// Message from a streaming worker back to the render thread. Two phases so collision data
/// arrives **before** the (more expensive) mesh:
/// - `Gen`: phase-1 — the generated raw chunk. The client inserts it into its `World` so
///   player collision can run on freshly streamed terrain immediately, without re-generating
///   the chunk and without waiting for the mesh.
/// - `Mesh`: phase-2 — the greedy-mesh triangles, inserted into the GPU mesh cache for drawing.
#[derive(Debug, Clone)]
pub enum WorkerMsg {
    Gen { coord: ChunkCoord, chunk: Chunk },
    Mesh { coord: ChunkCoord, tris: Vec<Triangle> },
}

/// Build a single flat quad (2 triangles) impersonating a distant chunk: it sits at the
/// column's surface height and is coloured by the chunk's dominant (most common) material.
/// This is the B2 imposter tier — ~2 triangles instead of thousands — and reads as terrain
/// at distance. The normal points up; AO is neutral so it shades like flat ground.
///
/// Height is derived from the chunk itself (highest non-AIR voxel + its slab), so no seed
/// is needed; the quad lands exactly on the chunk's terrain band.
pub fn mesh_chunk_imposter(chunk: &Chunk) -> Vec<Triangle> {
    use voxel_core::coords::{LocalVoxel, CHUNK_SIZE};
    // Dominant non-AIR material in the chunk (modus over all voxels).
    let mut counts: std::collections::HashMap<u8, usize> = std::collections::HashMap::new();
    let mut top_y: i32 = -1; // highest non-AIR local voxel y
    for y in 0..CHUNK_SIZE as u8 {
        for z in 0..CHUNK_SIZE as u8 {
            for x in 0..CHUNK_SIZE as u8 {
                let m = chunk.get(LocalVoxel::new(x, y, z)).0;
                if m != 0 {
                    *counts.entry(m).or_insert(0) += 1;
                    if (y as i32) > top_y {
                        top_y = y as i32;
                    }
                }
            }
        }
    }
    // All-AIR chunk (e.g. sky above the terrain): no surface to stand in for, so emit
    // nothing. Otherwise the far ring would paint a flat quad floating in the sky that
    // pops out when the chunk switches to Full/Half meshing as you fly closer.
    if top_y < 0 {
        return Vec::new();
    }

    let dominant = counts
        .into_iter()
        .max_by_key(|&(_, c)| c)
        .map(|(m, _)| voxel_core::palette::MaterialId::from(m))
        .unwrap_or(voxel_core::palette::MaterialId::from(0u8));

    // Surface height (world meters): chunk slab base + highest solid voxel, in voxel units.
    let y_vox = chunk.coord.y * CHUNK_SIZE + top_y as i64;
    let y = y_vox as f32 * VOXEL_SIZE_M;

    // Chunk footprint in world meters.
    let ox = chunk.coord.x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE_M;
    let oz = chunk.coord.z as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE_M;
    let s = CHUNK_SIZE as f32 * VOXEL_SIZE_M; // chunk edge length in meters
    let (x0, x1) = (ox, ox + s);
    let (z0, z1) = (oz, oz + s);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let mat = dominant;
    let ao = [1.0f32; 3];

    // Two triangles forming the quad (CCW when viewed from above -> upward normal).
    let a = Vec3::new(x0, y, z0);
    let b = Vec3::new(x1, y, z0);
    let c = Vec3::new(x1, y, z1);
    let d = Vec3::new(x0, y, z1);
    let t1 = Triangle { a, b, c, normal: up, material: mat, ao, sun: [1.0; 3] };
    let t2 = Triangle { a, b: c, c: d, normal: up, material: mat, ao, sun: [1.0; 3] };
    vec![t1, t2]
}

/// Convert a chunk-local mesh (vertices in voxel units) into canonical GPU world meters.
/// `lod` downsamples the chunk before meshing: `Lod::Half` collapses every 2×2×2 voxel
/// block into a single 2×-scale voxel (distant chunks need far less geometry), and
/// `Lod::Imposter` collapses the whole chunk into a single flat surface quad (B2).
/// `with_skirts` adds a hanging skirt around the chunk footprint's lower boundary so that
/// seams between different LOD tiers (Full↔Half↔Imposter) do not expose vertical gaps
/// (crack-free LOD, Stap 1, 2026-07-15).
pub fn mesh_chunk_world_meters(
    chunk: &Chunk,
    lod: crate::chunk_stream::Lod,
    with_skirts: bool,
    neighbours: &[Chunk],
    y_max: i64,
) -> Vec<Triangle> {
    // Imposter: cheap flat quad, no greedy sweep at all. Sunlight is irrelevant for the
    // far-ring billboard (it shades like flat ground); leave sun at the default full value
    // the imposter constructor sets.
    if let crate::chunk_stream::Lod::Imposter = lod {
        return mesh_chunk_imposter(chunk);
    }
    // LOD: downsample to 2x blocks first, then mesh the coarse chunk at 2x world scale.
    let (mesh_chunk, voxel_scale) = match lod {
        crate::chunk_stream::Lod::Full => (chunk.clone(), VOXEL_SIZE_M),
        crate::chunk_stream::Lod::Half => {
            let half = downsample_chunk_2x(chunk);
            // Each coarse voxel spans 2 fine voxels = 2 * VOXEL_SIZE_M in world meters.
            (half, VOXEL_SIZE_M * 2.0)
        }
        // Imposter is handled above (early return); this arm keeps the match exhaustive.
        crate::chunk_stream::Lod::Imposter => (chunk.clone(), VOXEL_SIZE_M),
    };
    // World origin in coarse-voxel units: `to_world` multiplies by `voxel_scale`, so to
    // land the chunk at its true world position (coord * CHUNK_SIZE * VOXEL_SIZE_M) the
    // origin must be scaled by VOXEL_SIZE_M / voxel_scale (NOT its inverse — inverting it
    // placed Half chunks at 4x their position, i.e. squares floating in the sky).
    let origin = [
        mesh_chunk.coord.x as f32 * CHUNK_SIZE as f32 * (VOXEL_SIZE_M / voxel_scale),
        mesh_chunk.coord.y as f32 * CHUNK_SIZE as f32 * (VOXEL_SIZE_M / voxel_scale),
        mesh_chunk.coord.z as f32 * CHUNK_SIZE as f32 * (VOXEL_SIZE_M / voxel_scale),
    ];
    let to_world = |p: Vec3| {
        Vec3::new(
            (origin[0] + p.x) * voxel_scale,
            (origin[1] + p.y) * voxel_scale,
            (origin[2] + p.z) * voxel_scale,
        )
    };
    let mut tris: Vec<Triangle> = voxel_mesher::greedy_mesh(&mesh_chunk)
        .into_iter()
        .map(|t| Triangle {
            a: to_world(t.a),
            b: to_world(t.b),
            c: to_world(t.c),
            ..t
        })
        .collect();

    // Stap 3 (BFS zonlicht-lighting): bake sky-light into the mesh so caves/overhangs render
    // dark and open terrain renders bright. MUST run AFTER `to_world` (above) so the triangle
    // positions are in world meters — `bake_sunlight` reads them as world meters and maps back
    // to voxels via the chunk origin. Running it before the offset (on raw 0..32 voxel coords)
    // made every non-origin chunk read the wrong voxel and render as a flat dark rectangle.
    crate::sunlight::bake_sunlight(chunk, &mut tris, neighbours, y_max, voxel_scale, origin);

    if with_skirts {
        // Crack-free LOD seams: hang a skirt from the chunk's *actual surface* downward so
        // any step to a neighbour of different LOD is masked. Two correctness fixes vs a
        // naive box: (a) skip all-air chunks (otherwise empty slabs grow floating skirt
        // boxes in the sky); (b) hang from the real surface top, not the chunk ceiling
        // (hanging from the ceiling produced detached "spikes" above the terrain).
        let mut surface_top_vox = -1i32;
        for y in 0..CHUNK_SIZE as u8 {
            for z in 0..CHUNK_SIZE as u8 {
                for x in 0..CHUNK_SIZE as u8 {
                    if chunk.get(LocalVoxel::new(x, y, z)).0 != 0 {
                        surface_top_vox = surface_top_vox.max(y as i32);
                    }
                }
            }
        }
        if surface_top_vox < 0 {
            return tris; // all-air chunk: nothing to mask, no floating skirt
        }
        let s = CHUNK_SIZE as f32 * voxel_scale; // chunk edge in world meters
        // `origin` is in voxel-units; convert to world meters (same transform as `to_world`).
        let ox = origin[0] * voxel_scale;
        let oz = origin[2] * voxel_scale;
        let base_y = origin[1] * voxel_scale; // chunk slab base in world meters
        let surface_top = base_y + (surface_top_vox as f32 + 1.0) * voxel_scale;
        let skirt_drop = voxel_scale * 4.0; // hang 4 coarse voxels down to mask LOD steps
        let bot_y = (surface_top - skirt_drop).max(base_y);
        let mat = voxel_core::palette::MaterialId::from(2u8);
        let mut push_quad = |x0: f32, x1: f32, z0: f32, z1: f32| {
            // Four side walls of the skirt band, from surface_top down to bot_y.
            let a = Vec3::new(x0, surface_top, z0);
            let b = Vec3::new(x1, surface_top, z0);
            let c = Vec3::new(x1, surface_top, z1);
            let d = Vec3::new(x0, surface_top, z1);
            let e = Vec3::new(x0, bot_y, z0);
            let f = Vec3::new(x1, bot_y, z0);
            let g = Vec3::new(x1, bot_y, z1);
            let h = Vec3::new(x0, bot_y, z1);
            let n = Vec3::new(0.0, -1.0, 0.0);
            let ao = [1.0f32; 3];
            let sun = [1.0f32; 3];
            tris.push(Triangle { a: e, b: f, c: b, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: e, b: b, c: a, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: f, b: g, c: c, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: f, b: c, c: b, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: g, b: h, c: d, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: g, b: d, c: c, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: h, b: e, c: a, normal: n, material: mat, ao, sun });
            tris.push(Triangle { a: h, b: a, c: d, normal: n, material: mat, ao, sun });
        };
        // Skirt along the four chunk edges (full footprint perimeter).
        push_quad(ox, ox + s, oz, oz + s);
    }

    tris
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

/// Run one streaming job to completion on the calling thread, sending both worker messages
/// in order: `Gen` (phase-1: raw chunk for collision) then `Mesh` (phase-2: triangles for
/// drawing). CPU-only: never touches the wgpu Device/Queue. Used directly by the client's
/// bounded worker pool and by tests (synchronous, no thread pool needed).
pub fn run_mesh_job(
    job: crate::chunk_stream::ChunkJob,
    seed: u32,
    tx: &crossbeam_channel::Sender<WorkerMsg>,
) {
    let chunk = voxel_worldgen::generate_chunk(job.coord, seed);
    // Phase 1: collision-first. Ship the raw chunk so the client World (player collision)
    // has it immediately — no re-generate, no wait for the mesh.
    let _ = tx.send(WorkerMsg::Gen {
        coord: job.coord,
        chunk: chunk.clone(),
    });
    // Phase 2: mesh-later. The (more expensive) greedy mesh follows.
    // Stap 3: also generate the 6 direct neighbour chunks (4 horizontal + chunk above/below)
    // so sunlight can flow across chunk boundaries, and a generous y_max covering the full
    // world height (terrain can reach ~119 m; 1024 vox ≈ 128 m is safe headroom).
    let y_max = 1024; // world-voxel ceiling for sunlight seeding (covers all real terrain)
    let neighbours: Vec<Chunk> = [
        (job.coord.x + 1, job.coord.y, job.coord.z),
        (job.coord.x - 1, job.coord.y, job.coord.z),
        (job.coord.x, job.coord.y, job.coord.z + 1),
        (job.coord.x, job.coord.y, job.coord.z - 1),
        (job.coord.x, job.coord.y + 1, job.coord.z),
        (job.coord.x, job.coord.y - 1, job.coord.z),
    ]
    .iter()
    .map(|&(x, y, z)| voxel_worldgen::generate_chunk(ChunkCoord::new(x, y, z), seed))
    .collect();
    let tris = mesh_chunk_world_meters(&chunk, job.lod, false, &neighbours, y_max);
    let _ = tx.send(WorkerMsg::Mesh {
        coord: job.coord,
        tris,
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
        let tris = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, false, &[], 1024);
        assert!(
            !tris.is_empty(),
            "spawn surface chunk ({cx},{cy},{cz}) must produce triangles for frame-1 render"
        );
    }

    #[test]
    fn mesh_chunk_offthread_streams_result() {
        // P3 proof: a chunk is generated+meshed and both phases arrive via the channel
        // without blocking the calling thread.
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerMsg>();
        // Use the chunk that actually contains the terrain surface (BEDROCK truncates deep
        // chunks to AIR, so cy=0 alone would be empty far below the ~26 m surface).
        let cx = 3i64;
        let cz = 5i64;
        let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
            / voxel_core::coords::VOXEL_SIZE_M) as i64
            / 32;
        let coord = ChunkCoord::new(cx, cy, cz);
        run_mesh_job(
            crate::chunk_stream::ChunkJob {
                coord,
                lod: crate::chunk_stream::Lod::Full,
            },
            7,
            &tx,
        );
        // Gen (phase 1) arrives first.
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(WorkerMsg::Gen { coord: c, .. }) => assert_eq!(c, coord),
            other => panic!("expected Gen first, got {other:?}"),
        }
        // Mesh (phase 2) follows.
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(WorkerMsg::Mesh { coord: c, tris }) => {
                assert_eq!(c, coord);
                assert!(!tris.is_empty(), "generated chunk must produce triangles");
            }
            other => panic!("expected Mesh second, got {other:?}"),
        }
    }

    #[test]
    fn worker_sends_gen_before_mesh() {
        // A3 (collision-first) core guarantee: for every streamed job, the raw chunk (Gen)
        // is delivered before its mesh. This is what lets player collision run on freshly
        // streamed terrain immediately, instead of waiting for the (slower) mesh pass.
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerMsg>();
        // Target the surface chunk of a column so the chunk actually carries terrain.
        let cx = 2i64;
        let cz = 3i64;
        let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
            / voxel_core::coords::VOXEL_SIZE_M) as i64
            / 32;
        let coord = ChunkCoord::new(cx, cy, cz);
        run_mesh_job(
            crate::chunk_stream::ChunkJob {
                coord,
                lod: crate::chunk_stream::Lod::Full,
            },
            7,
            &tx,
        );
        let first = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("first message arrives");
        assert!(
            matches!(first, WorkerMsg::Gen { .. }),
            "phase-1 (Gen) must precede phase-2 (Mesh)"
        );
        // The Gen payload must be exactly the generated chunk for this coord (so the client
        // can insert it into its World for collision — even an all-AIR chunk is valid; the
        // point is the worker ships the raw data, not the mesh).
        match first {
            WorkerMsg::Gen { coord: c, chunk } => {
                assert_eq!(c, coord);
                let expected = voxel_worldgen::generate_chunk(coord, 7);
                assert_eq!(
                    chunk.is_empty(),
                    expected.is_empty(),
                    "Gen chunk must match the generated chunk for this coord"
                );
            }
            _ => unreachable!(),
        }
        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(WorkerMsg::Mesh { coord: c, tris }) => {
                assert_eq!(c, coord);
                assert!(!tris.is_empty(), "surface chunk must produce a mesh");
            }
            other => panic!("expected Mesh, got {other:?}"),
        }
    }

    #[test]
    fn streamed_mesh_is_in_chunk_world_meters() {
        // Chunk (2,0,3) spans x=8..12 m and z=12..16 m at 12.5 cm/voxel.
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerMsg>();
        let coord = ChunkCoord::new(2, 0, 3);
        run_mesh_job(
            crate::chunk_stream::ChunkJob {
                coord,
                lod: crate::chunk_stream::Lod::Full,
            },
            7,
            &tx,
        );
        // Drain until the Mesh arrives (skip the Gen).
        let tris = loop {
            match rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .expect("msg arrives")
            {
                WorkerMsg::Mesh { tris, .. } => break tris,
                WorkerMsg::Gen { .. } => continue,
            }
        };
        let positions = tris.iter().flat_map(|t| [t.a, t.b, t.c]);
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
    fn lod_half_shares_full_world_origin_at_double_voxel_size() {
        // A solid single voxel at local (0,0,0): Full meshes it at 0.125 m voxels, Half
        // downsamples 2x and meshes at 0.25 m voxels. Critical invariant: a Half chunk
        // occupies the SAME world footprint as the Full chunk at the same coord — only the
        // voxel granularity differs. (Regression: the origin factor was inverted, placing
        // Half chunks at 4x their world position => squares floating in the sky.)
        use voxel_core::coords::LocalVoxel;
        use voxel_core::palette::MaterialId;
        let coord = ChunkCoord::new(5, 0, 5);
        let mut full_chunk = Chunk::uniform(coord, MaterialId::from(0u8));
        full_chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(2u8));
        let full = mesh_chunk_world_meters(&full_chunk, crate::chunk_stream::Lod::Full, false, &[], 1024);
        let half = mesh_chunk_world_meters(&full_chunk, crate::chunk_stream::Lod::Half, false, &[], 1024);
        assert_eq!(full.len(), 10, "full-res floor voxel = 10 tris (bottom culled on bedrock)");
        assert_eq!(half.len(), 10, "half-res floor block = 10 tris (bottom culled on bedrock)");
        let min_x = |m: &[Triangle]| m.iter().flat_map(|t| [t.a, t.b, t.c]).map(|v| v.x).fold(f32::MAX, f32::min);
        let max_x = |m: &[Triangle]| m.iter().flat_map(|t| [t.a, t.b, t.c]).map(|v| v.x).fold(f32::MIN, f32::max);
        // Both LODs must start at the chunk's true world origin (coord * 32 * 0.125).
        let expected_origin = coord.x as f32 * CHUNK_SIZE as f32 * VOXEL_SIZE_M; // 20.0 m
        assert!((min_x(&full) - expected_origin).abs() < 1e-3, "Full origin {} != {expected_origin}", min_x(&full));
        assert!((min_x(&half) - expected_origin).abs() < 1e-3, "Half must share Full's world origin (got {}, want {expected_origin})", min_x(&half));
        // Half voxels are 2x the size: the single-voxel extent is 0.25 m vs Full's 0.125 m.
        let full_ext = max_x(&full) - min_x(&full);
        let half_ext = max_x(&half) - min_x(&half);
        assert!((full_ext - VOXEL_SIZE_M).abs() < 1e-3, "full voxel extent = 0.125 m (got {full_ext})");
        assert!((half_ext - VOXEL_SIZE_M * 2.0).abs() < 1e-3, "half voxel extent = 0.25 m (got {half_ext})");
    }

    #[test]
    fn imposter_is_single_flat_quad() {
        // B2: an imposter chunk collapses to exactly 2 triangles (one quad), all at the
        // same height (flat ground), coloured by the chunk's dominant material. Cheap
        // stand-in for the far ring vs the full greedy mesh (12+ tris for one voxel).
        use voxel_core::coords::LocalVoxel;
        use voxel_core::palette::MaterialId;
        let coord = ChunkCoord::new(5, 6, 5); // slab 6 -> sits higher than slab 0
        let mut chunk = Chunk::uniform(coord, MaterialId::from(0u8));
        chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(3u8));
        let imp = mesh_chunk_imposter(&chunk);
        assert_eq!(imp.len(), 2, "imposter = single flat quad (2 tris)");
        // All four corners share the same Y (flat), and Y reflects the chunk slab + voxel.
        let y0 = imp[0].a.y;
        for t in &imp {
            assert_eq!(t.a.y, y0);
            assert_eq!(t.b.y, y0);
            assert_eq!(t.c.y, y0);
            assert_eq!(t.material, MaterialId::from(3u8), "imposter uses dominant material");
            assert_eq!(t.normal, Vec3::new(0.0, 1.0, 0.0), "imposter normal points up");
        }
        // Y must equal (slab*32 + 0) * VOXEL_SIZE_M (the single voxel at local y=0).
        let expected_y = (coord.y * 32) as f32 * VOXEL_SIZE_M;
        assert!((y0 - expected_y).abs() < 1e-3, "imposter height = chunk surface (got {y0}, want {expected_y})");
    }

    #[test]
    fn imposter_of_air_chunk_is_empty() {
        // Sky chunks (all AIR, high above the terrain) must NOT emit an imposter quad —
        // otherwise the far ring paints flat squares floating in the sky that pop out
        // when you fly closer and the chunk switches to Full/Half (empty) meshing.
        use voxel_core::palette::MaterialId;
        let coord = ChunkCoord::new(5, 20, 5); // way up in the sky
        let chunk = Chunk::uniform(coord, MaterialId::from(0u8)); // all AIR
        let imp = mesh_chunk_imposter(&chunk);
        assert!(imp.is_empty(), "all-air chunk => no imposter quad (got {} tris)", imp.len());
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
            let tris = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, false, &[], 1024);
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
        let (tx, rx) = crossbeam_channel::unbounded::<WorkerMsg>();
        let coord = {
            let cx = 2i64;
            let cz = 2i64;
            let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
                / voxel_core::coords::VOXEL_SIZE_M) as i64
                / 32;
            ChunkCoord::new(cx, cy, cz)
        };
        run_mesh_job(
            crate::chunk_stream::ChunkJob {
                coord,
                lod: crate::chunk_stream::Lod::Full,
            },
            7,
            &tx,
        );

        let mut cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();

        // Wait for the worker and drain it into the cache (mirrors: every frame the client
        // retries try_recv until the mesh arrives; here we prove it eventually lands).
        // The Gen (phase 1) is the collision chunk; the Mesh (phase 2) is what we draw.
        let mut got_mesh = false;
        while !got_mesh {
            match rx.recv_timeout(Duration::from_secs(10)).expect("msg arrives") {
                WorkerMsg::Mesh { coord: c, tris } => {
                    cache.insert(c, tris);
                    got_mesh = true;
                }
                WorkerMsg::Gen { .. } => continue, // collision data; not drawn
            }
        }
        assert!(
            cache.contains_key(&coord),
            "drained mesh must land in the cache so the frame draws"
        );
        assert!(!cache[&coord].is_empty());
        }

        /// Stap 1 (crack-free skirts): `with_skirts=true` must add a hanging skirt band below the
        /// chunk surface so seams between LOD tiers are masked. We prove the feature exists by
        /// asserting (a) skirts add triangles vs `false`, and (b) the skirt band contains
        /// vertices strictly below the chunk's surface top (it hangs downward, masking steps).
        #[test]
        fn skirt_adds_hanging_band_below_surface() {
        use voxel_core::coords::LocalVoxel;
        use voxel_core::palette::MaterialId;

        // Solid ground block y=0..8 in a single chunk.
        let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
        for y in 0..8u8 {
            for z in 0..32u8 {
                for x in 0..32u8 {
                    chunk.set(LocalVoxel::new(x, y, z), MaterialId::from(2u8));
                }
            }
        }

        let no_skirt = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, false, &[], 1024);
        let with_skirt = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, true, &[], 1024);

        // Skirts must add geometry.
        assert!(
            with_skirt.len() > no_skirt.len(),
            "skirt must add triangles (no_skirt={}, with_skirt={})",
            no_skirt.len(),
            with_skirt.len()
        );

        // The skirt band hangs below the surface top (8 * 0.125 = 1.0 m). Find the surface
        // top from the no-skirt mesh (max Y of any vertex), then assert the skirt mesh has
        // vertices strictly below it (the hanging band).
        let surface_top = no_skirt
            .iter()
            .flat_map(|t| [t.a, t.b, t.c])
            .map(|v| v.y)
            .fold(0.0f32, f32::max);
        let has_below = with_skirt
            .iter()
            .flat_map(|t| [t.a, t.b, t.c])
            .any(|v| v.y < surface_top - VOXEL_SIZE_M * 0.5);
        assert!(
            has_below,
            "skirt band must hang below surface top ({surface_top}); no downward vertices found"
        );
    }

    #[test]
    fn off_origin_chunk_bakes_sun_correctly() {
        // Regression guard (2026-07-15): bake_sunlight MUST run on world-meter triangle
        // positions (after the `to_world` offset), not raw 0..32 voxel coords. On a chunk
        // whose coord != (0,0,0) a wrong offset reads the wrong voxel and paints the whole
        // surface as a flat dark rectangle in-game. Here we prove an off-origin surface
        // chunk still bakes full sky light on its top faces.
        use voxel_core::coords::{CHUNK_SIZE, VOXEL_SIZE_M, LocalVoxel};
        use voxel_core::palette::MaterialId;
        let coord = ChunkCoord::new(2, 0, 3); // off-origin: x=8..12 m, z=12..16 m
        let mut chunk = Chunk::uniform(coord, MaterialId::from(0u8));
        for y in 0..8u8 {
            for z in 0..CHUNK_SIZE as u8 {
                for x in 0..CHUNK_SIZE as u8 {
                    chunk.set(LocalVoxel::new(x, y, z), MaterialId::from(2u8));
                }
            }
        }
        let above = Chunk::uniform(ChunkCoord::new(2, 1, 3), MaterialId::from(0u8));
        let tris = mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, false, &[above], 1024);
        let max_sun = tris
            .iter()
            .filter(|t| t.normal.y > 0.5)
            .flat_map(|t| t.sun.iter().copied())
            .fold(0.0f32, f32::max);
        assert!(
            max_sun > 0.9,
            "off-origin chunk ({},{},{}) top must bake full sky (got {max_sun:.3}), not a dark rectangle",
            coord.x, coord.y, coord.z
        );
    }
}

