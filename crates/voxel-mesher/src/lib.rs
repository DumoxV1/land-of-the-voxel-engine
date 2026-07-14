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

/// A single triangle: three vertices sharing a normal and material id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub a: Vec3,
    pub b: Vec3,
    pub c: Vec3,
    pub normal: Vec3,
    pub material: MaterialId,
}

/// View over a chunk's solidity and material, treating out-of-bounds as empty (air).
struct VoxView<'a> {
    chunk: &'a Chunk,
    size: i64,
}

impl<'a> VoxView<'a> {
    fn is_solid(&self, x: i64, y: i64, z: i64) -> bool {
        if x < 0 || y < 0 || z < 0 || x >= self.size || y >= self.size || z >= self.size {
            return false; // border is air
        }
        let m = self.chunk.get(LocalVoxel::new(x as u8, y as u8, z as u8));
        m != MaterialId::from(0u8)
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

/// Emit two triangles for a quad on a face of voxel (x,y,z), given the face normal
/// and the two in-plane tangent axes (u, v) with their lengths (du, dv).
fn emit_quad(
    out: &mut Vec<(Triangle, MaterialId)>,
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
    out.push((Triangle { a: p0, b: p1, c: p2, normal, material }, material));
    out.push((Triangle { a: p0, b: p2, c: p3, normal, material }, material));
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
                    emit_face(&mut out, x, y, z, *nrm, mat);
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
                    emit_face(&mut out, x, y, z, *nrm, mat);
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

/// Emit a single unit face (quad = 2 triangles) on the +normal side of voxel (x,y,z).
fn emit_face(out: &mut Vec<(Triangle, MaterialId)>, x: i64, y: i64, z: i64, normal: Vec3, material: MaterialId) {
    let (u, v) = tangent_basis(normal);
    emit_quad(out, x as f64, y as f64, z as f64, normal, u, v, 1.0, 1.0, material);
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

    let mut mask: Vec<Vec<Option<MaterialId>>> = vec![vec![None; n]; n];

    for d in 0..n {
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
                if view.is_solid(x, y, z) && !neighbour_solid {
                    mask[i][j] = Some(view.material(x, y, z));
                } else {
                    mask[i][j] = None;
                }
            }
        }
        let mut i = 0;
        while i < n {
            let mut j = 0;
            while j < n {
                if let Some(mat) = mask[i][j] {
                    let mut w = 1;
                    while j + w < n && mask[i][j + w] == Some(mat) {
                        w += 1;
                    }
                    let mut h = 1;
                    'outer: while i + h < n {
                        for jj in j..j + w {
                            if mask[i + h][jj] != Some(mat) {
                                break 'outer;
                            }
                        }
                        h += 1;
                    }
                    // Emit one merged quad.
                    emit_merged_quad(out, axis, u_axis, v_axis, d, i, j, h, w, normal, mat);
                    for ii in i..i + h {
                        for jj in j..j + w {
                            mask[ii][jj] = None;
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
    let mut base = [0.0f64; 3];
    base[axis] = d as f64;
    base[u_axis] = i as f64;
    base[v_axis] = j as f64;

    let mut u = [0.0f64; 3];
    u[u_axis] = 1.0;
    let mut v = [0.0f64; 3];
    v[v_axis] = 1.0;

    emit_quad(
        out,
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
