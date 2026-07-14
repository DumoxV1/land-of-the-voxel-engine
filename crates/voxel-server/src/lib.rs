//! voxel-server: headless authoritative game server (S-09 spike).
//!
//! Simulates a voxel world + players without any renderer/GPU (ADR-0003). Server-authoritative:
//! the world, edits (replayable `EditLog`), and player physics all live here. Networking is a
//! later phase; this crate proves the simulation is headless and deterministic.

use std::collections::HashMap;

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_edit::{Edit, EditLog};
use voxel_player::{Input, Player, PlayerController};
use voxel_world::World;

/// A connected player on the server: its `Player` state, a controller, and its pending input.
struct Client {
    player: Player,
    ctrl: PlayerController,
    input: Input,
}

/// Headless authoritative server: owns the world, the replayable edit log, and all players.
pub struct Server {
    world: World,
    log: EditLog,
    players: HashMap<u32, Client>,
    tick: u64,
}

impl Server {
    /// Create a server with a seeded world (deterministic base terrain).
    pub fn new(seed: u32) -> Self {
        Self {
            world: World::new(seed),
            log: EditLog::new(),
            players: HashMap::new(),
            tick: 0,
        }
    }

    /// Add a player at a spawn position (above terrain; gravity settles it).
    pub fn add_player(&mut self, id: u32, pos: [f32; 3]) {
        self.players.insert(
            id,
            Client {
                player: Player::new(pos),
                ctrl: PlayerController::new(),
                input: Input::none(),
            },
        );
    }

    /// Set the input for a player (normally fed by the network layer in a later phase).
    pub fn set_input(&mut self, id: u32, input: Input) {
        if let Some(c) = self.players.get_mut(&id) {
            c.input = input;
        }
    }

    /// Advance the simulation by `dt` seconds. Steps every player's controller against the
    /// shared world. No renderer, no GPU.
    pub fn tick(&mut self, dt: f32) {
        self.tick += 1;
        // We must avoid holding a borrow on self.players while also mutating self.world.
        let ids: Vec<u32> = self.players.keys().copied().collect();
        for id in ids {
            if let Some(client) = self.players.get_mut(&id) {
                let Client {
                    player,
                    ctrl,
                    input,
                } = client;
                ctrl.step(&mut self.world, player, *input, dt);
            }
        }
    }

    /// Apply a place/remove edit authored by `actor` at a world position. Updates the shared
    /// world and appends to the replayable log (server-authoritative record).
    pub fn apply_edit(&mut self, actor: u32, world_pos: WorldVoxel, material: MaterialId) {
        let old = self.material_at(world_pos);
        self.world.set_voxel(world_pos, material);
        self.log.push(Edit {
            world: world_pos,
            old,
            new: material,
            actor,
            tick: self.tick,
            revision: 0, // assigned by the log
        });
    }

    /// Read the material at a world position (the shared, authoritative view).
    pub fn material_at(&mut self, world_pos: WorldVoxel) -> MaterialId {
        let coord = voxel_core::coords::ChunkCoord::from_world(world_pos);
        let local = voxel_core::coords::LocalVoxel::from_world(world_pos);
        self.world.get_or_generate(coord).get(local)
    }

    /// Borrow a player's state (or `None` if the id is unknown).
    pub fn player(&self, id: u32) -> Option<&Player> {
        self.players.get(&id).map(|c| &c.player)
    }

    /// Number of recorded edits.
    pub fn edit_count(&self) -> usize {
        self.log.len()
    }

    /// Borrow the underlying world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Number of connected players.
    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Total ticks elapsed since the server started.
    pub fn tick_count(&self) -> u64 {
        self.tick
    }

    /// Borrow the edit log.
    pub fn log(&self) -> &EditLog {
        &self.log
    }
}
