//! voxel-world: multi-chunk world store (S-05 spike).
//!
//! Holds many `voxel_core::Chunk`s keyed by `ChunkCoord`, generating them on demand from a
//! seed via `voxel-worldgen` and caching edits. Renderer-agnostic: depends only on
//! `voxel-core` + `voxel-worldgen`.

use std::collections::{HashMap, HashSet};

use voxel_core::chunk::Chunk;
use voxel_core::coords::{ChunkCoord, LocalVoxel, WorldVoxel};
use voxel_core::palette::MaterialId;
use voxel_worldgen::generate_chunk;

/// A multi-chunk world: caches generated/base chunks and any player edits made to them.
///
/// Chunks are generated lazily from a fixed seed (so the base world is reproducible) and
/// edits persist in the cache — re-fetching a coord never regenerates over an edited chunk.
pub struct World {
    chunks: HashMap<ChunkCoord, Chunk>,
    dirty: HashSet<ChunkCoord>,
    seed: u32,
}

impl World {
    /// Create an empty world with the given base-generation seed.
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            seed,
        }
    }

    /// Get a chunk, generating and caching it from the seed if absent. Idempotent: an already
    /// cached chunk (including edits) is returned unchanged. Returns an owned copy so callers
    /// can hold several chunks without borrow conflicts.
    pub fn get_or_generate(&mut self, coord: ChunkCoord) -> Chunk {
        self.chunks
            .entry(coord)
            .or_insert_with(|| generate_chunk(coord, self.seed));
        self.chunks.get(&coord).unwrap().clone()
    }

    /// Get a chunk, generating if needed, as an owned copy.
    pub fn chunk_at(&mut self, coord: ChunkCoord) -> Chunk {
        self.get_or_generate(coord)
    }

    /// Write a voxel at a world position into the owning chunk, marking that chunk dirty.
    pub fn set_voxel(&mut self, world: WorldVoxel, material: MaterialId) {
        let coord = ChunkCoord::from_world(world);
        let local = LocalVoxel::from_world(world);
        let chunk = self
            .chunks
            .entry(coord)
            .or_insert_with(|| generate_chunk(coord, self.seed));
        chunk.set(local, material);
        self.dirty.insert(coord);
    }

    /// Coordinates of chunks marked dirty since the last `take_dirty`.
    pub fn dirty_chunks(&self) -> HashSet<ChunkCoord> {
        self.dirty.clone()
    }

    /// Take and clear the dirty set (caller is responsible for re-meshing / persisting).
    pub fn take_dirty(&mut self) -> HashSet<ChunkCoord> {
        std::mem::take(&mut self.dirty)
    }
}
