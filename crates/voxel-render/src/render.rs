//! Minimal software rasterizer (S-03): project greedy-mesh triangles, z-buffer, shade.
//!
//! Pure Rust, no GPU. Output is a PNG via the `image` crate. This is a spike to make the
//! `voxel_mesher` output visible end-to-end; it is intentionally simple (per-face normal
//! shading + material colour, no lighting model).

use crate::camera::Camera;
use image::{ImageBuffer, Rgba, RgbaImage};
use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, CHUNK_SIZE};
use voxel_core::palette::MaterialId;
use voxel_mesher::{Triangle, Vec3, greedy_mesh};

/// Background colour for empty space (RGBA).
pub const BACKGROUND: [u8; 4] = [20, 20, 28, 255];

/// Fixed material palette colours (index 0 = air/empty, kept for completeness).
const MATERIAL_COLORS: [[u8; 3]; 16] = [
    [60, 60, 70],    // 0
    [150, 90, 60],   // 1 - dirt/stone
    [90, 160, 80],   // 2 - grass
    [120, 120, 130], // 3 - stone
    [200, 200, 210], // 4 - metal
    [80, 140, 200],  // 5 - water-ish
    [180, 120, 60],  // 6
    [160, 160, 60],  // 7
    [200, 100, 100], // 8
    [100, 200, 160], // 9
    [140, 100, 200], // 10
    [200, 160, 90],  // 11
    [90, 110, 200],  // 12
    [200, 90, 160],  // 13
    [120, 200, 90],  // 14
    [210, 210, 120], // 15
];

/// Render a single chunk to an `RgbaImage` of the given dimensions.
pub fn render_scene(chunk: &Chunk, cam: &Camera, w: u32, h: u32) -> RgbaImage {
    let tris = greedy_mesh(chunk);
    rasterize(&tris, cam, w, h)
}

/// Render many chunks (a `World` view) to one image. Each `(coord, chunk)` is meshed and the
/// triangles are offset into world space by `coord * CHUNK_SIZE` so adjacent chunks line up.
/// `chunks` is a slice of (chunk coord, chunk data); the caller supplies them (no hard
/// dependency on a world store, keeping the renderer agnostic).
pub fn render_world(chunks: &[(ChunkCoord, Chunk)], cam: &Camera, w: u32, h: u32) -> RgbaImage {
    let mut all: Vec<Triangle> = Vec::new();
    let s = CHUNK_SIZE as f32;
    for (coord, chunk) in chunks {
        let ox = coord.x as f32 * s;
        let oy = coord.y as f32 * s;
        let oz = coord.z as f32 * s;
        for t in greedy_mesh(chunk) {
            all.push(Triangle {
                a: Vec3::new(t.a.x + ox, t.a.y + oy, t.a.z + oz),
                b: Vec3::new(t.b.x + ox, t.b.y + oy, t.b.z + oz),
                c: Vec3::new(t.c.x + ox, t.c.y + oy, t.c.z + oz),
                normal: t.normal,
                material: t.material,
                ao: t.ao,
                sun: [1.0; 3],
            });
        }
    }
    rasterize(&all, cam, w, h)
}

/// Build a perspective view+projection and rasterize the triangles with a z-buffer.
fn rasterize(tris: &[Triangle], cam: &Camera, w: u32, h: u32) -> RgbaImage {
    let mut img: RgbaImage = ImageBuffer::from_pixel(w, h, Rgba(BACKGROUND));
    if tris.is_empty() {
        return img;
    }
    let target = [16.0f32, 16.0, 16.0]; // chunk center (CHUNK_SIZE=32)
    let eye = cam.eye(target);
    let (view, proj) = view_proj(&eye, &target, cam.fov_deg, w, h);

    // Transform all triangles to screen space (and clip).
    let mut buffer: Vec<f32> = vec![f32::INFINITY; (w * h) as usize];

    for t in tris {
        let v0 = project_vertex(&t.a, &view, &proj, w, h);
        let v1 = project_vertex(&t.b, &view, &proj, w, h);
        let v2 = project_vertex(&t.c, &view, &proj, w, h);
        // Skip if any vertex is behind the camera (w<=0) for this minimal spike.
        if v0[3] <= 0.0 || v1[3] <= 0.0 || v2[3] <= 0.0 {
            continue;
        }
        shade_triangle(&mut img, &mut buffer, v0, v1, v2, t.normal, t.material);
    }
    img
}

/// Returns (view, proj) as 4x4 row-major matrices acting on [x,y,z,1].
/// Convention: right-handed, camera looks down +z_local (DX-style). Points in front of the
/// camera have positive local z, and clip w (= local z) is positive.
fn view_proj(eye: &[f32; 3], target: &[f32; 3], fov_deg: f32, w: u32, h: u32) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    // Look-at basis: forward (eye -> target), right = cross(forward, up), up' = cross(right, forward).
    let mut fwd = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
    normalize(&mut fwd);
    let up = [0.0f32, 1.0, 0.0];
    let mut right = cross(&fwd, &up);
    normalize(&mut right);
    let true_up = cross(&right, &fwd);

    // View: rotate world into camera space, translate eye to origin.
    let mut view = [[0.0f32; 4]; 4];
    for i in 0..3 {
        view[0][i] = right[i];
        view[1][i] = true_up[i];
        view[2][i] = fwd[i];
    }
    view[0][3] = -dot(&right, eye);
    view[1][3] = -dot(&true_up, eye);
    view[2][3] = -dot(&fwd, eye);
    view[3][3] = 1.0;

    let aspect = w as f32 / h as f32;
    let f = 1.0 / (fov_deg.to_radians() / 2.0).tan();
    let near = 0.1f32;
    let far = 1000.0f32;
    let mut proj = [[0.0f32; 4]; 4];
    proj[0][0] = f / aspect;
    proj[1][1] = f;
    proj[2][2] = far / (near - far);
    proj[2][3] = (far * near) / (near - far);
    proj[3][2] = 1.0; // clip w = local z (>0 in front)

    (view, proj)
}

