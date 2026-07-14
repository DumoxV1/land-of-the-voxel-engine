//! Integration tests for S-02 `voxel-mesher` (strict TDD — written before implementation).
//! These reference the public API that must exist. Running `cargo test` before
//! implementation yields a compile failure (RED). After minimal implementation they go GREEN.

use voxel_core::coords::LocalVoxel;
use voxel_core::palette::MaterialId;
use voxel_mesher::{greedy_mesh, culled_mesh, naive_mesh, Triangle, Vec3};

/// Build a chunk from a closure: true => solid (material 1), false => empty (material 0).
fn build_chunk(solid: impl Fn(i64, i64, i64) -> bool) -> voxel_core::chunk::Chunk {
    use voxel_core::coords::{ChunkCoord, CHUNK_SIZE};
    let n = CHUNK_SIZE as i64;
    let mut chunk = voxel_core::chunk::Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
    for x in 0..n {
        for y in 0..n {
            for z in 0..n {
                if solid(x, y, z) {
                    chunk.set(LocalVoxel::new(x as u8, y as u8, z as u8), MaterialId::from(1u8));
                }
            }
        }
    }
    chunk
}

/// Total mesh surface area (sum of triangle areas). For watertight manifold meshes this
/// is merge-granularity-independent and detects cracks (area too small) / overlaps (too big).
fn surface_area(tris: &[Triangle]) -> f64 {
    let mut area = 0.0f64;
    for t in tris {
        let ab = Vec3::new(t.b.x - t.a.x, t.b.y - t.a.y, t.b.z - t.a.z);
        let ac = Vec3::new(t.c.x - t.a.x, t.c.y - t.a.y, t.c.z - t.a.z);
        let cx = ab.y * ac.z - ab.z * ac.y;
        let cy = ab.z * ac.x - ab.x * ac.z;
        let cz = ab.x * ac.y - ab.y * ac.x;
        area += 0.5 * (cx * cx + cy * cy + cz * cz).sqrt() as f64;
    }
    area
}

fn all_normals_axis_aligned(tris: &[Triangle]) {
    for t in tris {
        let s = t.normal.x.abs() + t.normal.y.abs() + t.normal.z.abs();
        assert!((s - 1.0).abs() < 1e-6, "face normal must be unit axis-aligned");
    }
}

#[test]
fn golden_empty_no_triangles() {
    let chunk = build_chunk(|_, _, _| false);
    for tris in [naive_mesh(&chunk), culled_mesh(&chunk), greedy_mesh(&chunk)] {
        assert_eq!(tris.len(), 0, "empty chunk => 0 triangles");
    }
}

#[test]
fn golden_single_voxel_six_faces() {
    let chunk = build_chunk(|x, y, z| x == 0 && y == 0 && z == 0);
    let tris = [naive_mesh(&chunk), culled_mesh(&chunk), greedy_mesh(&chunk)];
    for t in tris.iter() {
        assert_eq!(t.len(), 12, "single voxel => exactly 12 triangles (6 faces)");
        all_normals_axis_aligned(t);
    }
}

#[test]
fn golden_full_chunk_exposes_outer_shell_only() {
    // Border is air (by design), so a fully-solid chunk exposes exactly its 6 outer faces.
    let n = voxel_core::coords::CHUNK_SIZE as f64;
    let chunk = build_chunk(|_, _, _| true);
    let naive = naive_mesh(&chunk).len();
    let culled = culled_mesh(&chunk);
    let greedy = greedy_mesh(&chunk);

    // naive emits every internal+external face: 6 * N^3 voxels * 2 tris.
    assert_eq!(naive, 6 * (n as usize).pow(3) * 2, "naive emits all faces");
    // culled removes every internal face -> only the shell: 6 faces * N^2 * 2 tris.
    let shell = 6 * (n as usize).pow(2) * 2;
    assert_eq!(culled.len(), shell, "culled keeps only the shell");
    assert!(culled.len() < naive, "culled must remove internal faces");
    // greedy merges coplanar faces; never exceeds 1.5x culled (plan §3.3 / north-star S-02).
    assert!(greedy.len() <= (culled.len() as f64 * 1.5).ceil() as usize,
        "greedy <= 1.5*culled");
    // greedy shell is watertight: area == 6*N^2.
    assert!((surface_area(&greedy) - 6.0 * n * n).abs() < 1e-6, "greedy shell watertight");
    all_normals_axis_aligned(&greedy);
}

