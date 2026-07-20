//! voxel-edit: first-class voxel edit events + an edit tool (S-06 spike).
//!
//! Models a voxel edit as a replayable `Edit` (world position, old/new material, actor, tick,
//! monotonic revision) and an `EditTool` that applies safe place/remove edits to a `World`
//! while appending to an append-only `EditLog`. Used by persistence (S-07) and multiplayer
//! (S-09). Renderer-agnostic: depends only on `voxel-core` + `voxel-world`.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_world::World;

/// A single voxel edit, recorded as a first-class event for replay / persistence / networking.
///
/// `old` is the world value before the edit (enables undo and integrity checks); `new` is the
/// resulting value (`0` = removed/air). `revision` is a monotonic counter for ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    pub world: WorldVoxel,
    pub old: MaterialId,
    pub new: MaterialId,
    pub actor: u32,
    pub tick: u64,
    pub revision: u64,
}

/// Append-only log of edits with a monotonic revision counter.
#[derive(Debug, Clone, Default)]
pub struct EditLog {
    edits: Vec<Edit>,
    next_revision: u64,
}

impl EditLog {
    /// Create an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an edit, assigning the next monotonic revision. Returns the stored edit.
    pub fn push(&mut self, mut edit: Edit) -> Edit {
        edit.revision = self.next_revision;
        self.next_revision += 1;
        self.edits.push(edit);
        edit
    }

    /// Number of logged edits.
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// The latest revision (largest assigned), or 0 if empty.
    pub fn revision(&self) -> u64 {
        self.next_revision.saturating_sub(1)
    }

    /// Borrow the recorded edits.
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// Replay every edit onto a world (idempotent w.r.t. the log contents: each edit sets the
    /// recorded `new` value, so the resulting world equals the one the edits were made on).
    pub fn apply_all(&self, world: &mut World) {
        for e in &self.edits {
            world.set_voxel(e.world, e.new);
        }
    }
}

/// Applies place/remove edits to a `World` and records them in an `EditLog`.
pub struct EditTool {
    log: EditLog,
}

impl EditTool {
    /// Create a tool with an empty log.
    pub fn new() -> Self {
        Self { log: EditLog::new() }
    }

    /// Place `material` at a world position. Records the prior value as `old`.
    pub fn place(
        &mut self,
        world: &mut World,
        world_pos: WorldVoxel,
        material: MaterialId,
        actor: u32,
        tick: u64,
    ) -> Edit {
        let old = current(world, world_pos);
        world.set_voxel(world_pos, material);
        self.log.push(Edit {
            world: world_pos,
            old,
            new: material,
            actor,
            tick,
            revision: 0, // assigned by the log
        })
    }

    /// Remove the voxel at a world position (set to air, material 0).
    pub fn remove(
        &mut self,
        world: &mut World,
        world_pos: WorldVoxel,
        actor: u32,
        tick: u64,
    ) -> Edit {
        self.place(world, world_pos, MaterialId::from(0), actor, tick)
    }

    /// Consume the tool, returning its edit log.
    pub fn into_log(self) -> EditLog {
        self.log
    }

    /// Borrow the current log.
    pub fn log(&self) -> &EditLog {
        &self.log
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the current material at a world position (generating the chunk if needed).
fn current(world: &mut World, world_pos: WorldVoxel) -> MaterialId {
    world
        .get_or_generate(voxel_core::coords::ChunkCoord::from_world(world_pos))
        .get(voxel_core::coords::LocalVoxel::from_world(world_pos))
}

/// I1 (live edit): voxel ray-cast (Amanatides & Woo DDA) through a `World`.
///
/// March from `origin` (a world voxel) along **unit** `dir` for at most `max_dist` voxels,
/// returning the first solid (non-air) voxel hit and the face normal (the empty neighbour the
/// ray entered through). `None` if nothing solid is hit within range.
///
/// Pure w.r.t. the world (only reads `material_at`). Works in voxel units; the client converts
/// the camera eye (meters) to voxel units via `VOXEL_SIZE_M`.
pub fn raycast_voxel(
    world: &mut World,
    origin: WorldVoxel,
    dir: [f32; 3],
    max_dist: f32,
) -> Option<(WorldVoxel, WorldVoxel)> {
    // Normalize direction.
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
    if len < 1e-8 {
        return None;
    }
    let d = [dir[0] / len, dir[1] / len, dir[2] / len];

    let mut pos = [origin.x as f32, origin.y as f32, origin.z as f32];
    // Current voxel (floor of pos).
    let mut voxel = [
        pos[0].floor() as i64,
        pos[1].floor() as i64,
        pos[2].floor() as i64,
    ];

    // Step direction per axis (+1/-1 or 0).
    let step = [
        if d[0] > 0.0 { 1 } else if d[0] < 0.0 { -1 } else { 0 },
        if d[1] > 0.0 { 1 } else if d[1] < 0.0 { -1 } else { 0 },
        if d[2] > 0.0 { 1 } else if d[2] < 0.0 { -1 } else { 0 },
    ];

    // T-max: distance to the next voxel boundary on each axis. T-delta: distance between
    // boundaries along the ray. Guard against div-by-zero for axis-aligned rays.
    let mut t_max = [f32::INFINITY; 3];
    let mut t_delta = [f32::INFINITY; 3];
    for a in 0..3 {
        if step[a] != 0 {
            let next_boundary = if step[a] > 0 {
                voxel[a] as f32 + 1.0
            } else {
                voxel[a] as f32
            };
            t_max[a] = (next_boundary - pos[a]) / d[a];
            t_delta[a] = (1.0 / d[a]).abs();
        }
    }

    // Face the ray entered through (the empty neighbour normal). Start with +X/+Y/+Z by step.
    let mut normal = WorldVoxel::new(0, 0, 0);

    // Already inside a solid voxel?
    let start = WorldVoxel::new(voxel[0], voxel[1], voxel[2]);
    if world.material_at(start).0 != 0 {
        return Some((start, WorldVoxel::new(0, 0, 0)));
    }

    let max_steps = (max_dist.ceil() as i64).max(1);
    for _ in 0..max_steps {
        // Advance to the nearest voxel boundary.
        let a = if t_max[0] <= t_max[1] && t_max[0] <= t_max[2] {
            0
        } else if t_max[1] <= t_max[2] {
            1
        } else {
            2
        };
        if t_max[a] > max_dist {
            break;
        }
        voxel[a] += step[a];
        // Normal of the face we just crossed points back toward where we came from.
        let mut n = [0i64, 0, 0];
        n[a] = -step[a];
        normal = WorldVoxel::new(n[0], n[1], n[2]);
        t_max[a] += t_delta[a];

        let v = WorldVoxel::new(voxel[0], voxel[1], voxel[2]);
        if world.material_at(v).0 != 0 {
            return Some((v, normal));
        }
    }
    None
}
