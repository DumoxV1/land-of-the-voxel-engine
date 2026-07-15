//! Throughput micro-bench: realistic surface-chunks (cy chosen like the client does).
//! Run: cargo test -p voxel-gpu throughput_baseline -- --nocapture
#![cfg(test)]
use voxel_core::coords::ChunkCoord;
use voxel_worldgen::generate_chunk;
use voxel_gpu::mesh_chunk_world_meters;
use voxel_worldgen::surface_height_m;

#[test]
fn throughput_baseline() {
    let seed = 7u32;
    let n = 400u32;
    let t0 = std::time::Instant::now();
    let mut total_tris = 0usize;
    for i in 0..n {
        let cx = (i % 20) as i64;
        let cz = (i / 20) as i64;
        // pick the surface chunk like the client's max_cy logic
        let col_top = (surface_height_m(cx*32+16, cz*32+16, seed) / 0.125) as i64;
        let cy = ((col_top + 32) / 32).min(12);
        let c = generate_chunk(ChunkCoord::new(cx, cy, cz), seed);
        let tris = mesh_chunk_world_meters(&c, voxel_gpu::chunk_stream::Lod::Full, true);
        total_tris += tris.len();
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    let per = ms / n as f64;
    println!(
        "THROUGHPUT: {} surface-chunks in {:.1} ms = {:.3} ms/chunk, {:.0} chunks/s, {} tris total ({:.0} tris/chunk)",
        n, ms, per, 1000.0 / per, total_tris, total_tris as f64 / n as f64
    );
    // Regression guard: gen+mesh of a visible (surface) chunk must stay well under a
    // frame budget. 13.7 ms/chunk measured at start (2026-07-15); keep headroom so
    // future optimizations can be proven and regressions caught.
    assert!(per < 25.0, "chunk gen+mesh too slow: {per:.2} ms/chunk (want < 25)");
}

/// A1+A2 win measurement (2026-07-15). Streams a realistic client view-volume — a disk of
/// columns, each streaming Y-layers 0..=14 (like the live client) — and reports gen+mesh
/// throughput plus the air-chunk fraction that now takes the O(1) / mesh-skip fast paths.
/// Ignored by default (timing bench); run explicitly:
///   cargo test -p voxel-gpu stream_volume_bench --release -- --ignored --nocapture
#[test]
#[ignore]
fn stream_volume_bench() {
    let seed = 7u32;
    let radius = 48i64; // ~192 m view radius (matches T9)
    let max_cy = 14i64;
    let mut chunks = 0usize;
    let mut air = 0usize;
    let mut total_tris = 0usize;
    let t0 = std::time::Instant::now();
    for cx in -radius..=radius {
        for cz in -radius..=radius {
            if cx * cx + cz * cz > radius * radius {
                continue; // radial cull (disk)
            }
            for cy in 0..=max_cy {
                let c = generate_chunk(ChunkCoord::new(cx, cy, cz), seed);
                let tris = mesh_chunk_world_meters(&c, voxel_gpu::chunk_stream::Lod::Full, true);
                if tris.is_empty() {
                    air += 1;
                }
                total_tris += tris.len();
                chunks += 1;
            }
        }
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    println!(
        "STREAM_VOLUME: {chunks} chunks ({}% air) in {:.1} ms = {:.4} ms/chunk, {:.0} chunks/s, {} tris",
        air * 100 / chunks.max(1),
        ms,
        ms / chunks as f64,
        chunks as f64 / (ms / 1000.0),
        total_tris,
    );
}
