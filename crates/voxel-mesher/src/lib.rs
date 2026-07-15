//! voxel-mesher: renderer-agnostic voxel meshing (S-02).
//!
//! Provides three progressive mesher backends over a `voxel_core::Chunk`:
//!  - `naive_mesh`  — one cube (6 faces) per solid voxel.
//!  - `culled_mesh` — removes faces adjacent to a solid neighbour.
//!  - `greedy_mesh` — merges co-planar, same-material faces into maximal quads.
//!
//! All output is pure data (`Triangle`): position + normal + material. No GPU/renderer
//! dependency (ADR-0002). The chunk border is treated as EMPTY (air) so exposed outer
//! faces are generated.

use voxel_core::chunk::Chunk;
use voxel_core::coords::{LocalVoxel, CHUNK_SIZE};
use voxel_core::palette::MaterialId;

/// A 3D vector (f32) used for vertex positions and face normals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// A single triangle: three vertices sharing a normal and material id. `ao` holds the
/// per-corner (a, b, c) vertex ambient occlusion in [0,1] (1.0 = fully lit / open sky,
/// lower = crevice). Baked at mesh time (F5 vertex-AO, 0 runtime cost).
///
/// `sun` is the per-corner combined sun+hemisphere light in [0,1] (1.0 = full sky light,
/// lower = in shadow / deep inside a cave). Baked at mesh time by the BFS sunlight
/// propagation pass (Stap 3, 2026-07-15) — 0 runtime cost on the GPU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub normal: Vec3,
    pub material: MaterialId,
    pub ao: [f32; 3],
    pub sun: [f32; 3],
}

/// View over a chunk's solidity and material, treating out-of-bounds as empty (air).
struct VoxView<'a> {
    chunk: &'a Chunk,
    size: i64,
}

impl<'a> VoxView<'a> {
    fn is_solid(&self, x: i64, y: i64, z: i64) -> bool {
        if x < 0 || z < 0 || x >= self.size || z >= self.size {
            return false; // horizontal border is air
        }
        if y < 0 {
            return true; // bedrock: virtual solid floor below the world, so the
                         // bottom faces of the lowest terrain layer are culled and
                         // you never see "under the map" when flying beneath it.
        }
        if y >= self.size {
            return false; // sky above is air
        }
        let m = self.chunk.get(LocalVoxel::new(x as u8, y as u8, z as u8));
        m != MaterialId::from(0u8)
    }

    /// Like `is_solid`, but treats the virtual bedrock floor (y<0) as AIR so it never
    /// acts as an AO occluder — the floor is not a real surface, just a culling boundary.
    fn is_solid_ao(&self, x: i64, y: i64, z: i64) -> bool {
        if y < 0 {
            return false;
        }
        self.is_solid(x, y, z)
    }

    fn material(&self, x: i64, y: i64, z: i64) -> MaterialId {
        self.chunk.get(LocalVoxel::new(x as u8, y as u8, z as u8))
    }
}

/// The six face directions, with their outward normals.
const FACES: [(i64, i64, i64, Vec3); 6] = [
    (1, 0, 0, Vec3::new(1.0, 0.0, 0.0)),
    (-1, 0, 0, Vec3::new(-1.0, 0.0, 0.0)),
    (0, 1, 0, Vec3::new(0.0, 1.0, 0.0)),
    (0, -1, 0, Vec3::new(0.0, -1.0, 0.0)),
    (0, 0, 1, Vec3::new(0.0, 0.0, 1.0)),
    (0, 0, -1, Vec3::new(0.0, 0.0, -1.0)),
];

