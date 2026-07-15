//! Headless smoke test for the live-streaming first-person path used by `gpu_window`.
//! No winit window: drives the same chunk-streaming + mesh + render logic that the
//! window client uses, with a walking camera, to prove it runs on the 12.5 cm scale
//! (S-13) without panic and yields visible triangles.
//!
//! Kept in sync with the real client (2026-07-15): targets the SURFACE chunk column
//! (surface is tens of metres up since the fBm lift + BEDROCK truncation, so the old
//! hard-coded `cy=0` streamed only empty below-bedrock chunks → 0 tris). Uses
//! `mesh_chunk_world_meters` (canonical world-meter meshes) exactly like `spawn_mesh`,
//! so it also exercises the A1 empty-chunk mesh-skip + A2 gen early-out fast paths.
//!
//! Run: cargo run --release --example client_smoke -p voxel-gpu

use std::collections::HashMap;
use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M};
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_gpu::{mesh_chunk_world_meters, spawn_eye_y_m};
use voxel_mesher::Triangle;
use voxel_worldgen::surface_height_m;

const CHUNK_M: f32 = CHUNK_SIZE as f32 * VOXEL_SIZE_M; // 4 m (ADR-0005)
const VIEW_RADIUS: i64 = 16;
const MAX_CY: i64 = 14; // stream Y-layers 0..=14 per column (like the live client)
const SEED: u32 = 7;

fn main() {
    futures::executor::block_on(async {
        let mut scene = GpuScene::new_offscreen(1024, 768).await.expect("gpu scene");
        let mut cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();

        // First-person spawn: eye a bit above the spawn-column's real surface height.
        let spawn_cx = 1i64;
        let spawn_cz = 1i64;
        let col_wx = spawn_cx * CHUNK_SIZE + CHUNK_SIZE / 2;
        let col_wz = spawn_cz * CHUNK_SIZE + CHUNK_SIZE / 2;
        let top_vox = (surface_height_m(col_wx, col_wz, SEED) / VOXEL_SIZE_M) as i64;
        let eye_y_m = spawn_eye_y_m(top_vox, 12); // ~1.5 m of clearance
        println!(
            "spawn terrain top = {top_vox} voxels (~{:.2} m), eye_y = {:.2} m",
            top_vox as f32 * VOXEL_SIZE_M,
            eye_y_m
        );

        let mut cam = GpuCamera::new(
            [1.5 * CHUNK_M, eye_y_m, 1.5 * CHUNK_M],
            -std::f32::consts::FRAC_PI_2,
            -0.4,
            1024.0 / 768.0,
        );

        let mut total = 0usize;
        let frames = 120;
        for _f in 0..frames {
            // Walk forward (in +X) like a player moving through the world.
            cam.eye[0] += 8.0 * VOXEL_SIZE_M; // ~1 m/frame in world meters
            let [ex, _ey, ez] = cam.eye;
            let ccx = (ex / CHUNK_M).floor() as i64;
            let ccz = (ez / CHUNK_M).floor() as i64;
            let mut tris: Vec<Triangle> = Vec::new();
            for dx in -VIEW_RADIUS..=VIEW_RADIUS {
                for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                    let cx = ccx + dx;
                    let cz = ccz + dz;
                    // Same per-column solid-Y band the live client now uses
                    // (voxel-worldgen::column_solid_cy_range): skips all-AIR sky /
                    // below-bedrock chunks exactly, keeping this smoke test faithful to the
                    // real streaming path. Must still render every frame (no white gaps).
                    let (lo_cy, hi_cy) = voxel_worldgen::column_solid_cy_range(cx, cz, SEED);
                    for cy in lo_cy.max(0)..=hi_cy.min(MAX_CY) {
                        let coord = ChunkCoord::new(cx, cy, cz);
                        let entry = cache.entry(coord).or_insert_with(|| {
                            let c = voxel_worldgen::generate_chunk(coord, SEED);
                            mesh_chunk_world_meters(&c, voxel_gpu::chunk_stream::Lod::Full)
                        });
                        tris.extend_from_slice(entry);
                    }
                }
            }
            if tris.is_empty() {
                continue;
            }
            if scene.render_triangles(&tris, &cam).is_ok() {
                total += 1;
            }
        }
        assert!(
            total > 0,
            "client streaming path rendered 0 frames — surface chunks produced no triangles"
        );
        println!("smoke OK: rendered {total}/{frames} frames (12.5 cm streaming path), no panic");
    });
}
