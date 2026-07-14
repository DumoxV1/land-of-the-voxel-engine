//! voxel-persist: save/load a world as (seed + append-only edit log) (S-07 spike).
//!
//! The world base is reproducible from the seed, so only the seed + edits are stored — the
//! "procedural base + append-only edit log" approach from the canonical plan. Renderer-agnostic:
//! depends only on `voxel-core` + `voxel-world` + `voxel-edit`.

use std::io::{Read, Write};
use std::path::Path;

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_edit::{Edit, EditLog};
use voxel_world::World;

/// Magic bytes identifying a voxel-persist save file (version 1).
const MAGIC: [u8; 4] = *b"VWL1";

/// Errors that can occur while loading a save file.
#[derive(Debug)]
pub enum PersistError {
    /// IO failure (missing file, permission, etc.).
    Io(std::io::Error),
    /// File does not start with the expected magic bytes.
    BadMagic,
    /// File is truncated or otherwise malformed (could not decode a complete record).
    Truncated,
}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

/// Save a world's base seed + its edit log to `path`.
///
/// Only the seed and edits are written; the base terrain is regenerated from the seed on load.
pub fn save_world(world: &World, log: &EditLog, path: &Path) -> Result<(), PersistError> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&MAGIC);
    buf.extend_from_slice(&world.seed().to_le_bytes());
    buf.extend_from_slice(&(log.len() as u32).to_le_bytes());
    for e in log.edits() {
        buf.extend_from_slice(&e.world.x.to_le_bytes());
        buf.extend_from_slice(&e.world.y.to_le_bytes());
        buf.extend_from_slice(&e.world.z.to_le_bytes());
        buf.push(e.old.0);
        buf.push(e.new.0);
        buf.extend_from_slice(&e.actor.to_le_bytes());
        buf.extend_from_slice(&e.tick.to_le_bytes());
        buf.extend_from_slice(&e.revision.to_le_bytes());
    }
    // Atomic write (S-11 audit fix): write to a temp file, sync, then rename over the
    // target so a crash mid-write can never corrupt an existing save.
    let tmp = path.with_extension("tmp");
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(&buf)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Load a world + edit log from `path`, reconstructing the world by replaying the edits onto a
/// freshly seeded `World`.
pub fn load_world(path: &Path) -> Result<(World, EditLog), PersistError> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.read_to_end(&mut bytes)?;

    if bytes.len() < 4 || bytes[0..4] != MAGIC {
        return Err(PersistError::BadMagic);
    }
    let mut cursor = 4usize;

    let seed = read_u32(&bytes, &mut cursor)?;
    let count = read_u32(&bytes, &mut cursor)? as usize;

    let mut log = EditLog::new();
    for _ in 0..count {
        let x = read_i64(&bytes, &mut cursor)?;
        let y = read_i64(&bytes, &mut cursor)?;
        let z = read_i64(&bytes, &mut cursor)?;
        let old = read_u8(&bytes, &mut cursor)?;
        let new = read_u8(&bytes, &mut cursor)?;
        let actor = read_u32(&bytes, &mut cursor)?;
        let tick = read_u64(&bytes, &mut cursor)?;
        let revision = read_u64(&bytes, &mut cursor)?;
        log.push(Edit {
            world: WorldVoxel::new(x, y, z),
            old: MaterialId::from(old),
            new: MaterialId::from(new),
            actor,
            tick,
            revision,
        });
    }

    let mut world = World::new(seed);
    log.apply_all(&mut world);
    Ok((world, log))
}

fn read_u32(b: &[u8], c: &mut usize) -> Result<u32, PersistError> {
    if *c + 4 > b.len() {
        return Err(PersistError::Truncated);
    }
    let v = u32::from_le_bytes([b[*c], b[*c + 1], b[*c + 2], b[*c + 3]]);
    *c += 4;
    Ok(v)
}

fn read_i64(b: &[u8], c: &mut usize) -> Result<i64, PersistError> {
    if *c + 8 > b.len() {
        return Err(PersistError::Truncated);
    }
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[*c..*c + 8]);
    *c += 8;
    Ok(i64::from_le_bytes(a))
}

fn read_u8(b: &[u8], c: &mut usize) -> Result<u8, PersistError> {
    if *c + 1 > b.len() {
        return Err(PersistError::Truncated);
    }
    let v = b[*c];
    *c += 1;
    Ok(v)
}

fn read_u64(b: &[u8], c: &mut usize) -> Result<u64, PersistError> {
    if *c + 8 > b.len() {
        return Err(PersistError::Truncated);
    }
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[*c..*c + 8]);
    *c += 8;
    Ok(u64::from_le_bytes(a))
}
