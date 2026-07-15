//! Benchmark-gate op de productieschaal: 12,5 cm/voxel, 4 m/chunk (ADR-0005).
//!
//! Bouwt een `side x side` chunk-wereld, rendert alleen chunks binnen een view-distance
//! (`radius`) rond een bewegende camera, en meet frametime over `frames` frames.
//! Gebruikt dezelfde frustum-culling + wereldmeter-mesh als de live client, zodat de
//! benchmark de werkelijke renderkosten meet (niet "alle triangles ingeslikt").
//!
//! Schaal-feit: 1 chunk = 32^3 voxels * 0,125 m = 4 m. Een 250x250 grid = 62.500 chunks
//! = exact 1 km². De oude bench gebruikte 32 m/chunk en noemde 32x32 ten onrechte 1 km².
//!
//! Run: cargo run --release --example gpu_bench -p voxel-gpu -- [side] [radius] [frames] [w] [h]
//! Voorbeeld (1 km²): cargo run --release --example gpu_bench -p voxel-gpu -- 250 24 300

use std::collections::HashMap;
use std::time::Instant;

use voxel_core::coords::ChunkCoord;
use voxel_gpu::renderer::{Frustum, GpuCamera, GpuScene};
use voxel_gpu::mesh_chunk_world_meters;
use voxel_mesher::Triangle;
use voxel_world::World;

const CHUNK_M: f32 = 4.0; // 12.5 cm/voxel * 32 voxels = 4 m (ADR-0005)

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let side = args.get(1).and_then(|s| s.parse::<i64>().ok()).unwrap_or(250);
    let radius = args.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(24);
    let frames = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(300);
    let w = args.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1024);
    let h = args.get(5).and_then(|s| s.parse::<u32>().ok()).unwrap_or(768);

    // Echte oppervlakte: side*CHUNK_M meter zijde -> m². 250*4 = 1000 m = 1 km².
    let world_edge_m = side as f32 * CHUNK_M;
    let world_area_m2 = world_edge_m * world_edge_m;

    println!(
        "bench: side={} ({} chunks, edge={:.0} m, area={:.0} m² = {:.4} km²), radius={} chunks, frames={}, target={}x{}",
        side,
        side * side,
        world_edge_m,
        world_area_m2,
        world_area_m2 / 1_000_000.0,
        radius,
        frames,
        w,
        h
    );

    futures::executor::block_on(async {
        let mut world = World::new(7);
        let mut scene = GpuScene::new_offscreen(w, h)
            .await
            .expect("gpu scene init");

        // Mesh-cache per chunk (streaming: mesh elke chunk slechts één keer).
        let mut mesh_cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();

        // Camera anchor binnen wereldgrenzen; orbit binnen radius.
        let anchor_cx = if side > 2 * radius + 2 {
            (radius + 2).min(side - radius - 1).max(radius)
        } else {
            // Kleine grids: centreer binnen de wereld en beperk orbit tot de helft.
            (side / 2).max(1).min(side - 1)
        };
        let mid = anchor_cx as f32 * CHUNK_M;
        // Spawn-hoogte zoals de client: terrain-top (28 voxels) + 3 clearance, * 0.125.
        let eye_y = (28 + 3) as f32 * 0.125;
        let mut cam = GpuCamera::new(
            [mid, eye_y, mid],
            -std::f32::consts::FRAC_PI_2,
            -0.6,
            w as f32 / h as f32,
        );
        // Orbit binnen de wereld houden: bij kleine grids de straal beperken.
        let orbit_r = ((radius as f32) * CHUNK_M * 0.5).min(mid.max(1.0) - CHUNK_M);

        let mut frame_times: Vec<f64> = Vec::with_capacity(frames);
        let mut total_visible = 0usize;

        for f in 0..frames {
            let t = f as f32 / 60.0;
            let ex = mid + orbit_r * t.cos();
            let ez = mid + orbit_r * t.sin();
            cam.eye = [ex, eye_y, ez];
            let vp = cam.view_proj();
            let frustum = Frustum::from_view_proj(&vp);

            let ccx = (ex / CHUNK_M).floor() as i64;
            let ccz = (ez / CHUNK_M).floor() as i64;
            let half = CHUNK_M * 0.5;
            let half_y = CHUNK_M * 1.5;

            let mut visible: Vec<Triangle> = Vec::new();
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    let cx = ccx + dx;
                    let cz = ccz + dz;
                    if cx < 0 || cz < 0 || cx >= side || cz >= side {
                        continue;
                    }
                    let coord = ChunkCoord::new(cx, 0, cz);
                    // Frustum-culling, identiek aan de client.
                    if !frustum.intersects_aabb(
                        [
                            (cx as f32 + 0.5) * CHUNK_M,
                            half_y,
                            (cz as f32 + 0.5) * CHUNK_M,
                        ],
                        half,
                    ) {
                        continue;
                    }
                    let entry = mesh_cache.entry(coord).or_insert_with(|| {
                        let chunk = world.get_or_generate(coord);
                        mesh_chunk_world_meters(&chunk)
                    });
                    visible.extend_from_slice(entry);
                }
            }
            total_visible += visible.len();

            let t0 = Instant::now();
            let ok = scene.render_triangles(&visible, &cam).is_ok();
            let dt = t0.elapsed().as_secs_f64() * 1000.0;
            if ok {
                frame_times.push(dt);
            }
        }

        frame_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = frame_times.len();
        let pct = |p: f64| -> f64 {
            if n == 0 {
                return 0.0;
            }
            let idx = ((p / 100.0) * (n as f64 - 1.0)).round() as usize;
            frame_times[idx.min(n - 1)]
        };
        let p50 = pct(50.0);
        let p95 = pct(95.0);
        let p99 = pct(99.0);
        let avg_ms = if n > 0 {
            frame_times.iter().sum::<f64>() / n as f64
        } else {
            0.0
        };
        let avg_fps = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };
        let avg_visible = if frames > 0 {
            total_visible / frames
        } else {
            0
        };

        println!(
            "RESULT frames={} p50={:.2}ms p95={:.2}ms p99={:.2}ms avg={:.2}ms avg_fps={:.1} avg_visible_tris={}",
            n, p50, p95, p99, avg_ms, avg_fps, avg_visible
        );

        let json = format!(
            "{{
  \"spike\": \"S-12c-fase2-benchmark-gate\",
  \"gpu\": \"RTX 4080 Super\",
  \"scale\": \"12.5cm-voxel-4m-chunk\",
  \"side_chunks\": {},
  \"world_edge_m\": {},
  \"world_area_m2\": {},
  \"world_area_km2\": {:.4},
  \"view_radius_chunks\": {},
  \"frames\": {},
  \"render_w\": {},
  \"render_h\": {},
  \"avg_visible_triangles\": {},
  \"p50_ms\": {:.3},
  \"p95_ms\": {:.3},
  \"p99_ms\": {:.3},
  \"avg_ms\": {:.3},
  \"avg_fps\": {:.2}
}}
",
            side,
            world_edge_m,
            world_area_m2,
            world_area_m2 / 1_000_000.0,
            radius,
            n,
            w,
            h,
            avg_visible,
            p50,
            p95,
            p99,
            avg_ms,
            avg_fps
        );
        std::fs::write("crates/voxel-gpu/bench_1km2.json", &json).expect("write json");
        println!("wrote crates/voxel-gpu/bench_1km2.json");
    });
}
