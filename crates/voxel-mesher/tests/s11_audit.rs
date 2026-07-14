//! S-11 audit-hardening tests (strict TDD — RED before fixes).
//!
//! Audit finding M-01: positive faces must lie on the +side plane (d+1), and triangle
//! winding must be counter-clockwise seen from the outside (consistent with the normal),
//! so backface culling can be enabled.

use voxel_core::coords::{ChunkCoord, LocalVoxel};
use voxel_core::palette::MaterialId;
use voxel_mesher::{culled_mesh, greedy_mesh, naive_mesh, Triangle, Vec3};

fn single_voxel_chunk() -> voxel_core::chunk::Chunk {
    let mut chunk =
        voxel_core::chunk::Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
    chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(1u8));
    chunk
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// The unit cube voxel at (0,0,0) fills [0,1)^3. Its +X face lies on the plane x=1,
/// its -X face on x=0, etc. Golden test on actual vertex coordinates (the audit found
/// all six faces collapsed onto the min-corner planes).
#[test]
fn golden_single_voxel_face_planes() {
    let chunk = single_voxel_chunk();
    for (name, tris) in [
        ("naive", naive_mesh(&chunk)),
        ("culled", culled_mesh(&chunk)),
        ("greedy", greedy_mesh(&chunk)),
    ] {
        for t in &tris {
            let n = t.normal;
            // Which axis and which side?
            let (axis_val, verts): (f32, [f32; 3]) = if n.x.abs() > 0.5 {
                (if n.x > 0.0 { 1.0 } else { 0.0 }, [t.a.x, t.b.x, t.c.x])
            } else if n.y.abs() > 0.5 {
                (if n.y > 0.0 { 1.0 } else { 0.0 }, [t.a.y, t.b.y, t.c.y])
            } else {
                (if n.z > 0.0 { 1.0 } else { 0.0 }, [t.a.z, t.b.z, t.c.z])
            };
            for v in verts {
                assert!(
                    (v - axis_val).abs() < 1e-6,
                    "{name}: face with normal {:?} must lie on plane {axis_val}, found vertex coord {v}",
                    (n.x, n.y, n.z)
                );
            }
        }
    }
}

/// Geometric winding (cross(b-a, c-a)) must point the same way as the stored normal,
/// so `cull_mode: Back` renders every face correctly.
#[test]
fn winding_matches_normal() {
    let chunk = single_voxel_chunk();
    for (name, tris) in [
        ("naive", naive_mesh(&chunk)),
        ("culled", culled_mesh(&chunk)),
        ("greedy", greedy_mesh(&chunk)),
    ] {
        for t in &tris {
            let g = cross(sub(t.b, t.a), sub(t.c, t.a));
            let d = dot(g, t.normal);
            assert!(
                d > 1e-6,
                "{name}: triangle winding must be CCW seen from outside (dot(geom_normal, normal) = {d}, normal {:?})",
                (t.normal.x, t.normal.y, t.normal.z)
            );
        }
    }
}

/// Two stacked voxels: the shared face is interior. The top of the stack must lie on y=2.
#[test]
fn golden_two_stacked_voxels_top_plane() {
    let mut chunk =
        voxel_core::chunk::Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
    chunk.set(LocalVoxel::new(0, 0, 0), MaterialId::from(1u8));
    chunk.set(LocalVoxel::new(0, 1, 0), MaterialId::from(1u8));
    let tris = greedy_mesh(&chunk);
    let up: Vec<&Triangle> = tris.iter().filter(|t| t.normal.y > 0.5).collect();
    assert!(!up.is_empty(), "stack must have a +Y cap");
    for t in up {
        for y in [t.a.y, t.b.y, t.c.y] {
            assert!(
                (y - 2.0).abs() < 1e-6,
                "+Y cap of a 2-high stack must lie on y=2, found {y}"
            );
        }
    }
}