#[test]
fn culling_reduces_faces_on_solid_block() {
    // A solid 16^3 block inside the chunk: interior faces get culled.
    let n = 16i64;
    let chunk = build_chunk(|x, y, z| x < n && y < n && z < n);
    let naive = naive_mesh(&chunk).len();
    let culled = culled_mesh(&chunk).len();
    let greedy = greedy_mesh(&chunk);

    let surface = 6 * (n as usize).pow(2) * 2; // 6 faces of the block, 2 tris each
    assert_eq!(culled, surface, "culled = block surface only");
    assert!(culled < naive, "culled must be far less than naive for a solid block");
    assert!(greedy.len() <= (culled as f64 * 1.5).ceil() as usize, "greedy <= 1.5*culled");
    // block is a full n^3 shell (no holes) -> greedy covers 6*n^2 area.
    assert!((surface_area(&greedy) - 6.0 * n as f64 * n as f64).abs() < 1e-6,
        "greedy block watertight");
}

#[test]
fn no_cracks_full_chunk_shell() {
    // A fully-solid chunk (border = air) exposes exactly its 6 outer faces. Greedy must
    // cover exactly 6*N^2 area with no cracks and no overlaps.
    let n = voxel_core::coords::CHUNK_SIZE as i64;
    let chunk = build_chunk(|_, _, _| true);
    let greedy = greedy_mesh(&chunk);
    let expected_area = 6.0 * n as f64 * n as f64;
    assert!((surface_area(&greedy) - expected_area).abs() < 1e-3,
        "greedy full-chunk must cover exactly 6*N^2 (no cracks, no overlaps): got {}",
        surface_area(&greedy));
    all_normals_axis_aligned(&greedy);
    for t in &greedy {
        let cx = (t.a.x + t.b.x + t.c.x) / 3.0;
        let cy = (t.a.y + t.b.y + t.c.y) / 3.0;
        let cz = (t.a.z + t.b.z + t.c.z) / 3.0;
        let center = (n as f32) / 2.0;
        if t.normal.x != 0.0 {
            assert!((t.normal.x > 0.0) == (cx > center), "x-normal must point outward");
        }
        if t.normal.y != 0.0 {
            assert!((t.normal.y > 0.0) == (cy > center), "y-normal must point outward");
        }
        if t.normal.z != 0.0 {
            assert!((t.normal.z > 0.0) == (cz > center), "z-normal must point outward");
        }
    }
}

#[test]
fn hollow_shell_has_twelve_exposed_faces() {
    // A shell of thickness 1: solid only on the outer layer. It exposes 6 outer faces
    // (toward outside air, each N^2) and 6 inner faces (toward the hollow interior, each
    // (N-2)^2 because edge voxels already face other shell voxels). Greedy merges each
    // face into one quad => 12 quads => 24 triangles.
    let n = voxel_core::coords::CHUNK_SIZE as i64;
    let chunk = build_chunk(|x, y, z| {
        x == 0 || y == 0 || z == 0 || x == n - 1 || y == n - 1 || z == n - 1
    });
    let greedy = greedy_mesh(&chunk);
    // 12 exposed faces * 2 triangles each = 24 triangles (greedy merges perfectly).
    assert_eq!(greedy.len(), 24, "hollow shell => 12 faces * 2 tris (greedy merged)");
    all_normals_axis_aligned(&greedy);
    // True surface area: 6*N^2 (outer) + 6*(N-2)^2 (inner).
    let area: f64 = greedy.iter().map(|t| {
        let a = (t.a.x as f64, t.a.y as f64, t.a.z as f64);
        let b = (t.b.x as f64, t.b.y as f64, t.b.z as f64);
        let c = (t.c.x as f64, t.c.y as f64, t.c.z as f64);
        let ab = (b.0 - a.0, b.1 - a.1, b.2 - a.2);
        let ac = (c.0 - a.0, c.1 - a.1, c.2 - a.2);
        let cx = ab.1 * ac.2 - ab.2 * ac.1;
        let cy = ab.2 * ac.0 - ab.0 * ac.2;
        let cz = ab.0 * ac.1 - ab.1 * ac.0;
        0.5 * (cx * cx + cy * cy + cz * cz).sqrt()
    }).sum();
    let expected = 6.0 * n as f64 * n as f64 + 6.0 * (n as f64 - 2.0).powi(2);
    assert!((area - expected).abs() < expected * 0.02,
        "hollow shell area = 6*N^2 + 6*(N-2)^2: got {} expected {}", area, expected);
}
