//! S-03 software-raster tests (strict TDD — written before implementation, run RED first).
//! References `voxel_render::{Camera, render_scene}` and `image::RgbaImage`, which may not
//! yet exist. Running `cargo test` before implementation yields a compile failure (RED).

use image::RgbaImage;
use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, LocalVoxel};
use voxel_core::palette::MaterialId;
use voxel_render::{Camera, render_scene};

/// Background colour for empty space.
const BG: [u8; 4] = [20, 20, 28, 255];

fn count_non_bg(img: &RgbaImage) -> usize {
    img.pixels().filter(|p| p.0 != BG).count()
}

#[test]
fn empty_chunk_renders_blank() {
    let chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0));
    let cam = Camera::new(35.0, 25.0, 60.0, 50.0);
    let img = render_scene(&chunk, &cam, 64, 64);
    assert_eq!(count_non_bg(&img), 0, "empty chunk must render fully blank");
}

#[test]
fn single_voxel_renders_visible() {
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0));
    chunk.set(LocalVoxel::new(16, 16, 16), MaterialId::from(1));
    let cam = Camera::new(35.0, 25.0, 60.0, 50.0);
    let img = render_scene(&chunk, &cam, 64, 64);
    assert!(
        count_non_bg(&img) > 0,
        "a single solid voxel must produce visible (non-background) pixels"
    );
}

#[test]
fn full_chunk_center_filled() {
    let mut chunk = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0));
    for x in 0..32u8 {
        for y in 0..32u8 {
            for z in 0..32u8 {
                chunk.set(LocalVoxel::new(x, y, z), MaterialId::from(1));
            }
        }
    }
    let cam = Camera::new(35.0, 25.0, 75.0, 50.0);
    let img = render_scene(&chunk, &cam, 64, 64);
    let center = *img.get_pixel(32, 32);
    assert_ne!(
        center.0, BG,
        "a full chunk must project geometry over the image center"
    );
}
