//! Chunk storage. S-01 starts with a `Uniform` (single material) chunk and a
//! `NonUniform` dense array once any voxel is set. Sparse/palette-packed states
//! follow in a later S-01 iteration.

use crate::coords::{ChunkCoord, LocalVoxel, CHUNK_SIZE};
use crate::palette::MaterialId;
use crate::serialize::ChunkPayload;

/// State of a chunk, used for storage/serialization strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    /// Every voxel shares one material.
    Uniform,
    /// At least one voxel differs from the uniform baseline.
    NonUniform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub coord: ChunkCoord,
    state: ChunkState,
    uniform: MaterialId,
    // Dense storage, only allocated lazily when the chunk becomes non-uniform.
    data: Option<Vec<MaterialId>>,
}

impl Chunk {
    pub fn uniform(coord: ChunkCoord, material: MaterialId) -> Self {
        Self {
            coord,
            state: ChunkState::Uniform,
            uniform: material,
            data: None,
        }
    }

    pub fn state(&self) -> ChunkState {
        self.state
    }

    pub fn get(&self, local: LocalVoxel) -> MaterialId {
        match &self.data {
            Some(d) => d[local.flat()],
            None => self.uniform,
        }
    }

    pub fn set(&mut self, local: LocalVoxel, material: MaterialId) {
        if self.state == ChunkState::Uniform && material == self.uniform {
            return;
        }
        if self.data.is_none() {
            let n = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
            self.data = Some(vec![self.uniform; n]);
        }
        let data = self.data.as_mut().unwrap();
        data[local.flat()] = material;
        self.state = ChunkState::NonUniform;
    }

    /// Material used when the chunk is still uniform (before dense allocation).
    pub fn uniform_material(&self) -> MaterialId {
        self.uniform
    }

    /// Borrow the dense data if the chunk has diverged from uniform.
    pub fn dense_data(&self) -> Option<&[MaterialId]> {
        self.data.as_deref()
    }

    /// Serialize to a canonical, byte-stable representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        ChunkPayload::from_chunk(self).to_bytes()
    }

    /// Deserialize from canonical bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        ChunkPayload::from_bytes(bytes).and_then(ChunkPayload::into_chunk)
    }
}
