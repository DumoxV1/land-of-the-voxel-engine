//! Integration tests for S-02 `voxel-mesher` (strict TDD — written before implementation).
//! These reference the public API that must exist. Running `cargo test` before
//! implementation yields a compile failure (RED). After minimal implementation they go GREEN.

use voxel_core::coords::LocalVoxel;
use voxel_core::palette::MaterialId;
use voxel_mesher::{greedy_mesh, culled_mesh, naive_mesh, Triangle, Vec3};

/// Build a chunk from a closure: true => solid (material 1), false => empty (material 0).
fn build_chunk(solid: impl Fn(i64, i64, i64) -> bool) -> voxel_core::chunk::Chunk {
    use voxel_core::coords::{ChunkCoord, CHUNK_SIZE};
    let n = CHUNK_SIZE;
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
    let naive = naive_mesh(&chunk);
    let culled = culled_mesh(&chunk);
    let greedy = greedy_mesh(&chunk);
    // naive emits all 6 faces (no culling) -> 12 tris.
    assert_eq!(naive.len(), 12, "naive: single voxel => 6 faces (12 tris)");
    // culled + greedy drop the bottom face against the virtual bedrock floor -> 5 faces (10 tris).
    assert_eq!(culled.len(), 10, "culled: lone voxel on floor => 5 exposed faces (10 tris)");
    assert_eq!(greedy.len(), 10, "greedy: lone voxel on floor => 5 exposed faces (10 tris)");
    all_normals_axis_aligned(&greedy);
}

#[test]
fn golden_full_chunk_exposes_outer_shell_only() {
    // Bedrock floor below y<0 means the chunk's bottom face (y==0) is culled
    // against virtual solid, so a fully-solid chunk exposes 5 outer faces, not 6.
    let n = voxel_core::coords::CHUNK_SIZE as f64;
    let chunk = build_chunk(|_, _, _| true);
    let naive = naive_mesh(&chunk).len();
    let culled = culled_mesh(&chunk);
    let greedy = greedy_mesh(&chunk);

    // naive emits every internal+external face: 6 * N^3 voxels * 2 tris.
    assert_eq!(naive, 6 * (n as usize).pow(3) * 2, "naive emits all faces");
    // culled removes every internal face + the bottom (bedrock) -> 5 faces * N^2 * 2 tris.
    let shell = 5 * (n as usize).pow(2) * 2;
    assert_eq!(culled.len(), shell, "culled keeps the 5-face shell (bottom on bedrock)");
    assert!(culled.len() < naive, "culled must remove internal faces");
    // greedy merges coplanar faces; never exceeds 1.5x culled (plan §3.3 / north-star S-02).
    assert!(greedy.len() <= (culled.len() as f64 * 1.5).ceil() as usize,
        "greedy <= 1.5*culled");
    // greedy shell is watertight over its 5 exposed faces: area == 5*N^2.
    assert!((surface_area(&greedy) - 5.0 * n * n).abs() < 1e-6, "greedy shell watertight (5 faces)");
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

    let surface = 5 * (n as usize).pow(2) * 2; // 5 exposed faces of the block (bottom on bedrock), 2 tris each
    assert_eq!(culled, surface, "culled = block surface only");
    assert!(culled < naive, "culled must be far less than naive for a solid block");
    assert!(greedy.len() <= (culled as f64 * 1.5).ceil() as usize, "greedy <= 1.5*culled");
    // block is a full n^3 shell (no holes), bottom on bedrock -> greedy covers 5*n^2 area.
    assert!((surface_area(&greedy) - 5.0 * n as f64 * n as f64).abs() < 1e-6,
        "greedy block watertight (5 faces, bedrock)");
}

#[test]
fn no_cracks_full_chunk_shell() {
    // A fully-solid chunk (border = air) exposes exactly its 6 outer faces. Greedy must
    // cover exactly 6*N^2 area with no cracks and no overlaps.
    let n = voxel_core::coords::CHUNK_SIZE;
    let chunk = build_chunk(|_, _, _| true);
    let greedy = greedy_mesh(&chunk);
    let expected_area = 5.0 * n as f64 * n as f64; // bottom face culled on bedrock floor
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
    let n = voxel_core::coords::CHUNK_SIZE;
    let chunk = build_chunk(|x, y, z| {
        x == 0 || y == 0 || z == 0 || x == n - 1 || y == n - 1 || z == n - 1
    });
    let greedy = greedy_mesh(&chunk);
    // 11 exposed faces * 2 triangles each = 22 triangles (greedy merges perfectly).
    // Outer bottom (y==0) is culled on bedrock; inner bottom (faces the hollow,
    // not bedrock) stays -> 5 outer + 6 inner = 11 faces.
    assert_eq!(greedy.len(), 22, "hollow shell => 11 faces * 2 tris (greedy merged, bedrock)");
    all_normals_axis_aligned(&greedy);
    // True surface area: 5*N^2 (outer, minus bottom) + 6*(N-2)^2 (inner, full 6 faces).
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
    let expected = 5.0 * n as f64 * n as f64 + 6.0 * (n as f64 - 2.0).powi(2);
    assert!((area - expected).abs() < expected * 0.02,
        "hollow shell area = 6*N^2 + 6*(N-2)^2: got {} expected {}", area, expected);
}

/// Vertex-AO (F5): a concave corner (where a solid neighbour overhangs a face corner)
/// must darken that corner (ao < 1.0), while a fully isolated voxel in open air must have
/// ao == 1.0 everywhere. This proves the mesher bakes ambient occlusion into the triangle
/// vertices rather than leaving it flat.
#[test]
fn vertex_ao_darkens_concave_corner() {
    // A 2x2 floor on y=0 with one voxel stacked on top at the (0,0) corner: the top face
    // of the floor is one greedy quad, and the corner under the stacked voxel is concave
    // (occluded by the overhang) -> its vertex AO must drop below 1.0.
    let chunk = build_chunk(|x, y, z| {
        (x <= 1 && z <= 1 && y == 0) || (x == 0 && y == 1 && z == 0)
    });
    let tris = greedy_mesh(&chunk);
    assert!(!tris.is_empty(), "shape must produce some triangles");

    let mut saw_concave = false;
    for t in &tris {
        let corners = [t.a, t.b, t.c];
        for ao in t.ao.iter() {
            if t.normal.y > 0.5 && *ao < 0.999 {
                saw_concave = true;
            }
        }
    }
    assert!(saw_concave, "concave corner (under the overhang) must darken (ao < 1.0)");
}

#[test]
fn vertex_ao_is_one_for_isolated_voxel() {
    // A single voxel on the bedrock floor (y==0) has its bottom face culled, so 10
    // triangles (5 faces), and with no occluders every corner AO is still 1.0.
    let chunk = build_chunk(|x, y, z| x == 0 && y == 0 && z == 0);
    let tris = greedy_mesh(&chunk);
    assert_eq!(tris.len(), 10, "single voxel on floor => 10 triangles (5 faces)");
    for t in &tris {
        for ao in t.ao.iter() {
            assert!((ao - 1.0).abs() < 1e-5, "isolated voxel corner AO must be 1.0, got {}", ao);
        }
    }
}
