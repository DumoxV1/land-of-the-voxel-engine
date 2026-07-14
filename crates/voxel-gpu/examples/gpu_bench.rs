//! Fase-2 benchmark-gate: meet de GPU-client FPS op een 1 km² wereld (RTX 4080).
//!
//! Bouwt een `side x side` chunk-wereld (default 32x32 = ~1 km² bij 32 m/chunk),
//! rendert alleen chunks binnen een view-distance (`radius`) rond een bewegende
//! camera (view-distance chunk-streaming, S-12 deel 3 / advies #2), en meet de
//! frametime over `frames` frames. Schrijft p50/p95/p99 + avg FPS naar JSON.
//!
//! Run: cargo run --release --example gpu_bench -p voxel-gpu -- [side] [radius] [frames] [w] [h]
//! Voorbeeld (1 km²): cargo run --release --example gpu_bench -p voxel-gpu -- 32 8 300

use std::collections::HashMap;
use std::time::Instant;

use voxel_core::coords::ChunkCoord;
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_mesher::greedy_mesh;
use voxel_mesher::Triangle;
use voxel_world::World;

const CHUNK: f32 = 32.0; // 1 voxel = 1 m, 1 chunk = 32 m (wereldgen schaal)

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let side = args.get(1).and_then(|s| s.parse::<i64>().ok()).unwrap_or(32);
    let radius = args.get(2).and_then(|s| s.parse::<i64>().ok()).unwrap_or(60);
    let frames = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(300);
    let w = args.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(1024);
    let h = args.get(5).and_then(|s| s.parse::<u32>().ok()).unwrap_or(768);

    println!(
        "bench: side={} ({} chunks ~ {:.0} km²), radius={} chunks, frames={}, target={}x{}",
        side,
        side * side,
        (side * side) as f32 * CHUNK * CHUNK / 1_000_000.0,
        radius,
        frames,
        w,
        h
    );

    futures::executor::block_on(async {
        let mut world = World::new(7);
        let scene = GpuScene::new_offscreen(w, h)
            .await
            .expect("gpu scene init");

        // Mesh-cache per chunk (streaming: mesh elke chunk slechts één keer).
        let mut mesh_cache: HashMap<ChunkCoord, Vec<Triangle>> = HashMap::new();

        // Camera anchor: a fixed spot INSIDE the world bounds (not its center),
        // so the streamer only ever generates chunks within [0, side). The orbit
        // stays within radius of the anchor, clamped to the world edge.
        let anchor_cx = (radius + 2).min(side - radius - 1).max(radius);
        let anchor_cz = anchor_cx;
        let mid = anchor_cx as f32 * CHUNK;
        let mut cam = GpuCamera::new([mid, 50.0, mid], -std::f32::consts::FRAC_PI_2, -0.6, w as f32 / h as f32);
        let eye_y = 50.0; // ~6.25 m, above the ~4 m (32-voxel) terrain on the 12.5 cm scale
        let orbit_r = (radius as f32) * CHUNK * 0.5; // path well inside the world

        let mut frame_times: Vec<f64> = Vec::with_capacity(frames);
        let mut total_visible = 0usize;

        for f in 0..frames {
            // Beweeg de camera in een cirkel (stress voor streaming + frametime).
            let t = f as f32 / 60.0;
            let ex = mid + orbit_r * t.cos();
            let ez = mid + orbit_r * t.sin();
            cam.eye = [ex, eye_y, ez];

            // View-distance-streamer: verzamel zichtbare chunk-coords binnen `radius`.
            let ccx = (ex / CHUNK).floor() as i64;
            let ccz = (ez / CHUNK).floor() as i64;
            let mut coords: Vec<ChunkCoord> = Vec::new();
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    let cx = ccx + dx;
                    let cz = ccz + dz;
                    if cx < 0 || cz < 0 || cx >= side || cz >= side {
                        continue;
                    }
                    coords.push(ChunkCoord::new(cx, 0, cz));
                }
            }
            // Mesh (met cache) en bouw de zichtbare triangle-lijst buiten de cache-borrow.
            let mut visible: Vec<Triangle> = Vec::new();
            for coord in &coords {
                let entry = mesh_cache.entry(*coord).or_insert_with(|| {
                    let chunk = world.get_or_generate(*coord);
                    greedy_mesh(&chunk)
                });
                visible.extend_from_slice(entry);
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

        // Minimale handmatige JSON (geen serde-dep nodig).
        let json = format!(
            "{{\n  \"spike\": \"S-12c-fase2-benchmark-gate\",\n  \"gpu\": \"RTX 4080 Super\",\n  \"side_chunks\": {},\n  \"world_area_m2\": {},\n  \"view_radius_chunks\": {},\n  \"frames\": {},\n  \"render_w\": {},\n  \"render_h\": {},\n  \"avg_visible_triangles\": {},\n  \"p50_ms\": {:.3},\n  \"p95_ms\": {:.3},\n  \"p99_ms\": {:.3},\n  \"avg_ms\": {:.3},\n  \"avg_fps\": {:.2}\n}}\n",
            side,
            (side * side) as f32 * CHUNK * CHUNK,
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
