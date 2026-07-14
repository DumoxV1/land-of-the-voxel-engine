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
