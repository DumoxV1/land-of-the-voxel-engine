//! Sunlight propagation (Stap 3 — BFS flood-fill, 2026-07-15).
//!
//! Computes per-voxel *sky light* for a set of streamed chunks and bakes the resulting
//! normalised [0,1] sun value into each triangle's `sun` attribute. The algorithm is the
//! classic Seed-of-Andromeda BFS flood-fill (Minecraft-style sunlight), extended to span
//! chunk-Y boundaries so caves and overhangs under tall terrain are correctly shadowed.
//!
//! Design notes / trade-offs:
//! - We never touch `voxel-core::Chunk`'s storage (keeps it storage-agnostic and avoids
//!   changing the serialized chunk format). Instead we build a temporary solidity grid from
//!   the chunks we are meshing.
//! - The grid is world-keyed: each voxel maps to a `(chunk, local)` pair, so a solid voxel in
//!   an *adjacent* chunk correctly blocks light propagating into the chunk we are meshing.
//! - Sunlight = 15 at the top of a sky column, decays by 1 per BFS step (down + sideways).
//!   Below an opaque roof it drops to ~0 (cave shadow). `sun` is then normalised to [0,1]
//!   and baked per triangle corner from the 3 voxels behind the face (solid side, like AO).
//!
//! This module is CPU-only (it runs in the streaming worker, never on the GPU).

use std::collections::{HashMap, HashSet, VecDeque};

use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, LocalVoxel, CHUNK_SIZE};
use voxel_mesher::{Triangle, Vec3};

/// Maximum sunlight level (Minecraft convention). 15 = direct sky, 0 = full shadow.
const MAX_LIGHT: i32 = 15;

/// A single solid voxel in the world-keyed grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VoxKey {
    cx: i64,
    cy: i64,
    cz: i64,
    lx: u8,
    ly: u8,
    lz: u8,
}

impl VoxKey {
    /// Map a world voxel to its grid key via Euclidean div/rem (negative-safe).
    fn from_world(wx: i64, wy: i64, wz: i64) -> Self {
        let cx = wx.div_euclid(CHUNK_SIZE);
        let cy = wy.div_euclid(CHUNK_SIZE);
        let cz = wz.div_euclid(CHUNK_SIZE);
        let lx = wx.rem_euclid(CHUNK_SIZE) as u8;
        let ly = wy.rem_euclid(CHUNK_SIZE) as u8;
        let lz = wz.rem_euclid(CHUNK_SIZE) as u8;
        VoxKey { cx, cy, cz, lx, ly, lz }
    }

    /// Reconstruct the world voxel position.
    fn to_world(&self) -> (i64, i64, i64) {
        (
            self.cx * CHUNK_SIZE + self.lx as i64,
            self.cy * CHUNK_SIZE + self.ly as i64,
            self.cz * CHUNK_SIZE + self.lz as i64,
        )
    }
}

/// World-keyed solidity + light grid over the streamed chunk set.
pub(crate) struct LightGrid {
    solid: HashSet<VoxKey>,
    light: HashMap<VoxKey, i32>,
    /// Set of all chunk coords that contributed voxels (so we can map results back).
    chunks: Vec<ChunkCoord>,
}

impl LightGrid {
    /// Build the grid from a set of chunks. Only the voxels inside these chunks are
    /// considered; voxels "in the air" (not part of any provided chunk) are treated as
    /// transparent so light can flow between chunks — but solid voxels in neighbouring
    /// provided chunks still block light.
    pub(crate) fn from_chunks(chunks: &[Chunk]) -> Self {
        let mut solid = HashSet::new();
        let mut seen_coord: HashMap<(i64, i64, i64), ChunkCoord> = HashMap::new();
        let mut coord_vec: Vec<ChunkCoord> = Vec::new();
        for chunk in chunks {
            let coord = chunk.coord;
            if seen_coord.insert((coord.x, coord.y, coord.z), coord).is_none() {
                coord_vec.push(coord);
            }
            // Early-out: uniform AIR chunk contributes nothing.
            if chunk.is_empty() {
                continue;
            }
            for ly in 0..CHUNK_SIZE as u8 {
                for lz in 0..CHUNK_SIZE as u8 {
                    for lx in 0..CHUNK_SIZE as u8 {
                        let m = chunk.get(LocalVoxel::new(lx, ly, lz)).0;
                        if m != 0 {
                            solid.insert(VoxKey {
                                cx: coord.x,
                                cy: coord.y,
                                cz: coord.z,
                                lx,
                                ly,
                                lz,
                            });
                        }
                    }
                }
            }
        }
        LightGrid {
            solid,
            light: HashMap::new(),
            chunks: coord_vec,
        }
    }

