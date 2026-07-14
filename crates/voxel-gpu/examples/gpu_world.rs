//! S-10 demo: generate a world, mesh it with greedy_mesh, and render it on the GPU (wgpu)
//! to a PNG. This is the engine running on your RTX 4080 — not the software rasterizer.
//!
//! Run with: cargo run --example gpu_world -p voxel-gpu

use voxel_core::coords::ChunkCoord;
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_mesher::greedy_mesh;
use voxel_world::World;

fn main() {
    futures::executor::block_on(async {
        // Build a 2x2 chunk block of generated terrain.
        let mut world = World::new(7);
        let mut tris = Vec::new();
        for cx in 0..2i64 {
            for cz in 0..2i64 {
                let coord = ChunkCoord::new(cx, 0, cz);
                let chunk = world.get_or_generate(coord);
                for t in greedy_mesh(&chunk) {
                    tris.push(t);
                }
            }
        }
        println!("meshed {} triangles across 4 chunks", tris.len());

        // Camera positioned on the 12.5 cm micro-voxel scale (S-13 / ADR-0005):
        // above the ~4 m (32-voxel) terrain, looking down across the 4 m chunks.
        let scene = GpuScene::new_offscreen(512, 512).await.expect("gpu scene");
        let cam = GpuCamera::new([16.0, 50.0, 24.0], -std::f32::consts::FRAC_PI_2, -0.6, 1.0);
        scene
            .render_triangles_png(&tris, &cam, "crates/voxel-gpu/gpu_world.png")
            .await
            .expect("render");
        println!("wrote crates/voxel-gpu/gpu_world.png (GPU-rendered)");
    });
}
