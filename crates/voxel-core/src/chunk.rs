//! Chunk storage with the three ADR-0001 states:
//! - `Uniform`: single shared material, zero storage.
//! - `PalettePacked`: dense voxel grid with <= 16 distinct materials, bitpacked 4-bit IDs
//!   (2 voxels per byte) backed by a per-chunk palette (max 16 entries).
//! - `Dense`: dense voxel grid with > 16 distinct materials (1 byte per voxel).
//!
//! Sparse/palette-packed states were deferred in the original S-01 and added in S-01-hardening.

use crate::coords::{ChunkCoord, LocalVoxel, CHUNK_SIZE};
use crate::palette::MaterialId;
use crate::serialize::ChunkPayload;

/// Maximum number of palette entries in a palette-packed chunk (4-bit material index).
pub const PALETTE_LIMIT: usize = 16;

/// State of a chunk, used for storage/serialization strategy (ADR-0001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkState {
    /// Every voxel shares one material. Zero storage.
    Uniform,
    /// Dense voxel grid with <= 16 distinct materials, bitpacked 4-bit IDs (2 voxels/byte)
    /// backed by a per-chunk palette (max 16 entries).
    PalettePacked,
    /// Dense voxel grid with > 16 distinct materials (1 byte per voxel).
    Dense,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    pub coord: ChunkCoord,
    state: ChunkState,
    uniform: MaterialId,
    // Per-chunk palette (only for PalettePacked). Maps a 4-bit material index to a MaterialId.
    palette: Option<Vec<MaterialId>>,
    // 4-bit packed voxel data (only for PalettePacked): 2 voxels per byte.
    packed: Option<Vec<u8>>,
    // Dense storage, 1 byte per voxel (only for Dense).
    dense: Option<Vec<MaterialId>>,
}

impl Chunk {
    pub fn uniform(coord: ChunkCoord, material: MaterialId) -> Self {
        Self {
            coord,
            state: ChunkState::Uniform,
            uniform: material,
            palette: None,
            packed: None,
            dense: None,
        }
    }

    /// Build a chunk from already-decoded raw parts. Used by serialization; validates
    /// nothing here (the payload layer validates). `pub(crate)` so the serialize module
    /// can reconstruct the exact palette order for byte-stable round-trips.
    pub(crate) fn from_raw(
        coord: ChunkCoord,
        state: ChunkState,
        uniform: MaterialId,
        palette: Option<Vec<MaterialId>>,
        packed: Option<Vec<u8>>,
        dense: Option<Vec<MaterialId>>,
    ) -> Self {
        Self {
            coord,
            state,
            uniform,
            palette,
            packed,
            dense,
        }
    }

    pub fn state(&self) -> ChunkState {
        self.state
    }

    /// True when every voxel in the chunk is AIR (material id 0).
    ///
    /// O(1) for the common `Uniform` case (a freshly generated above-surface / below-bedrock
    /// chunk stays uniform-AIR), so callers can cheaply skip meshing an empty chunk instead
    /// of running the full greedy sweep (~196k neighbour probes) only to emit zero triangles.
    /// A palette-packed or dense chunk only exists after a non-AIR `set`, so in practice it is
    /// never empty; the exhaustive scan below keeps the predicate correct regardless.
    pub fn is_empty(&self) -> bool {
        match self.state {
            ChunkState::Uniform => self.uniform == MaterialId::from(0u8),
            ChunkState::PalettePacked => self
                .palette
                .as_ref()
                .map(|p| p.iter().all(|m| *m == MaterialId::from(0u8)))
                .unwrap_or(true),
            ChunkState::Dense => self
                .dense
                .as_ref()
                .map(|d| d.iter().all(|m| *m == MaterialId::from(0u8)))
                .unwrap_or(true),
        }
    }

    pub fn get(&self, local: LocalVoxel) -> MaterialId {
        match self.state {
            ChunkState::Uniform => self.uniform,
            ChunkState::PalettePacked => {
                let f = local.flat();
                let byte = self.packed.as_ref().unwrap()[f / 2];
                let idx = if f.is_multiple_of(2) { byte & 0x0F } else { (byte & 0xF0) >> 4 };
                self.palette.as_ref().unwrap()[idx as usize]
            }
            ChunkState::Dense => self.dense.as_ref().unwrap()[local.flat()],
        }
    }

