//! Edit events with idempotence and revision ordering.
//!
//! Per ADR-0003 (and the canonical plan §3.2 / §3.5), every edit carries the
//! world position, old/new material, and a monotone edit id (revision). Two
//! edits at the same position with the same id are idempotent (a retransmit);
//! two edits at the same position with different new values are a conflict.

use crate::coords::WorldVoxel;
use crate::palette::MaterialId;

/// Monotone revision / edit identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditId(pub u64);

impl EditId {
    pub fn new(id: u64) -> Self {
        EditId(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edit {
    pub pos: WorldVoxel,
    pub old: MaterialId,
    pub new: MaterialId,
    pub id: EditId,
}

impl Edit {
    pub fn new(pos: WorldVoxel, old: MaterialId, new: MaterialId, id: EditId) -> Self {
        Self { pos, old, new, id }
    }

    /// Returns `None` if the two edits are idempotent (same position + id),
    /// or `Some(other)` if they conflict (same position, different new value or id).
    pub fn conflicts_with(&self, other: &Edit) -> Option<Edit> {
        if self.pos != other.pos {
            return None;
        }
        if self.id == other.id && self.new == other.new {
            None // identical retransmit — idempotent
        } else {
            Some(*other)
        }
    }
}