/// Project a world vertex to (sx, sy, depth_proxy, w_clip) in screen space.
/// Returns w_clip <= 0 if the vertex is behind the camera.
fn project_vertex(p: &Vec3, view: &[[f32; 4]; 4], proj: &[[f32; 4]; 4], w: u32, h: u32) -> [f32; 4] {
    let wp = mul4(view, [p.x, p.y, p.z, 1.0]);
    let cp = mul4(proj, wp);
    if cp[3] <= 0.0 {
        return [0.0, 0.0, 0.0, cp[3]];
    }
    let inv = 1.0 / cp[3];
    let ndc_x = cp[0] * inv;
    let ndc_y = cp[1] * inv;
    let sx = (ndc_x * 0.5 + 0.5) * w as f32;
    let sy = (1.0 - (ndc_y * 0.5 + 0.5)) * h as f32; // flip Y for image coords
    [sx, sy, cp[2], cp[3]]
}

/// Rasterize one triangle with a z-buffer and per-face normal shading.
fn shade_triangle(
    img: &mut RgbaImage,
    zbuf: &mut [f32],
    v0: [f32; 4],
    v1: [f32; 4],
    v2: [f32; 4],
    normal: Vec3,
    material: MaterialId,
) {
    let (w, h) = (img.width(), img.height());
    let min_x = v0[0].min(v1[0]).min(v2[0]).floor().max(0.0) as i32;
    let max_x = v0[0].max(v1[0]).max(v2[0]).ceil().min(w as f32 - 1.0) as i32;
    let min_y = v0[1].min(v1[1]).min(v2[1]).floor().max(0.0) as i32;
    let max_y = v0[1].max(v1[1]).max(v2[1]).ceil().min(h as f32 - 1.0) as i32;

    let area = edge(v0, v1, v2);
    if area.abs() < 1e-6 {
        return;
    }

    let base = mat_color(material);
    // Simple Lambert against a fixed light direction (normalized).
    let light = normalize3([0.4, 0.8, 0.6]);
    let n = normalize3([normal.x, normal.y, normal.z]);
    let lamb = (dot3(&n, &light)).max(0.0);
    let shade = 0.35 + 0.65 * lamb; // ambient + diffuse
    let color = [
        (base[0] as f32 * shade).clamp(0.0, 255.0) as u8,
        (base[1] as f32 * shade).clamp(0.0, 255.0) as u8,
        (base[2] as f32 * shade).clamp(0.0, 255.0) as u8,
    ];

    for py in min_y..=max_y {
        for px in min_x..=max_x {
            let p = [px as f32 + 0.5, py as f32 + 0.5];
            let w0 = edge([v1[0], v1[1], 0.0, 0.0], [v2[0], v2[1], 0.0, 0.0], [p[0], p[1], 0.0, 0.0]) / area;
            let w1 = edge([v2[0], v2[1], 0.0, 0.0], [v0[0], v0[1], 0.0, 0.0], [p[0], p[1], 0.0, 0.0]) / area;
            let w2 = edge([v0[0], v0[1], 0.0, 0.0], [v1[0], v1[1], 0.0, 0.0], [p[0], p[1], 0.0, 0.0]) / area;
            if w0 < -0.001 || w1 < -0.001 || w2 < -0.001 {
                continue;
            }
            // Perspective-correct depth (use w as proxy for 1/z).
            let depth = w0 / v0[3] + w1 / v1[3] + w2 / v2[3];
            let depth = 1.0 / depth;
            let idx = (py as u32 * w + px as u32) as usize;
            if depth < zbuf[idx] {
                zbuf[idx] = depth;
                img.put_pixel(px as u32, py as u32, Rgba([color[0], color[1], color[2], 255]));
            }
        }
    }
}

/// Edge function (2D cross) for barycentric coverage test.
fn edge(a: [f32; 4], b: [f32; 4], c: [f32; 4]) -> f32 {
    (c[0] - a[0]) * (b[1] - a[1]) - (c[1] - a[1]) * (b[0] - a[0])
}

fn mat_color(m: MaterialId) -> [u8; 3] {
    let i = (m.0 & 0x0F) as usize;
    MATERIAL_COLORS[i.min(15)]
}

fn mul4(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for r in 0..4 {
        out[r] = m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2] + m[r][3] * v[3];
    }
    out
}

fn cross(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: &mut [f32; 3]) {
    let l = dot(v, v).sqrt().max(1e-8);
    v[0] /= l;
    v[1] /= l;
    v[2] /= l;
}

fn dot3(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / l, v[1] / l, v[2] / l]
}
