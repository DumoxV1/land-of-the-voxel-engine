//! S-10 probe example: initialize wgpu on the host GPU and render a colored triangle offscreen
//! to a PNG. Proves the GPU path works before building the real voxel renderer.
//!
//! Run with: cargo run --example probe -p voxel-gpu

fn main() {
    // wgpu is async; drive it with a simple single-threaded executor via futures.
    env_logger::init();
    let rt = futures::executor::block_on(async {
        voxel_gpu::probe::render_probe_png("crates/voxel-gpu/probe.png")
            .await
            .map_err(|e| format!("{e:?}"))
    });
    match rt {
        Ok(()) => println!("probe OK -> crates/voxel-gpu/probe.png"),
        Err(e) => {
            eprintln!("probe FAILED: {e}");
            std::process::exit(1);
        }
    }
}