    /// True if the world voxel mapped by `k` lies outside every provided chunk (i.e. it is
    /// the real sky above our grid, or air beyond the streamed set). Used to seed sunlight
    /// only at genuine sky-exposed voxels.
    pub(crate) fn is_out_of_grid(&self, k: &VoxKey) -> bool {
        !self
            .chunks
            .iter()
            .any(|c| c.x == k.cx && c.y == k.cy && c.z == k.cz)
    }

    pub(crate) fn is_solid(&self, k: &VoxKey) -> bool {
        self.solid.contains(k)
    }

    /// Propagate sunlight via BFS (Seed of Andromeda), in two passes:
    ///   1. SKY-FILL (no decay): every air voxel with a clear vertical path to the world top
    ///      (i.e. reachable from a voxel whose `above` is outside our grid) is full sky (15).
    ///      This fills an entire open air column down to the ground — even if it is deeper
    ///      than MAX_LIGHT, because an open shaft is fully lit to its floor.
    ///   2. DECAY: from those sky voxels, light spreads to neighbours with -1 per step, so a
    ///      voxel under a roof / around a corner receives only the dimmed value (cave shadow).
    /// A voxel trapped between a roof and the ground has no path to the grid top, so it is
    /// never seeded and stays dark.
    pub(crate) fn propagate(&mut self, y_max: i64) {
        let mut queue: VecDeque<(VoxKey, i32)> = VecDeque::new();

        // Seed: every air voxel whose voxel straight ABOVE is OUTSIDE our grid (the real sky)
        // starts at MAX_LIGHT. This is the only true sky-exposure: anything with a solid or
        // in-grid voxel above it is either blocked or part of an interior pocket.
        for &coord in &self.chunks {
            for ly in 0..CHUNK_SIZE as u8 {
                for lz in 0..CHUNK_SIZE as u8 {
                    for lx in 0..CHUNK_SIZE as u8 {
                        let key = VoxKey {
                            cx: coord.x,
                            cy: coord.y,
                            cz: coord.z,
                            lx,
                            ly,
                            lz,
                        };
                        if self.is_solid(&key) {
                            continue;
                        }
                        let (_wx, wy, _wz) = key.to_world();
                        if wy > y_max {
                            continue;
                        }
                        let above = VoxKey::from_world(
                            key.cx * CHUNK_SIZE + key.lx as i64,
                            wy + 1,
                            key.cz * CHUNK_SIZE + key.lz as i64,
                        );
                        if self.is_out_of_grid(&above) {
                            self.light.insert(key, MAX_LIGHT);
                            queue.push_back((key, MAX_LIGHT));
                        }
                    }
                }
            }
        }

        // Pass 1 — sky fill (no decay): all IN-GRID air neighbours of a sky voxel become sky
        // too. This propagates the full-sky value DOWN an open column (and sideways into open
        // pockets) without losing brightness with depth. We restrict to in-grid voxels:
        // out-of-grid air is already treated as full sky by `sun_at_world`, and letting the
        // flood leave the grid would never terminate (infinite open space).
        let mut sky_q: VecDeque<VoxKey> = queue.iter().map(|(k, _)| *k).collect();
        while let Some(key) = sky_q.pop_front() {
            let (wx, wy, wz) = key.to_world();
            let neighbours = [
                (wx + 1, wy, wz),
                (wx - 1, wy, wz),
                (wx, wy + 1, wz),
                (wx, wy - 1, wz),
                (wx, wy, wz + 1),
                (wx, wy, wz - 1),
            ];
            for (nx, ny, nz) in neighbours {
                if ny > y_max {
                    continue;
                }
                let nkey = VoxKey::from_world(nx, ny, nz);
                if !self.in_grid(&nkey) {
                    continue; // out-of-grid air is already full sky; don't flood past the grid
                }
                if self.is_solid(&nkey) {
                    continue;
                }
                if self.light.get(&nkey).copied().unwrap_or(0) != MAX_LIGHT {
                    self.light.insert(nkey, MAX_LIGHT);
                    sky_q.push_back(nkey);
                }
            }
        }

        // Pass 2 — decay flood from the sky voxels. A neighbour gets max(current, light-1),
        // so open columns (already 15) keep 15, while voxels reached only around a corner /
        // under an overhang get the dimmed, shadowed value.
        while let Some((key, light)) = queue.pop_front() {
            if light <= 1 {
                continue;
            }
            let (wx, wy, wz) = key.to_world();
            let neighbours = [
                (wx + 1, wy, wz),
                (wx - 1, wy, wz),
                (wx, wy + 1, wz),
                (wx, wy - 1, wz),
                (wx, wy, wz + 1),
                (wx, wy, wz - 1),
            ];
            for (nx, ny, nz) in neighbours {
                if ny > y_max {
                    continue;
                }
                let nkey = VoxKey::from_world(nx, ny, nz);
                if !self.in_grid(&nkey) {
                    continue; // out-of-grid air is already full sky via sun_at_world
                }
                if self.is_solid(&nkey) {
                    continue;
                }
                let nlight = light - 1;
                let cur = self.light.get(&nkey).copied().unwrap_or(0);
                if nlight > cur {
                    self.light.insert(nkey, nlight);
                    queue.push_back((nkey, nlight));
                }
            }
        }
    }

