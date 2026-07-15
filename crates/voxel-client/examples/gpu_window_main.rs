//! Thin entry point for the live micro-voxel client.
//!
//! All client logic lives in the `voxel_client` library (`App`). This binary only builds
//! the event loop and runs it. Run with:
//! `cargo run --release --example gpu_window_main -p voxel-client`

fn main() {
    voxel_client::run();
}
