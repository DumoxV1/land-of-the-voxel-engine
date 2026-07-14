//! Material palette: a small palette of material IDs, bitpacked per voxel.

/// A palette material identifier. For S-01 a single byte; bitpacking into a
/// per-chunk palette follows when dense/palette chunk states are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialId(pub u8);

impl From<u8> for MaterialId {
    fn from(v: u8) -> Self {
        MaterialId(v)
    }
}

impl MaterialId {
    pub fn as_u8(self) -> u8 {
        self.0
    }
}
