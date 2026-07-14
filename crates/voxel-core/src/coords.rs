//! Integer world coordinates with Euclidean division (negative-safe).

use std::ops::Div;

/// Side length of a cubic chunk, in voxels. Fixed for S-01; benchmarks may revisit.
pub const CHUNK_SIZE: i64 = 32;

/// A voxel position in the infinite integer world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldVoxel {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl WorldVoxel {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }
}

/// The coordinate of a chunk (a CHUNK_SIZE³ block of voxels), in chunk-space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl ChunkCoord {
    pub fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    /// Map a world voxel to its owning chunk using Euclidean division so that
    /// negative world coordinates land in the correct (negative) chunk.
    pub fn from_world(w: WorldVoxel) -> Self {
        Self {
            x: euclidean_div(w.x, CHUNK_SIZE),
            y: euclidean_div(w.y, CHUNK_SIZE),
            z: euclidean_div(w.z, CHUNK_SIZE),
        }
    }

    /// Reconstruct the world voxel from this chunk coordinate and a local index.
    pub fn world_voxel(&self, local: LocalVoxel) -> WorldVoxel {
        WorldVoxel::new(
            self.x * CHUNK_SIZE + local.x as i64,
            self.y * CHUNK_SIZE + local.y as i64,
            self.z * CHUNK_SIZE + local.z as i64,
        )
    }
}

/// A voxel index within a chunk, always in `[0, CHUNK_SIZE)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalVoxel {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl LocalVoxel {
    pub fn new(x: u8, y: u8, z: u8) -> Self {
        Self { x, y, z }
    }

    /// Map a world voxel to its local index using Euclidean remainder.
    pub fn from_world(w: WorldVoxel) -> Self {
        Self {
            x: euclidean_rem(w.x, CHUNK_SIZE) as u8,
            y: euclidean_rem(w.y, CHUNK_SIZE) as u8,
            z: euclidean_rem(w.z, CHUNK_SIZE) as u8,
        }
    }

    /// Flat index into a dense `CHUNK_SIZE³` array.
    pub fn flat(&self) -> usize {
        (self.x as usize) * (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize)
            + (self.y as usize) * (CHUNK_SIZE as usize)
            + (self.z as usize)
    }
}

/// Euclidean division: floor division that rounds toward negative infinity.
/// Unlike Rust's default `a / b` (truncating toward zero), this keeps the
/// remainder in `[0, |b|)` for negative operands.
fn euclidean_div(a: i64, b: i64) -> i64 {
    let q = a.div(b);
    let r = a % b;
    if (r < 0 && b > 0) || (r > 0 && b < 0) {
        q - 1
    } else {
        q
    }
}

/// Euclidean remainder: always in `[0, |b|)`.
fn euclidean_rem(a: i64, b: i64) -> i64 {
    let r = a % b;
    if r < 0 {
        if b < 0 {
            r - b
        } else {
            r + b
        }
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euclidean_div_boundary() {
        assert_eq!(euclidean_div(-CHUNK_SIZE, CHUNK_SIZE), -1);
        assert_eq!(euclidean_rem(-CHUNK_SIZE, CHUNK_SIZE), 0);
        assert_eq!(euclidean_div(-1, CHUNK_SIZE), -1);
        assert_eq!(euclidean_rem(-1, CHUNK_SIZE), CHUNK_SIZE - 1);
    }
}