    /// True if the world voxel mapped by `k` lies inside one of the provided chunks.
    pub(crate) fn in_grid(&self, k: &VoxKey) -> bool {
        self.chunks
            .iter()
            .any(|c| c.x == k.cx && c.y == k.cy && c.z == k.cz)
    }

    /// Normalised [0,1] sunlight at a world voxel.
    /// - solid voxel -> 0 (light never enters)
    /// - air voxel OUTSIDE our grid -> real sky -> 1.0
    /// - air voxel INSIDE our grid but never reached by the flood -> trapped/shadowed -> 0
    /// - air voxel inside grid, reached by flood -> its propagated level
    pub(crate) fn sun_at_world(&self, wx: i64, wy: i64, wz: i64) -> f32 {
        let key = VoxKey::from_world(wx, wy, wz);
        if self.is_solid(&key) {
            return 0.0;
        }
        if !self.in_grid(&key) {
            return 1.0; // open sky beyond the streamed set
        }
        let l = self.light.get(&key).copied().unwrap_or(0);
        l as f32 / MAX_LIGHT as f32
    }
}

/// Bake sunlight into a chunk's world-meter mesh.
///
/// `tris` are the world-meter triangles for `chunk` (already transformed by
/// `mesh_chunk_world_meters`). `neighbours` are the adjacent chunks needed so light correctly
/// flows across chunk boundaries (the caller supplies the 6 direct neighbours + the chunk
/// above/below). `y_max` is the world-voxel height up to which sunlight is propagated (cover
/// the tallest terrain). `voxel_scale` and `origin` are the SAME transform `mesh_chunk_world_meters`
/// used (world_m = (origin + local_vox) * voxel_scale), so we can invert a triangle vertex back
/// to its source voxel.
///
/// Triangles keep their geometry/normal/material/ao; only `sun` (per-corner, [0,1]) is set,
/// sampled from the 3 world voxels *behind* each face (on the solid side, like AO).
pub fn bake_sunlight(
    chunk: &Chunk,
    tris: &mut [Triangle],
    neighbours: &[Chunk],
    y_max: i64,
    voxel_scale: f32,
    origin: [f32; 3],
) {
    let mut all: Vec<Chunk> = Vec::with_capacity(neighbours.len() + 1);
    all.push(chunk.clone());
    for n in neighbours {
        all.push(n.clone());
    }
    let mut grid = LightGrid::from_chunks(&all);
    grid.propagate(y_max);

    let to_world_vox = |p: Vec3| -> (i64, i64, i64) {
        // Invert world_m = (origin + local_vox) * voxel_scale  ->  local_vox = world_m/voxel_scale - origin/voxel_scale.
        let lx = ((p.x / voxel_scale) - (origin[0] / voxel_scale)).round() as i64;
        let ly = ((p.y / voxel_scale) - (origin[1] / voxel_scale)).round() as i64;
        let lz = ((p.z / voxel_scale) - (origin[2] / voxel_scale)).round() as i64;
        (lx, ly, lz)
    };

    for t in tris.iter_mut() {
        let n = t.normal;
        // Sample the voxel on the EXPOSED (air) side of the face — the empty space the face
        // looks into — which is where the sky light that illuminates the face lives. (This is
        // the opposite direction from AO, which samples the solid side for occluders.)
        let front = (n.x as i64, n.y as i64, n.z as i64);
        let corners = [t.a, t.b, t.c];
        let mut sun = [0.0f32; 3];
        for (i, c) in corners.iter().enumerate() {
            let (vx, vy, vz) = to_world_vox(*c);
            sun[i] = grid.sun_at_world(vx + front.0, vy + front.1, vz + front.2);
        }
        t.sun = sun;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voxel_core::coords::LocalVoxel;
    use voxel_core::palette::MaterialId;

    /// Build a 32x32xH solid chunk (material 2) from y=0..h, rest AIR, at the coord.
    fn solid_chunk_h(coord: ChunkCoord, h: u8) -> Chunk {
        let mut c = Chunk::uniform(coord, MaterialId::from(0u8));
        for y in 0..h {
            for z in 0..CHUNK_SIZE as u8 {
                for x in 0..CHUNK_SIZE as u8 {
                    c.set(LocalVoxel::new(x, y, z), MaterialId::from(2u8));
                }
            }
        }
        c
    }

    /// All 26 neighbours (3x3x3 minus centre) of (0,0,0) as fully-solid chunks, except the
    /// chunk directly above (0,1,0) which is AIR when `open_top`, else a solid roof. This
    /// fully seals the shell so chunk-edge / diagonal-corner vertices stay in-grid.
    fn full_shell(open_top: bool) -> Vec<Chunk> {
        let mut out = Vec::new();
        for dy in -1..=1i64 {
            for dz in -1..=1i64 {
                for dx in -1..=1i64 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let coord = ChunkCoord::new(dx, dy, dz);
                    if dy == 1 && dx == 0 && dz == 0 {
                        if open_top {
                            out.push(Chunk::uniform(coord, MaterialId::from(0u8)));
                        } else {
                            out.push(solid_chunk_h(coord, CHUNK_SIZE as u8));
                        }
                    } else {
                        out.push(solid_chunk_h(coord, CHUNK_SIZE as u8));
                    }
                }
            }
        }
        out
    }

    // ---- Pure LightGrid (voxel-level) tests: the core of Stap 3 ----

    #[test]
    fn lightgrid_open_column_is_full_sky() {
        // A solid chunk with an open shaft above -> its top air voxel is full sky (1.0).
        let chunk = solid_chunk_h(ChunkCoord::new(0, 0, 0), 4);
        let above = Chunk::uniform(ChunkCoord::new(0, 1, 0), MaterialId::from(0u8)); // AIR
        let mut g = LightGrid::from_chunks(&[chunk, above]);
        g.propagate(1024);
        // Air voxel directly above the ground top (world-y 4) must be full sky.
        let s = g.sun_at_world(0, 4, 0);
        assert!((s - 1.0).abs() < 1e-6, "open air voxel must be full sky (got {s})");
    }

    #[test]
    fn lightgrid_cave_column_is_dark() {
        // Stap 3 acceptance (WORKPLAN_5_STEPS.md): an air pocket trapped under a solid roof
        // (a "cave") must be dark (0.0), while an identical open pocket is lit (1.0).
        // Build a 3-wide column neighbourhood so chunk-edge leaks are impossible.
        let y_max = 1024;

        // Open pocket: ground (0,0,0) with AIR above.
        let open_ground = solid_chunk_h(ChunkCoord::new(0, 0, 0), 4);
        let open_above = Chunk::uniform(ChunkCoord::new(0, 1, 0), MaterialId::from(0u8));
        let mut g_open = LightGrid::from_chunks(&[open_ground, open_above]);
        g_open.propagate(y_max);
        let open_s = g_open.sun_at_world(0, 4, 0);

        // Cave pocket: same ground, but a solid roof sits directly above (0,1,0).
        let cave_ground = solid_chunk_h(ChunkCoord::new(0, 0, 0), 4);
        let roof = solid_chunk_h(ChunkCoord::new(0, 1, 0), CHUNK_SIZE as u8); // full solid roof
        let mut g_cave = LightGrid::from_chunks(&[cave_ground, roof]);
        g_cave.propagate(y_max);
        let cave_s = g_cave.sun_at_world(0, 4, 0);

        assert!(open_s > 0.9, "open pocket must be lit (got {open_s:.3})");
        assert!(cave_s < 0.1, "cave pocket under a roof must be dark (got {cave_s:.3})");
    }

    // ---- Mesh integration: confirm `sun` is actually baked into the triangles ----

    #[test]
    fn mesh_bakes_sun_into_triangles() {
        // The end-to-end pipeline must fill `Triangle.sun` (not leave the 1.0 default for a
        // shadowed surface). Use a sealed shell so the result is deterministic.
        let chunk = solid_chunk_h(ChunkCoord::new(0, 0, 0), CHUNK_SIZE as u8);
        // Cave (sealed shell): top face under the roof must be dark.
        let cave_tris = crate::mesh_chunk_world_meters(
            &chunk,
            crate::chunk_stream::Lod::Full,
            false,
            &full_shell(false),
            1024,
        );
        assert!(!cave_tris.is_empty(), "mesh produced no triangles");
        let cave_top = cave_tris
            .iter()
            .filter(|t| t.normal.y > 0.5)
            .flat_map(|t| t.sun.iter().copied())
            .fold(1.0f32, f32::min);
        assert!(cave_top < 0.5, "sealed cave top must be shadowed (got {cave_top:.3})");

        // Open (shaft): top face must be lit.
        let open_tris = crate::mesh_chunk_world_meters(
            &chunk,
            crate::chunk_stream::Lod::Full,
            false,
            &full_shell(true),
            1024,
        );
        let open_top = open_tris
            .iter()
            .filter(|t| t.normal.y > 0.5)
            .flat_map(|t| t.sun.iter().copied())
            .fold(0.0f32, f32::max);
        assert!(open_top > 0.9, "open shaft top must be lit (got {open_top:.3})");
    }
}