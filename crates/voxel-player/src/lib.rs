//! voxel-player: first-person player controller with voxel collision (S-08 spike).
//!
//! A `Player` (position, AABB hitbox, yaw) moved by a `PlayerController` that collides against
//! the solid voxels of a `World` axis-by-axis (slide along walls, rest on ground). Renderer-
//! agnostic: depends only on `voxel-core` + `voxel-world`.

use voxel_core::coords::{WorldVoxel, VOXEL_SIZE_M};
use voxel_core::palette::MaterialId;
use voxel_world::World;

/// Half-extents of the player's AABB hitbox in VOXEL units (the controller works in voxel
/// coordinates; multiply by VOXEL_SIZE_M for meters). Width 2.4 vox = 0.30 m, height
/// 2*7.6 = 15.2 vox = 1.90 m (human reference avatar, 2026-07-15) so the terrain scale
/// reads correctly.
pub const HALF: [f32; 3] = [2.4, 7.6, 2.4];
/// Total player height in meters (2 * HALF[1] * VOXEL_SIZE_M) — the human reference avatar
/// is exactly 1.90 m.
pub const PLAYER_HEIGHT_M: f32 = 2.0 * HALF[1] * VOXEL_SIZE_M;
/// Gravity acceleration (voxel units / s^2) ≈ 24.5 m/s^2.
const GRAVITY: f32 = 196.0;
/// Jump impulse (voxel units / s) ≈ 8 m/s (≈1.3 m jump).
const JUMP_SPEED: f32 = 64.0;
/// Horizontal move speed (voxel units / s) ≈ 1.5 m/s.
const MOVE_SPEED: f32 = 12.0;
/// Maximum physics sub-step (s) to avoid tunnelling through thin voxels at high speed.
const MAX_SUB_DT: f32 = 0.02;
/// Terminal fall speed (world units / s). Keeps per-substep displacement < 1 voxel so a
/// 1-thick floor can never be tunnelled through (S-11 audit fix P-01).
const TERMINAL_FALL_SPEED: f32 = 40.0;
/// Max step-up height in voxels: lets the avatar walk up gentle slopes/ledges instead of
/// being blocked by every 1-voxel rise (the "can't walk up a hill, only jump" bug).
const STEP_HEIGHT: f32 = 1.0;

/// A player avatar: position, facing yaw, and ground state.
#[derive(Debug, Clone)]
pub struct Player {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub on_ground: bool,
}

/// Movement mode toggled at runtime (e.g. press F): `Walk` collides with terrain and
/// obeys gravity; `Fly` is free 6-DOF (no gravity, no collision) for exploration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMode {
    Walk,
    Fly,
}

impl PlayerMode {
    /// Toggle between walk and fly.
    pub fn toggle(self) -> Self {
        match self {
            PlayerMode::Walk => PlayerMode::Fly,
            PlayerMode::Fly => PlayerMode::Walk,
        }
    }
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

        // X and Z steps each resolve from the SAME base Y (not chained), so a diagonal
        // slope can't double-step (~2 vox/substep). Final Y is the highest of the two
        // accepted steps; if neither stepped, Y is unchanged.
        let base_y = player.pos[1];
        let mut step_y = base_y;
        // X axis (slide independent of Z); step up gentle rises instead of blocking.
        let try_x = [player.pos[0] + dx, base_y, player.pos[2]];
        if let Some(sy) = try_step(world, &try_x, base_y) {
            player.pos[0] = try_x[0];
            step_y = step_y.max(sy);
        }
        // Z axis.
        let try_z = [player.pos[0], base_y, player.pos[2] + dz];
        if let Some(sy) = try_step(world, &try_z, base_y) {
            player.pos[2] = try_z[2];
            step_y = step_y.max(sy);
        }
        // Apply the accepted step-up (if any) to the player's Y. NOTE: a flat horizontal
        // move always returns `Some(base_y)` (try_step), so we must NOT derive `on_ground`
        // from the horizontal step — that would flag the player as grounded while airborne
        // and permit mid-air jumps. `on_ground` is owned exclusively by the Y-axis
        // gravity/floor resolution below.
        if step_y > base_y + 1e-3 {
            player.pos[1] = step_y;
        }