    pub fn set(&mut self, local: LocalVoxel, material: MaterialId) {
        if self.state == ChunkState::Uniform {
            if material == self.uniform {
                return;
            }
            self.transition_uniform_to_palette(local, material);
            return;
        }
        let f = local.flat();
        match self.state {
            ChunkState::PalettePacked => self.set_palette(local, f, material),
            ChunkState::Dense => {
                self.dense.as_mut().unwrap()[f] = material;
            }
            ChunkState::Uniform => unreachable!(),
        }
    }

    /// Material used when the chunk is still uniform (before dense allocation).
    pub fn uniform_material(&self) -> MaterialId {
        self.uniform
    }

    /// Borrow the per-chunk palette if the chunk is palette-packed.
    pub fn palette(&self) -> Option<&[MaterialId]> {
        self.palette.as_deref()
    }

    /// Borrow the 4-bit packed voxel data if the chunk is palette-packed.
    pub fn packed_data(&self) -> Option<&[u8]> {
        self.packed.as_deref()
    }

    /// Borrow the dense data if the chunk has diverged past the palette limit.
    pub fn dense_data(&self) -> Option<&[MaterialId]> {
        self.dense.as_deref()
    }

    /// Serialize to a canonical, byte-stable representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        ChunkPayload::from_chunk(self).to_bytes()
    }

    /// Deserialize from canonical bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        ChunkPayload::from_bytes(bytes).and_then(ChunkPayload::into_chunk)
    }

    fn voxel_count() -> usize {
        (CHUNK_SIZE as usize).pow(3)
    }

    fn transition_uniform_to_palette(&mut self, local: LocalVoxel, material: MaterialId) {
        let n = Self::voxel_count();
        let mut palette = vec![self.uniform];
        let mut packed = vec![0u8; n.div_ceil(2)];
        palette.push(material);
        let new_idx = (palette.len() - 1) as u8; // = 1
        let f = local.flat();
        write_nibble(&mut packed, f, new_idx);
        self.state = ChunkState::PalettePacked;
        self.palette = Some(palette);
        self.packed = Some(packed);
    }

    fn set_palette(&mut self, _local: LocalVoxel, f: usize, material: MaterialId) {
        let palette = self.palette.as_mut().unwrap();
        let idx = match palette.iter().position(|&m| m == material) {
            Some(i) => i as u8,
            None => {
                if palette.len() < PALETTE_LIMIT {
                    palette.push(material);
                    (palette.len() - 1) as u8
                } else {
                    // 17th distinct material: promote to dense (1 byte/voxel).
                    self.promote_to_dense();
                    self.dense.as_mut().unwrap()[f] = material;
                    return;
                }
            }
        };
        write_nibble(self.packed.as_mut().unwrap(), f, idx);
    }

    fn promote_to_dense(&mut self) {
        let n = Self::voxel_count();
        let palette = self.palette.as_ref().unwrap();
        let packed = self.packed.as_ref().unwrap();
        let mut dense = Vec::with_capacity(n);
        for f in 0..n {
            let byte = packed[f / 2];
            let idx = if f % 2 == 0 { byte & 0x0F } else { (byte & 0xF0) >> 4 };
            dense.push(palette[idx as usize]);
        }
        self.state = ChunkState::Dense;
        self.palette = None;
        self.packed = None;
        self.dense = Some(dense);
    }
}

#[cfg(test)]
mod is_empty_tests {
    use super::*;

    #[test]
    fn uniform_air_chunk_is_empty() {
        let c = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
        assert!(c.is_empty(), "uniform-AIR chunk must report empty");
    }

    #[test]
    fn uniform_solid_chunk_is_not_empty() {
        let c = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(3u8));
        assert!(!c.is_empty(), "uniform-STONE chunk must not report empty");
    }

    #[test]
    fn chunk_with_one_solid_voxel_is_not_empty() {
        let mut c = Chunk::uniform(ChunkCoord::new(0, 0, 0), MaterialId::from(0u8));
        c.set(LocalVoxel::new(1, 2, 3), MaterialId::from(2u8));
        assert!(
            !c.is_empty(),
            "chunk with a single solid voxel must not report empty"
        );
    }
}

/// Write a 4-bit material index `idx` into the packed array at flat voxel `f`.
/// Even `f` -> low nibble, odd `f` -> high nibble.
fn write_nibble(packed: &mut [u8], f: usize, idx: u8) {
    let byte_idx = f / 2;
    let byte = &mut packed[byte_idx];
    if f.is_multiple_of(2) {
        *byte = (*byte & 0xF0) | (idx & 0x0F);
    } else {
        *byte = (*byte & 0x0F) | ((idx & 0x0F) << 4);
    }
}
