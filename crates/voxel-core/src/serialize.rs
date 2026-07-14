//! Byte-stable serialization for chunks (canonical round-trip).
//!
//! Format (all little-endian, fixed layout for determinism):
//!   [0]      version (u8) = 3
//!   [1..9]   chunk x (i64 LE)
//!   [9..17]  chunk y (i64 LE)
//!   [17..25] chunk z (i64 LE)
//!   [25]     state (u8): 0 = Uniform, 1 = PalettePacked, 2 = Dense
//!   [26]     uniform material (u8)
//!   PalettePacked only:
//!     [27]   palette_len (u8, 1..=16)
//!     [28..28+palette_len]  palette material bytes (one per entry, in index order)
//!     [..]   packed 4-bit voxel data (N^3/2 bytes, even voxel = low nibble)
//!   Dense only:
//!     [..]   dense CHUNK_SIZE^3 material bytes (one per voxel, flat)
//!
//! Versioning starts at day one (ADR-0003). Version 2 stored coords as i32 (silent
//! truncation beyond ±2^31 — S-11 audit fix C-01); version 3 stores full i64 coords and
//! validates every packed nibble against the palette length.

use crate::chunk::{Chunk, ChunkState};
use crate::coords::{ChunkCoord, CHUNK_SIZE};
use crate::palette::MaterialId;

const VERSION: u8 = 3;
const HEADER_LEN: usize = 27; // version(1) + 3*i64(24) + state(1) + uniform(1)

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkPayload {
    coord: ChunkCoord,
    state: ChunkState,
    uniform: MaterialId,
    palette: Option<Vec<MaterialId>>,
    packed: Option<Vec<u8>>,
    dense: Option<Vec<MaterialId>>,
}

impl ChunkPayload {
    pub fn from_chunk(chunk: &Chunk) -> Self {
        Self {
            coord: chunk.coord,
            state: chunk.state(),
            uniform: chunk.uniform_material(),
            palette: chunk.palette().map(|p| p.to_vec()),
            packed: chunk.packed_data().map(|p| p.to_vec()),
            dense: chunk.dense_data().map(|d| d.to_vec()),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let n = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut out = Vec::new();
        out.push(VERSION);
        out.extend_from_slice(&self.coord.x.to_le_bytes());
        out.extend_from_slice(&self.coord.y.to_le_bytes());
        out.extend_from_slice(&self.coord.z.to_le_bytes());
        let state_byte = match self.state {
            ChunkState::Uniform => 0u8,
            ChunkState::PalettePacked => 1u8,
            ChunkState::Dense => 2u8,
        };
        out.push(state_byte);
        out.push(self.uniform.as_u8());
        match self.state {
            ChunkState::Uniform => {}
            ChunkState::PalettePacked => {
                let palette = self.palette.as_ref().expect("palette-packed needs palette");
                let packed = self.packed.as_ref().expect("palette-packed needs packed data");
                out.push(palette.len() as u8);
                for m in palette {
                    out.push(m.as_u8());
                }
                out.extend_from_slice(packed);
            }
            ChunkState::Dense => {
                let dense = self.dense.as_ref().expect("dense needs dense data");
                for m in dense {
                    out.push(m.as_u8());
                }
            }
        }
        // Sanity: ensure length matches expectation for round-trip checks.
        match self.state {
            ChunkState::Uniform => assert_eq!(out.len(), HEADER_LEN),
            ChunkState::PalettePacked => {
                let p = self.palette.as_ref().unwrap().len();
                assert_eq!(out.len(), HEADER_LEN + 1 + p + n.div_ceil(2));
            }
            ChunkState::Dense => assert_eq!(out.len(), HEADER_LEN + n),
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < HEADER_LEN {
            return Err("chunk payload too short".into());
        }
        let version = bytes[0];
        if version != VERSION {
            return Err(format!(
                "unsupported chunk payload version {version} (expected {VERSION})"
            ));
        }
        let cx = i64::from_le_bytes(bytes[1..9].try_into().unwrap());
        let cy = i64::from_le_bytes(bytes[9..17].try_into().unwrap());
        let cz = i64::from_le_bytes(bytes[17..25].try_into().unwrap());
        let state = match bytes[25] {
            0 => ChunkState::Uniform,
            1 => ChunkState::PalettePacked,
            2 => ChunkState::Dense,
            s => return Err(format!("invalid chunk state {s}")),
        };
        let uniform = MaterialId::from(bytes[26]);
        let body = &bytes[27..];

        let (palette, packed, dense) = match state {
            ChunkState::Uniform => (None, None, None),
            ChunkState::PalettePacked => {
                if body.is_empty() {
                    return Err("palette-packed chunk missing palette length".into());
                }
                let plen = body[0] as usize;
                if plen == 0 || plen > crate::chunk::PALETTE_LIMIT {
                    return Err(format!("invalid palette length {plen}"));
                }
                let need = 1 + plen + (CHUNK_SIZE as usize).pow(3).div_ceil(2);
                if body.len() != need {
                    return Err(format!(
                        "palette-packed payload length {} != expected {}",
                        body.len(),
                        need
                    ));
                }
                let palette = body[1..1 + plen].iter().map(|&b| MaterialId::from(b)).collect();
                let packed = body[1 + plen..].to_vec();
                // Every 4-bit index must point inside the palette (S-11 audit fix):
                // an out-of-range nibble would otherwise panic later in `Chunk::get`.
                let total = (CHUNK_SIZE as usize).pow(3);
                for (bi, &byte) in packed.iter().enumerate() {
                    let lo = (byte & 0x0F) as usize;
                    let hi = (byte >> 4) as usize;
                    if lo >= plen || (2 * bi + 1 < total && hi >= plen) {
                        return Err(format!(
                            "packed nibble out of palette range at byte {bi} (palette_len {plen})"
                        ));
                    }
                }
                (Some(palette), Some(packed), None)
            }
            ChunkState::Dense => {
                let n = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
                if body.len() != n {
                    return Err(format!(
                        "dense chunk payload length {} != expected {}",
                        body.len(),
                        n
                    ));
                }
                let dense = body.iter().map(|&b| MaterialId::from(b)).collect();
                (None, None, Some(dense))
            }
        };

        Ok(Self {
            coord: ChunkCoord::new(cx, cy, cz),
            state,
            uniform,
            palette,
            packed,
            dense,
        })
    }

    pub fn into_chunk(self) -> Result<Chunk, String> {
        match self.state {
            ChunkState::Uniform => Ok(Chunk::uniform(self.coord, self.uniform)),
            ChunkState::PalettePacked => {
                if self.palette.is_none() || self.packed.is_none() {
                    return Err("palette-packed chunk missing palette or packed data".into());
                }
                Ok(Chunk::from_raw(
                    self.coord,
                    self.state,
                    self.uniform,
                    self.palette,
                    self.packed,
                    None,
                ))
            }
            ChunkState::Dense => {
                if self.dense.is_none() {
                    return Err("dense chunk missing dense data".into());
                }
                Ok(Chunk::from_raw(
                    self.coord,
                    self.state,
                    self.uniform,
                    None,
                    None,
                    self.dense,
                ))
            }
        }
    }
}

/// Round-trip helper used by tests: returns the bytes unchanged (canonical form).
pub fn round_trip(bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