/// Vertex ambient occlusion (F5, 0-fps method). For a face on solid voxel (x,y,z) with
/// outward unit normal `n` and in-plane unit tangents `u`,`v`, compute AO at the corner
/// (su,sv) in {0,1} (low/high edge along each tangent). Samples the three voxels that
/// border that corner on the SOLID side (-n direction): the two edge voxels and the
/// diagonal. Returns 0.4 (fully occluded crevice) .. 1.0 (open sky).
fn corner_ao(
    view: &VoxView,
    x: i64,
    y: i64,
    z: i64,
    n: Vec3,
    u: (f64, f64, f64),
    v: (f64, f64, f64),
    su: i64,
    sv: i64,
) -> f32 {
    // Step INTO the solid (opposite the outward face normal) for the occluder samples.
    let nx = -n.x as f64;
    let ny = -n.y as f64;
    let nz = -n.z as f64;
    let su_s = if su == 0 { -1.0f64 } else { 1.0 };
    let sv_s = if sv == 0 { -1.0f64 } else { 1.0 };
    let sx = (x as f64 + nx + u.0 * su_s) as i64;
    let sy = (y as f64 + ny + u.1 * su_s) as i64;
    let sz = (z as f64 + nz + u.2 * su_s) as i64;
    let s1 = view.is_solid_ao(sx, sy, sz) as u8;
    let tx = (x as f64 + nx + v.0 * sv_s) as i64;
    let ty = (y as f64 + ny + v.1 * sv_s) as i64;
    let tz = (z as f64 + nz + v.2 * sv_s) as i64;
    let s2 = view.is_solid_ao(tx, ty, tz) as u8;
    let cx = (x as f64 + nx + u.0 * su_s + v.0 * sv_s) as i64;
    let cy = (y as f64 + ny + u.1 * su_s + v.1 * sv_s) as i64;
    let cz = (z as f64 + nz + u.2 * su_s + v.2 * sv_s) as i64;
    let c = view.is_solid_ao(cx, cy, cz) as u8;
    // Minecraft-style: if both edge voxels are solid the corner is fully occluded.
    let occ = if s1 == 1 && s2 == 1 { 3u8 } else { s1 + s2 + c };
    let t = occ as f32 / 3.0;
    1.0 - 0.6 * t // 1.0 open .. 0.4 fully blocked
}

/// Emit two triangles for a quad on a face of voxel (x,y,z), given the face normal
/// and the two in-plane tangent axes (u, v) with their lengths (du, dv).
fn emit_quad(
    out: &mut Vec<(Triangle, MaterialId)>,
    view: &VoxView,
    x: f64,
    y: f64,
    z: f64,
    normal: Vec3,
    u: (f64, f64, f64),
    v: (f64, f64, f64),
    du: f64,
    dv: f64,
    material: MaterialId,
) {
    let base = Vec3::new(x as f32, y as f32, z as f32);
    let corner = |su: f64, sv: f64| Vec3::new(
        (base.x as f64 + u.0 * su + v.0 * sv) as f32,
        (base.y as f64 + u.1 * su + v.1 * sv) as f32,
        (base.z as f64 + u.2 * su + v.2 * sv) as f32,
    );
    let p0 = corner(0.0, 0.0);
    let p1 = corner(du, 0.0);
    let p2 = corner(du, dv);
    let p3 = corner(0.0, dv);
    // Winding must be CCW seen from outside (geometric normal == face normal) so
    // backface culling works (S-11 audit fix). The tangent basis is fixed per axis, so
    // flip the vertex order when cross(u, v) points against the face normal.
    let gx = (u.1 * v.2 - u.2 * v.1) as f32;
    let gy = (u.2 * v.0 - u.0 * v.2) as f32;
    let gz = (u.0 * v.1 - u.1 * v.0) as f32;
    // Per-corner vertex AO (F5), indexed by (su,sv) == (0,0),(1,0),(1,1),(0,1).
    let vx = x as i64;
    let vy = y as i64;
    let vz = z as i64;
    let ao_p0 = corner_ao(view, vx, vy, vz, normal, u, v, 0, 0);
    let ao_p1 = corner_ao(view, vx, vy, vz, normal, u, v, 1, 0);
    let ao_p2 = corner_ao(view, vx, vy, vz, normal, u, v, 1, 1);
    let ao_p3 = corner_ao(view, vx, vy, vz, normal, u, v, 0, 1);
    if gx * normal.x + gy * normal.y + gz * normal.z >= 0.0 {
        out.push((
            Triangle { a: p0, b: p1, c: p2, normal, material, ao: [ao_p0, ao_p1, ao_p2], sun: [1.0; 3] },
            material,
        ));
        out.push((
            Triangle { a: p0, b: p2, c: p3, normal, material, ao: [ao_p0, ao_p2, ao_p3], sun: [1.0; 3] },
            material,
        ));
    } else {
        out.push((
            Triangle { a: p0, b: p2, c: p1, normal, material, ao: [ao_p0, ao_p2, ao_p1], sun: [1.0; 3] },
            material,
        ));
        out.push((
            Triangle { a: p0, b: p3, c: p2, normal, material, ao: [ao_p0, ao_p3, ao_p2], sun: [1.0; 3] },
            material,
        ));
    }
}

