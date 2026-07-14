//! voxel-player: first-person player controller with voxel collision (S-08 spike).
//!
//! A `Player` (position, AABB hitbox, yaw) moved by a `PlayerController` that collides against
//! the solid voxels of a `World` axis-by-axis (slide along walls, rest on ground). Renderer-
//! agnostic: depends only on `voxel-core` + `voxel-world`.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_world::World;

/// Half-extents of the player's AABB hitbox (centered on `pos`).
const HALF: [f32; 3] = [0.3, 0.9, 0.3];
/// Gravity acceleration (world units / s^2).
const GRAVITY: f32 = 24.0;
/// Jump impulse (world units / s).
const JUMP_SPEED: f32 = 8.0;
/// Horizontal move speed (world units / s).
const MOVE_SPEED: f32 = 5.0;
/// Maximum physics sub-step (s) to avoid tunnelling through thin voxels at high speed.
const MAX_SUB_DT: f32 = 0.02;

/// A player avatar: position, facing yaw, and ground state.
#[derive(Debug, Clone)]
pub struct Player {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub on_ground: bool,
}

impl Player {
    /// Create a player at `pos` facing +X (yaw = 0).
    pub fn new(pos: [f32; 3]) -> Self {
        Self {
            pos,
            yaw: 0.0,
            on_ground: false,
        }
    }
}

/// Per-step movement intent (boolean buttons; analog later).
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
}

impl Input {
    /// Only forward held.
    pub fn forward() -> Self {
        Self {
            forward: true,
            ..Default::default()
        }
    }
    /// No input.
    pub fn none() -> Self {
        Self::default()
    }
}

/// Drives a `Player` against a `World` with axis-separated collision.
pub struct PlayerController {
    vel_y: f32,
}

impl PlayerController {
    /// Create a controller (tracks vertical velocity for gravity/jump).
    pub fn new() -> Self {
        Self { vel_y: 0.0 }
    }

    /// Advance the player by `dt` seconds given the world and input. Internally sub-steps so
    /// fast motion (e.g. falling) cannot tunnel through voxels.
    pub fn step(&mut self, world: &mut World, player: &mut Player, input: Input, dt: f32) {
        let sub = (dt / MAX_SUB_DT).ceil().max(1.0) as usize;
        let sdt = dt / sub as f32;
        for _ in 0..sub {
            self.substep(world, player, input, sdt);
        }
    }

    fn substep(&mut self, world: &mut World, player: &mut Player, input: Input, dt: f32) {
        // Desired horizontal direction from yaw (0 = +X, 90deg = +Z).
        let (sin, cos) = player.yaw.sin_cos();
        let mut dx = 0.0f32;
        let mut dz = 0.0f32;
        if input.forward {
            dx += cos;
            dz += sin;
        }
        if input.back {
            dx -= cos;
            dz -= sin;
        }
        if input.right {
            dx -= sin;
            dz += cos;
        }
        if input.left {
            dx += sin;
            dz -= cos;
        }
        let len = (dx * dx + dz * dz).sqrt();
        if len > 1e-6 {
            dx = dx / len * MOVE_SPEED * dt;
            dz = dz / len * MOVE_SPEED * dt;
        }

        // X axis (slide independent of Z).
        let try_x = [player.pos[0] + dx, player.pos[1], player.pos[2]];
        if !collides(world, &try_x) {
            player.pos[0] = try_x[0];
        }
        // Z axis.
        let try_z = [player.pos[0], player.pos[1], player.pos[2] + dz];
        if !collides(world, &try_z) {
            player.pos[2] = try_z[2];
        }

        // Y axis (gravity + jump).
        if input.jump && player.on_ground {
            self.vel_y = JUMP_SPEED;
            player.on_ground = false;
        }
        self.vel_y -= GRAVITY * dt;
        let dy = self.vel_y * dt;
        let try_y = [player.pos[0], player.pos[1] + dy, player.pos[2]];
        if collides(world, &try_y) {
            if dy < 0.0 {
                // Resolve: rest the feet on top of the highest solid voxel below.
                player.pos[1] = resolve_floor_y(world, player.pos[0], player.pos[2], player.pos[1]);
                player.on_ground = true;
            } else if dy > 0.0 {
                // Hit a ceiling: stop just below it (keep current y).
                player.on_ground = false;
            }
            self.vel_y = 0.0;
        } else {
            player.pos[1] = try_y[1];
            // Ground probe just below the feet.
            player.on_ground = collides(
                world,
                &[player.pos[0], player.pos[1] - 0.05, player.pos[2]],
            );
        }
    }
}

/// True if the player's AABB at `pos` overlaps any solid (non-air) voxel.
fn collides(world: &mut World, pos: &[f32; 3]) -> bool {
    let min = [pos[0] - HALF[0], pos[1] - HALF[1], pos[2] - HALF[2]];
    let max = [pos[0] + HALF[0], pos[1] + HALF[1], pos[2] + HALF[2]];
    let x0 = min[0].floor() as i64;
    let x1 = max[0].floor() as i64;
    let y0 = min[1].floor() as i64;
    let y1 = max[1].floor() as i64;
    let z0 = min[2].floor() as i64;
    let z1 = max[2].floor() as i64;
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                if solid_at(world, x, y, z) {
                    return true;
                }
            }
        }
    }
    false
}

/// Read solidity (non-air) at a world voxel without panicking on missing chunks.
fn solid_at(world: &mut World, x: i64, y: i64, z: i64) -> bool {
    let wv = WorldVoxel::new(x, y, z);
    let coord = voxel_core::coords::ChunkCoord::from_world(wv);
    let local = voxel_core::coords::LocalVoxel::from_world(wv);
    let chunk = world.get_or_generate(coord);
    chunk.get(local) != MaterialId::from(0)
}

/// Rest the player's center on top of the highest solid voxel below the feet.
///
/// A solid voxel at world-Y `yv` fills `[yv, yv+1)`; the feet must sit on its top `yv+1`, so
/// the player center is `(yv + 1) + HALF[1]`. `center_y` is the current (colliding) center.
fn resolve_floor_y(world: &mut World, x: f32, z: f32, center_y: f32) -> f32 {
    let foot = center_y - HALF[1];
    let start = foot.floor() as i64;
    for y in (start - 8..=start).rev() {
        if solid_at(world, x.floor() as i64, y, z.floor() as i64) {
            return (y as f32) + 1.0 + HALF[1];
        }
    }
    center_y
}

impl Default for PlayerController {
    fn default() -> Self {
        Self::new()
    }
}