        // Y axis (gravity + jump).
        if input.jump && player.on_ground {
            self.vel_y = JUMP_SPEED;
            player.on_ground = false;
        }
        self.vel_y -= GRAVITY * dt;
        if self.vel_y < -TERMINAL_FALL_SPEED {
            self.vel_y = -TERMINAL_FALL_SPEED;
        }
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

/// Try to move the player's AABB to `target` (same x/z, possibly raised y). Returns the
/// y to place the player at if the move is unobstructed, stepping up by at most
/// `STEP_HEIGHT` voxels when a *wall* at the player's own level blocks the flat move.
///
/// Crucial: the flat-move check EXCLUDES the floor layer under the feet. The ground the
/// player stands on must not count as "blocking" — otherwise we'd step up 1 voxel every
/// frame and fly off to infinity (NaN eye -> streaming selects 0 chunks -> white screen).
/// A genuine wall (solid voxel at the player's own level) still triggers the step-up.
/// The raised position is only accepted if there is solid ground directly beneath the
/// feet across the FULL footprint (so you climb a ledge, not a 1-voxel pillar). Returns
/// `None` if even the max step-up is blocked.
fn try_step(world: &mut World, target: &[f32; 3], base_y: f32) -> Option<f32> {
    // Shift the AABB up 1 voxel so the foot-level floor is excluded from the wall test.
    let floor_excluded = [target[0], target[1] + 1.0, target[2]];
    if !collides(world, &floor_excluded) {
        return Some(target[1]);
    }
    let steps = STEP_HEIGHT.ceil() as i32;
    for s in 1..=steps {
        let y = base_y + s as f32;
        let raised = [target[0], y, target[2]];
        let raised_floor_excl = [raised[0], y + 1.0, raised[2]];
        if collides(world, &raised_floor_excl) {
            continue; // still a wall at this height
        }
        // Require solid ground directly beneath the raised feet across the whole footprint
        // (not just the center column), so we don't step onto a 1-voxel-wide pillar.
        let foot_y = y - HALF[1] - 1.0;
        let foot_min = [raised[0] - HALF[0], foot_y, raised[2] - HALF[2]];
        let foot_max = [raised[0] + HALF[0], foot_y + 0.5, raised[2] + HALF[2]];
        if !footprint_has_ground(world, &foot_min, &foot_max) {
            continue;
        }
        return Some(y);
    }
    None
}

/// True if any solid voxel sits in the thin slab spanned by `min`/`max` (the foot layer),
/// i.e. there is ground directly beneath the player's footprint.
fn footprint_has_ground(world: &mut World, min: &[f32; 3], max: &[f32; 3]) -> bool {
    let x0 = (min[0]).floor() as i64;
    let x1 = (max[0]).floor() as i64;
    let y0 = (min[1]).floor() as i64;
    let y1 = (max[1]).floor() as i64;
    let z0 = (min[2]).floor() as i64;
    let z1 = (max[2]).floor() as i64;
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

/// Read solidity (non-air) at a world voxel without cloning a whole chunk (S-12 fix:
/// was `get_or_generate(...).get(...)`, which cloned a 32 KB chunk per sample during
/// collision; now uses the cheap `material_at` reader from voxel-world).
fn solid_at(world: &mut World, x: i64, y: i64, z: i64) -> bool {
    let wv = WorldVoxel::new(x, y, z);
    world.material_at(wv) != MaterialId::from(0)
}

/// Rest the player's center on top of the highest solid voxel below the feet.
///
/// A solid voxel at world-Y `yv` fills `[yv, yv+1)`; the feet must sit on its top `yv+1`, so
/// the player center is `(yv + 1) + HALF[1]`. `center_y` is the current (colliding) center.
/// Samples every column overlapped by the AABB footprint (up to four), not just the center
/// column (S-11 audit fix), and takes the highest solid top.
fn resolve_floor_y(world: &mut World, x: f32, z: f32, center_y: f32) -> f32 {
    let foot = center_y - HALF[1];
    let start = foot.floor() as i64;
    let x0 = (x - HALF[0]).floor() as i64;
    let x1 = (x + HALF[0]).floor() as i64;
    let z0 = (z - HALF[2]).floor() as i64;
    let z1 = (z + HALF[2]).floor() as i64;
    for y in (start - 8..=start).rev() {
        for cx in x0..=x1 {
            for cz in z0..=z1 {
                if solid_at(world, cx, y, cz) {
                    return (y as f32) + 1.0 + HALF[1];
                }
            }
        }
    }
    center_y
}

impl Default for PlayerController {
    fn default() -> Self {
        Self::new()
    }
}