/// Naïve mesher: one cube (6 faces) per solid voxel, regardless of neighbours.
pub fn naive_mesh(chunk: &Chunk) -> Vec<Triangle> {
    let view = VoxView { chunk, size: CHUNK_SIZE };
    let mut out: Vec<(Triangle, MaterialId)> = Vec::new();
    let n = CHUNK_SIZE;
    for x in 0..n {
        for y in 0..n {
            for z in 0..n {
                if !view.is_solid(x, y, z) {
                    continue;
                }
                let mat = view.material(x, y, z);
                for (_, _, _, nrm) in FACES.iter() {
                    emit_face(&mut out, &view, x, y, z, *nrm, mat);
                }
            }
        }
    }
    out.into_iter().map(|(t, _)| t).collect()
}

/// Culled mesher: skip a face when its neighbour in the face direction is solid.
pub fn culled_mesh(chunk: &Chunk) -> Vec<Triangle> {
    let view = VoxView { chunk, size: CHUNK_SIZE };
    let mut out: Vec<(Triangle, MaterialId)> = Vec::new();
    let n = CHUNK_SIZE;
    for x in 0..n {
        for y in 0..n {
            for z in 0..n {
                if !view.is_solid(x, y, z) {
                    continue;
                }
                let mat = view.material(x, y, z);
                for (dx, dy, dz, nrm) in FACES.iter() {
                    if view.is_solid(x + dx, y + dy, z + dz) {
                        continue; // neighbour solid -> cull this face
                    }
                    emit_face(&mut out, &view, x, y, z, *nrm, mat);
                }
            }
        }
    }
    out.into_iter().map(|(t, _)| t).collect()
}

/// Greedy mesher: per (axis, sign) layer, greedily merge co-planar, same-material,
/// same-normal faces into maximal axis-aligned quads.
pub fn greedy_mesh(chunk: &Chunk) -> Vec<Triangle> {
    let view = VoxView { chunk, size: CHUNK_SIZE };
    let mut out: Vec<(Triangle, MaterialId)> = Vec::new();
    let n = CHUNK_SIZE as usize;
    for axis in 0..3 {
        for sign in [0usize, 1usize] {
            greedy_layer(&mut out, &view, axis, sign, n);
        }
    }
    out.into_iter().map(|(t, _)| t).collect()
}

/// Emit a single unit face (quad = 2 triangles) on the `normal` side of voxel (x,y,z).
/// A voxel at (x,y,z) fills [x,x+1)x[y,y+1)x[z,z+1): positive faces lie on the +1 planes
/// (S-11 audit fix — previously all faces collapsed onto the min-corner planes).
fn emit_face(out: &mut Vec<(Triangle, MaterialId)>, view: &VoxView, x: i64, y: i64, z: i64, normal: Vec3, material: MaterialId) {
    let (u, v) = tangent_basis(normal);
    let bx = x as f64 + if normal.x > 0.0 { 1.0 } else { 0.0 };
    let by = y as f64 + if normal.y > 0.0 { 1.0 } else { 0.0 };
    let bz = z as f64 + if normal.z > 0.0 { 1.0 } else { 0.0 };
    emit_quad(out, view, bx, by, bz, normal, u, v, 1.0, 1.0, material);
}

/// Choose in-plane tangent unit vectors for a given axis-aligned normal.
fn tangent_basis(n: Vec3) -> ((f64, f64, f64), (f64, f64, f64)) {
    if n.x != 0.0 {
        ((0.0, 1.0, 0.0), (0.0, 0.0, 1.0))
    } else if n.y != 0.0 {
        ((1.0, 0.0, 0.0), (0.0, 0.0, 1.0))
    } else {
        ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0))
    }
}

