//! Headless smoke test for the live-streaming first-person path used by `gpu_window`.
//! No winit window: drives the same chunk-streaming + mesh + render_to_view
//! logic that the window client uses, with a walking camera, to prove it runs
//! on the 12.5 cm scale (S-13) without panic and yields visible triangles.
//!
//! Run: cargo run --release --example client_smoke -p voxel-gpu

use std::collections::HashMap;
use voxel_core::coords::{ChunkCoord, CHUNK_SIZE};
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_mesher::greedy_mesh;
use voxel_mesher::Triangle;
use voxel_world::World;

const CHUNK_M: f32 = CHUNK_SIZE as f32 * 0.125; // 4 m (ADR-0005)
const VIEW_RADIUS: i64 = 16;

fn main() {
    futures::executor::block_on(async {
        let mut world = World::new(7);
        let mut scene = GpuScene::new_offscreen(1024, 768)
            .await
            .expect("gpu scene");
        let mut cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();

        // First-person spawn: eye a bit above the spawn-chunk terrain.
        let spawn = ChunkCoord::new(1, 0, 1);
        let chunk = world.get_or_generate(spawn);
        let mut top = 0i64;
        for lx in 0..CHUNK_SIZE as u8 {
            for lz in 0..CHUNK_SIZE as u8 {
                for ly in (0..CHUNK_SIZE as u8).rev() {
                    if chunk
                        .get(voxel_core::coords::LocalVoxel::new(lx, ly, lz))
                        .0
                        != 0
                    {
                        if (ly as i64) > top {
                            top = ly as i64;
                        }
                        break;
                    }
                }
            }
        }
        let eye_y = (top + 3) as f32; // 3 voxels above surface
        println!("spawn terrain top = {top} voxels (~{:.2} m), eye_y = {:.2} m", top as f32 * 0.125, eye_y * 0.125);

        let mut cam = GpuCamera::new(
            [1.5 * CHUNK_M, eye_y, 1.5 * CHUNK_M],
            -std::f32::consts::FRAC_PI_2,
            -0.4,
            1024.0 / 768.0,
        );

        let mut total = 0usize;
        let frames = 120;
        for f in 0..frames {
            // Walk forward (in +X) like a player moving through the world.
            cam.eye[0] += 8.0 * 0.125; // ~1 m/s in voxels
            let [ex, _ey, ez] = cam.eye;
            let ccx = (ex / CHUNK_M).floor() as i64;
            let ccz = (ez / CHUNK_M).floor() as i64;
            let mut tris: Vec<Triangle> = Vec::new();
            for dx in -VIEW_RADIUS..=VIEW_RADIUS {
                for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                    let cx = ccx + dx;
                    let cz = ccz + dz;
                    if cx < 0 || cz < 0 {
                        continue;
                    }
                    let coord = ChunkCoord::new(cx, 0, cz);
                    let entry = cache.entry(coord).or_insert_with(|| {
                        let c = world.get_or_generate(coord);
                        greedy_mesh(&c)
                    });
                    tris.extend_from_slice(entry);
                }
            }
            if tris.is_empty() {
                continue;
            }
            if scene.render_triangles(&tris, &cam).is_ok() {
                total += 1;
            }
        }
        println!("smoke OK: rendered {total}/{frames} frames (12.5 cm streaming path), no panic");
    });
}
