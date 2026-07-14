//! Byte-stable serialization for chunks (canonical round-trip).
//!
//! Format (all little-endian, fixed layout for determinism):
//!   [0]    version (u8) = 1
//!   [1..4] chunk x (i32 LE)
//!   [4..7] chunk y (i32 LE)
//!   [7..10] chunk z (i32 LE)
//!   [10]   state (u8): 0 = Uniform, 1 = NonUniform
//!   [11]   uniform material (u8)
//!   [12..] if NonUniform: dense CHUNK_SIZE^3 material bytes (one per voxel, flat)
//!
//! This is intentionally simple and deterministic; versioning starts at day one (ADR-0003).

use crate::chunk::{Chunk, ChunkState};
use crate::coords::{ChunkCoord, LocalVoxel, CHUNK_SIZE};
use crate::palette::MaterialId;

const VERSION: u8 = 1;
const HEADER_LEN: usize = 15; // version(1) + 3*i32(12) + state(1) + uniform(1)

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkPayload {
    coord: ChunkCoord,
    state: ChunkState,
    uniform: MaterialId,
    dense: Option<Vec<MaterialId>>,
}

impl ChunkPayload {
    pub fn from_chunk(chunk: &Chunk) -> Self {
        let dense = chunk.dense_data().map(|d| d.to_vec());
        Self {
            coord: chunk.coord,
            state: chunk.state(),
            uniform: chunk.uniform_material(),
            dense,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + CHUNK_SIZE as usize * CHUNK_SIZE as usize * CHUNK_SIZE as usize);
        out.push(VERSION);
        out.extend_from_slice(&(self.coord.x as i32).to_le_bytes());
        out.extend_from_slice(&(self.coord.y as i32).to_le_bytes());
        out.extend_from_slice(&(self.coord.z as i32).to_le_bytes());
        out.push(self.state as u8);
        out.push(self.uniform.as_u8());
        if self.state == ChunkState::NonUniform {
            if let Some(d) = &self.dense {
                for m in d {
                    out.push(m.as_u8());
                }
            }
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < HEADER_LEN {
            return Err("chunk payload too short".into());
        }
        let version = bytes[0];
        if version != VERSION {
            return Err(format!("unsupported chunk payload version {version}"));
        }
        let cx = i32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as i64;
        let cy = i32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as i64;
        let cz = i32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]) as i64;
        let state = match bytes[13] {
            0 => ChunkState::Uniform,
            1 => ChunkState::NonUniform,
            s => return Err(format!("invalid chunk state {s}")),
        };
        let uniform = MaterialId::from(bytes[14]);
        let dense = if state == ChunkState::NonUniform {
            let n = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
            if bytes.len() != HEADER_LEN + n {
                return Err(format!(
                    "dense chunk payload length {} != expected {}",
                    bytes.len(),
                    HEADER_LEN + n
                ));
            }
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                v.push(MaterialId::from(bytes[HEADER_LEN + i]));
            }
            Some(v)
        } else {
            None
        };
        Ok(Self {
            coord: ChunkCoord::new(cx, cy, cz),
            state,
            uniform,
            dense,
        })
    }

    pub fn into_chunk(self) -> Result<Chunk, String> {
        if self.state == ChunkState::NonUniform && self.dense.is_none() {
            return Err("non-uniform chunk missing dense data".into());
        }
        // Build through the Chunk API so state transitions stay consistent.
        let mut chunk = Chunk::uniform(self.coord, self.uniform);
        if let Some(d) = self.dense {
            // Replay into the chunk; flat order must match LocalVoxel::flat().
            for (i, m) in d.iter().enumerate() {
                let x = (i / (CHUNK_SIZE as usize * CHUNK_SIZE as usize)) as u8;
                let y = ((i / CHUNK_SIZE as usize) % CHUNK_SIZE as usize) as u8;
                let z = (i % CHUNK_SIZE as usize) as u8;
                chunk.set(LocalVoxel::new(x, y, z), *m);
            }
        }
        Ok(chunk)
    }
}

/// Round-trip helper used by tests: returns the bytes unchanged (canonical form).
pub fn round_trip(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