/// Greedy sweep over one (axis, sign) layer.
fn greedy_layer(out: &mut Vec<(Triangle, MaterialId)>, view: &VoxView, axis: usize, sign: usize, n: usize) {
    let normal = {
        let mut v = [0.0f32; 3];
        v[axis] = if sign == 1 { 1.0 } else { -1.0 };
        Vec3::new(v[0], v[1], v[2])
    };
    let (u_axis, v_axis) = other_axes(axis);

    // Flat mask buffer (n*n), allocated ONCE and reused across all `d` layers. The old
    // code allocated a `Vec<Vec<_>>` of n*n entries *per layer* (6*n allocations/chunk) —
    // pure GC churn on the rayon pool. Indexed as `i*n + j`.
    let mut mask: Vec<Option<MaterialId>> = vec![None; n * n];

    for d in 0..n {
        // Reset only the entries we will touch this layer.
        mask.fill(None);
        for i in 0..n {
            for j in 0..n {
                let (x, y, z) = coords_for(axis, u_axis, v_axis, d, i, j, n);
                let neighbour_solid = if sign == 1 {
                    view.is_solid(
                        x + if axis == 0 { 1 } else { 0 },
                        y + if axis == 1 { 1 } else { 0 },
                        z + if axis == 2 { 1 } else { 0 },
                    )
                } else {
                    view.is_solid(
                        x - if axis == 0 { 1 } else { 0 },
                        y - if axis == 1 { 1 } else { 0 },
                        z - if axis == 2 { 1 } else { 0 },
                    )
                };
                let idx = i * n + j;
                if view.is_solid(x, y, z) && !neighbour_solid {
                    mask[idx] = Some(view.material(x, y, z));
                } else {
                    mask[idx] = None;
                }
            }
        }
        let mut i = 0;
        while i < n {
            let mut j = 0;
            while j < n {
                let idx = i * n + j;
                if let Some(mat) = mask[idx] {
                    let mut w = 1;
                    while j + w < n && mask[i * n + (j + w)] == Some(mat) {
                        w += 1;
                    }
                    let mut h = 1;
                    'outer: while i + h < n {
                        for jj in j..j + w {
                            if mask[(i + h) * n + jj] != Some(mat) {
                                break 'outer;
                            }
                        }
                        h += 1;
                    }
                    // Emit one merged quad.
                    emit_merged_quad(out, view, axis, u_axis, v_axis, d, i, j, h, w, normal, mat);
                    for ii in i..i + h {
                        for jj in j..j + w {
                            mask[ii * n + jj] = None;
                        }
                    }
                    j += w;
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
}

/// Map (layer d, u-index i, v-index j) -> (x,y,z) world-local voxel coords.
fn coords_for(axis: usize, u_axis: usize, v_axis: usize, d: usize, i: usize, j: usize, n: usize) -> (i64, i64, i64) {
    let mut c = [0i64; 3];
    c[axis] = d as i64;
    c[u_axis] = i as i64;
    c[v_axis] = j as i64;
    for k in 0..3 {
        if c[k] >= n as i64 {
            c[k] = n as i64 - 1;
        }
    }
    (c[0], c[1], c[2])
}

fn other_axes(axis: usize) -> (usize, usize) {
    match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    }
}

/// Emit a greedily-merged quad spanning h x w voxels in the layer plane.
fn emit_merged_quad(
    out: &mut Vec<(Triangle, MaterialId)>,
    view: &VoxView,
    axis: usize,
    u_axis: usize,
    v_axis: usize,
    d: usize,
    i: usize,
    j: usize,
    h: usize,
    w: usize,
    normal: Vec3,
    material: MaterialId,
) {
    // Positive faces lie on the far plane of the voxel layer (d+1); negative on d.
    // (S-11 audit fix.)
    let plane = if normal.x + normal.y + normal.z > 0.0 { d as f64 + 1.0 } else { d as f64 };
    let mut base = [0.0f64; 3];
    base[axis] = plane;
    base[u_axis] = i as f64;
    base[v_axis] = j as f64;

    let mut u = [0.0f64; 3];
    u[u_axis] = 1.0;
    let mut v = [0.0f64; 3];
    v[v_axis] = 1.0;

    emit_quad(
        out,
        view,
        base[0],
        base[1],
        base[2],
        normal,
        (u[0], u[1], u[2]),
        (v[0], v[1], v[2]),
        h as f64,
        w as f64,
        material,
    );
}
