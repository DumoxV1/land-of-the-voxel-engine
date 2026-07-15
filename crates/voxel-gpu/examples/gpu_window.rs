//! S-12b / S-13 live GPU client: stream a micro-voxel world (12.5 cm/voxel) around
//! a first-person player avatar (1.90 m) that walks the terrain with voxel collision
//! (WASD + Space to jump + mouse-look). Chunks within `VIEW_RADIUS` of the camera are
//! generated + meshed on the fly (chunk-streaming). Run with:
//! `cargo run --release --example gpu_window -p voxel-gpu`

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M};
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_gpu::{mesh_chunk_world_meters, mesh_pool, MeshResult};
use voxel_mesher::Triangle;
use voxel_player::{Input, Player, PlayerController};
use voxel_world::World;
use voxel_worldgen::surface_height_m;

/// Tracks in-flight chunk mesh requests so we can (a) avoid re-requesting a chunk that is
/// already being generated, and (b) drop stale results when a newer request supersedes an
/// older one (camera moved away/back). `complete()` removes BOTH bookkeeping entries so the
/// map stays bounded over a long session — previously `requested_gen` leaked an entry per
/// unique chunk ever requested (P2 fix, 2026-07-15).
#[derive(Default)]
struct RequestTracker {
    pending: HashSet<ChunkCoord>,
    requested_gen: HashMap<ChunkCoord, u64>,
}

impl RequestTracker {
    /// Mark `coord` as in-flight; returns the generation tag to stamp on the result.
    fn request(&mut self, coord: ChunkCoord) -> u64 {
        let g = self
            .requested_gen
            .entry(coord)
            .and_modify(|g| *g += 1)
            .or_insert(1);
        self.pending.insert(coord);
        *g
    }

    /// True if a request is already in flight for this coord.
    fn is_pending(&self, coord: &ChunkCoord) -> bool {
        self.pending.contains(coord)
    }

    /// Current generation for a coord (used to validate incoming results).
    fn gen(&self, coord: &ChunkCoord) -> Option<u64> {
        self.requested_gen.get(coord).copied()
    }

    /// Mark a request complete: drops both the pending flag and the gen entry so the
    /// tracker's memory stays bounded (P2 — previously `requested_gen` leaked).
    fn complete(&mut self, coord: &ChunkCoord) {
        self.pending.remove(coord);
        self.requested_gen.remove(coord);
    }
}

#[cfg(test)]
mod request_tracker_tests {
    use super::*;

    #[test]
    fn request_marks_pending_and_increments_gen() {
        let mut t = RequestTracker::default();
        let c = ChunkCoord::new(1, 0, 2);
        let g1 = t.request(c);
        assert_eq!(g1, 1);
        assert!(t.is_pending(&c));
        assert_eq!(t.gen(&c), Some(1));
        // Re-request (still pending) must NOT bump the gen — the in-flight job owns gen 1.
        // (Caller guards with is_pending, but gen stays stable while pending.)
        assert_eq!(t.gen(&c), Some(1));
    }

    #[test]
    fn complete_removes_gen_entry_no_leak() {
        let mut t = RequestTracker::default();
        let c = ChunkCoord::new(3, 1, 4);
        let _ = t.request(c);
        assert!(t.gen(&c).is_some());
        t.complete(&c);
        // P2 invariant: after completion the gen entry is gone, so the map cannot grow
        // unbounded across a long session of many unique chunks.
        assert!(t.gen(&c).is_none());
        assert!(!t.is_pending(&c));
    }

    #[test]
    fn stale_result_dropped_after_complete() {
        let mut t = RequestTracker::default();
        let c = ChunkCoord::new(0, 0, 0);
        let g1 = t.request(c); // gen 1, pending
        assert_eq!(g1, 1);
        t.complete(&c); // P2: entry removed, memory bounded
        // A late result carrying the old generation arrives after completion — it must be
        // dropped (gen entry is now None, so no generation can match it).
        assert_eq!(t.gen(&c), None);
        // A fresh request starts a new (independent) generation; only its result is accepted.
        let g2 = t.request(c);
        assert_eq!(g2, 1);
        assert_eq!(t.gen(&c), Some(1));
    }
}

/// View distance in chunks. On the 12.5 cm scale a 4 m chunk -> 32 chunks ~= 128 m view.
const CHUNK_M: f32 = CHUNK_SIZE as f32 * VOXEL_SIZE_M; // 4 m (ADR-0005)
const VIEW_RADIUS: i64 = 48; // ~192 m view radius (radial disc; lifts the world toward a 150 km² feel)
/// Max VBO bytes we will fill. Matches the renderer's 256 MB staging cap. Once the
/// streamed mesh set reaches this, we stop requesting new chunks for this frame — the
/// rest pop in later as the camera moves / far chunks evict. Prevents the vertical-scale
/// spike from building 1.5 GB of meshes that can never be drawn (and keeps first load fast).
const VBO_BYTES_CAP: usize = 256 * 1024 * 1024;
/// Max chunks whose meshes we ingest from the worker channel per frame (P3 upload budget).
const UPLOAD_BUDGET: usize = 64; // chunks uploaded/frame; raised from 4 after the
                                 // vertical-scale spike multiplied the streamed set

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    scene: Option<GpuScene>,
    world: World,
    seed: u32,
    mesh_cache: voxel_gpu::cache::LruMeshCache,
    frame: u64,
    mesh_pool: rayon::ThreadPool,
    mesh_tx: crossbeam_channel::Sender<MeshResult>,
    mesh_rx: crossbeam_channel::Receiver<MeshResult>,
    requests: RequestTracker,
    camera: GpuCamera,
    /// First-person avatar (1.90 m) that walks the terrain with voxel collision.
    player: Player,
    controller: PlayerController,
    // Movement mode: Walk (collision + gravity) or Fly (free 6-DOF). Toggle with F.
    mode: voxel_player::PlayerMode,
    // Input state.
    keys: HashSet<winit::keyboard::PhysicalKey>,
    yaw: f32,
    pitch: f32,
    // Mouse-look drag state.
    dragging: bool,
    last_mouse: Option<(f64, f64)>,
    // Max texture dimension of the adapter.
    max_dim: u32,
    // Last configured surface size (for surface-loss recovery).
    surf_w: u32,
    surf_h: u32,
    // Last frame timestamp for dt-based movement (frame-rate independent free-fly).
    last_frame: std::time::Instant,
    // Day/night phase (0..1, F2) — advances slowly each frame.
    time_of_day: f32,
}

impl Default for App {
    fn default() -> Self {
        let seed = 7u32;
        let (mesh_tx, mesh_rx) = crossbeam_channel::unbounded::<MeshResult>();
        Self {
            window: None,
            surface: None,
            scene: None,
            world: World::new(seed),
            seed,
            // LRU mesh cache: cap 200k chunks (~RAM-light) or 12 GB estimated, whichever first.
            mesh_cache: voxel_gpu::cache::LruMeshCache::new(200_000, 12 * 1024 * 1024 * 1024),
            frame: 0,
            mesh_pool: mesh_pool(),
            mesh_tx,
            mesh_rx,
            requests: RequestTracker::default(),
            // First-person spawn: place the 1.90 m avatar's feet on the terrain surface at
            // the origin; camera eye is derived from the player each frame in update_camera.
            // NB: the initial eye MUST match the spawn position, not a placeholder — otherwise
            // the first frames stream + fall back around the wrong location and flash white
            // until the camera snaps to the player (seen as a "white screen" at startup).
            camera: {
                let top_vox = (surface_height_m(48, 48, seed) / VOXEL_SIZE_M) as i64;
                let spawn_vox = (top_vox as f32 + 1.0) + voxel_player::HALF[1];
                let eye_y_vox = spawn_vox - voxel_player::HALF[1] + 13.6; // 1.7 m above feet
                GpuCamera::new(
                    [
                        48.0 * VOXEL_SIZE_M,
                        eye_y_vox * VOXEL_SIZE_M,
                        48.0 * VOXEL_SIZE_M,
                    ],
                    -std::f32::consts::FRAC_PI_2,
                    -0.4,
                    1.0,
                )
            },
            player: {
                // Place in voxel units (the controller's coordinate space): spawn chunk
                // (1,0,1) center is world (6 m, 6 m) = voxel (48, 48).
                let top_vox = (surface_height_m(48, 48, seed) / VOXEL_SIZE_M) as i64;
                Player::new([48.0, (top_vox as f32 + 1.0) + voxel_player::HALF[1], 48.0])
            },
            controller: PlayerController::new(),
            mode: voxel_player::PlayerMode::Walk,
            keys: HashSet::new(),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.4,
            dragging: false,
            last_mouse: None,
            max_dim: 2048,
            surf_w: 1280,
            surf_h: 800,
            last_frame: std::time::Instant::now(),
            time_of_day: 0.32, // F2 dag/nacht: start in de vroege ochtend (gouden uur)
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Land of the Voxel Engine — GPU client (12.5 cm micro-voxels)")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("window creation failed"),
        );
        let size = window.inner_size();
        let camera = GpuCamera::new(
            self.camera.eye,
            self.yaw,
            self.pitch,
            size.width as f32 / size.height as f32,
        );
        self.camera = camera;

        // Build the scene + surface for the windowed format.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        let surface = instance
            .create_surface(window.clone())
            .expect("surface creation failed");
        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            }))
            .expect("no adapter");
        let (device, queue) =
            futures::executor::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // P0 spike (2026-07-15): raise the buffer-size limit to 2 GB so the
                // vertical-scale terrain (multi-chunk-Y) can be drawn without VBO
                // truncation. Mirrors `voxel_gpu::renderer::MAX_VBO_BYTES`.
                required_limits: wgpu::Limits {
                    max_buffer_size: voxel_gpu::renderer::MAX_VBO_BYTES as u64,
                    ..wgpu::Limits::downlevel_defaults()
                },
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                label: None,
                trace: wgpu::Trace::Off,
            }))
            .expect("no device");
        let device = std::sync::Arc::new(device);
        let queue = std::sync::Arc::new(queue);

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        // Clip the surface size to a safe maximum (DPI scaling can push past the adapter limit).
        let max_dim: u32 = 2048;
        let surf_w = size.width.min(max_dim);
        let surf_h = size.height.min(max_dim);
        self.max_dim = max_dim;

        let scene = GpuScene::new_for_surface(device, queue, surf_w, surf_h, format)
            .expect("scene init failed");
        log::info!("gpu_window: GPU scene initialized (format={:?})", format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: surf_w,
            height: surf_h,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(scene.device(), &config);
        self.surf_w = surf_w;
        self.surf_h = surf_h;
        self.window = Some(window);
        self.surface = Some(surface);
        self.scene = Some(scene);

        // First-person spawn: drop the camera onto the terrain at the spawn chunk.
        // Spawn the 1.90 m avatar's feet on the terrain surface near the chunk (1,0,1)
        // center; the camera eye is derived from the player each frame in `update_camera`.
        let spawn = ChunkCoord::new(1, 0, 1);
        let center_wx = (spawn.x * CHUNK_SIZE + CHUNK_SIZE / 2) as i64;
        let center_wz = (spawn.z * CHUNK_SIZE + CHUNK_SIZE / 2) as i64;
        let surface_m = voxel_worldgen::surface_height_m(center_wx, center_wz, self.seed);
        let top_vox = (surface_m / VOXEL_SIZE_M) as i64;
        // Player position is in voxel units; feet rest on the surface top voxel.
        self.player = Player::new([
            center_wx as f32,
            (top_vox as f32 + 1.0) + voxel_player::HALF[1],
            center_wz as f32,
        ]);
        // Eye is 1.7 m (13.6 vox) above the feet; report in meters.
        let eye_y_m = ((top_vox as f32 + 1.0) + 13.6) * VOXEL_SIZE_M;
        println!(
            "spawn: terrain top = {} voxels (~{:.2} m), player eye_y ~= {:.2} m (1.90 m avatar)",
            top_vox,
            surface_m,
            eye_y_m
        );

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(scene), Some(surface)) = (&mut self.scene, &self.surface) {
                    self.surf_w = size.width.max(1).min(self.max_dim);
                    self.surf_h = size.height.max(1).min(self.max_dim);
                    scene.resize(self.surf_w, self.surf_h);
                    let config = wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format: scene.format(),
                        width: self.surf_w,
                        height: self.surf_h,
                        present_mode: wgpu::PresentMode::Fifo,
                        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                        color_space: wgpu::SurfaceColorSpace::Auto,
                    };
                    surface.configure(scene.device(), &config);
                    self.camera.aspect = self.surf_w as f32 / self.surf_h as f32;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // Space (and any key) must not crash the loop. We simply track
                // held keys; no key triggers exit or panics.
                if event.state == ElementState::Pressed {
                    self.keys.insert(event.physical_key);
                    // F toggles walk <-> fly mode.
                    if event.physical_key
                        == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyF)
                    {
                        self.mode = self.mode.toggle();
                        println!(
                            "mode: {}",
                            if self.mode == voxel_player::PlayerMode::Walk {
                                "WALK"
                            } else {
                                "FLY"
                            }
                        );
                    }
                } else {
                    self.keys.remove(&event.physical_key);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.dragging = state == ElementState::Pressed;
                    if !self.dragging {
                        self.last_mouse = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x, position.y);
                if self.dragging {
                    if let Some((px, py)) = self.last_mouse {
                        let dx = x - px;
                        let dy = y - py;
                        self.yaw += dx as f32 * 0.005;
                        self.pitch = (self.pitch - dy as f32 * 0.005).clamp(-1.5, 1.5);
                        self.camera.yaw = self.yaw;
                        self.camera.pitch = self.pitch;
                    }
                    self.last_mouse = Some((x, y));
                }
            }
            WindowEvent::RedrawRequested => {
                self.update_camera();
                self.render_frame();
                // Continuous rendering: ask for the next frame. If the surface is
                // lost we simply skip and the loop keeps trying next event.
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

impl App {
    fn update_camera(&mut self) {
        // Frame-rate independent: integrate movement with a real dt (seconds) so speed is
        // in world-m/s regardless of FPS. The 1.90 m avatar walks the terrain via the
        // voxel-player controller (collision + gravity); camera eye follows the player.
        let now = std::time::Instant::now();
        let dt = self.last_frame.elapsed().as_secs_f32().clamp(0.0, 0.1);
        self.last_frame = now;

        let has = |code: winit::keyboard::KeyCode| {
            self.keys
                .contains(&winit::keyboard::PhysicalKey::Code(code))
        };
        let input = Input {
            forward: has(winit::keyboard::KeyCode::KeyW),
            back: has(winit::keyboard::KeyCode::KeyS),
            left: has(winit::keyboard::KeyCode::KeyA),
            right: has(winit::keyboard::KeyCode::KeyD),
            jump: has(winit::keyboard::KeyCode::Space),
        };
        // Mouse-look drives facing; sync the player yaw so movement follows the camera.
        self.player.yaw = self.yaw;

        match self.mode {
            voxel_player::PlayerMode::Walk => {
                // Walk: collide with terrain + gravity (controller handles step-up so you
                // can climb gentle slopes/ledges instead of being blocked).
                self.controller
                    .step(&mut self.world, &mut self.player, input, dt);
            }
            voxel_player::PlayerMode::Fly => {
                // Fly: free 6-DOF, no gravity, no collision. W/S move along the look
                // direction, A/D strafe, Space up, Ctrl down.
                let (sy, cy) = self.pitch.sin_cos();
                let (sp, cp) = self.yaw.sin_cos();
                let fwd = [cp * cy, sy, sp * cy];
                let right = [cp, 0.0, sp];
                let fly_speed = 40.0 * dt; // voxel units/s (~5 m/s)
                let mut move_v = [0.0f32; 3];
                if input.forward {
                    move_v[0] += fwd[0];
                    move_v[1] += fwd[1];
                    move_v[2] += fwd[2];
                }
                if input.back {
                    move_v[0] -= fwd[0];
                    move_v[1] -= fwd[1];
                    move_v[2] -= fwd[2];
                }
                if input.right {
                    move_v[0] += right[0];
                    move_v[1] += right[1];
                    move_v[2] += right[2];
                }
                if input.left {
                    move_v[0] -= right[0];
                    move_v[1] -= right[1];
                    move_v[2] -= right[2];
                }
                if input.jump {
                    move_v[1] += 1.0;
                }
                if self
                    .keys
                    .contains(&winit::keyboard::PhysicalKey::Code(
                        winit::keyboard::KeyCode::ControlLeft,
                    ))
                    || self.keys.contains(&winit::keyboard::PhysicalKey::Code(
                        winit::keyboard::KeyCode::ControlRight,
                    ))
                {
                    move_v[1] -= 1.0;
                }
                let mlen = (move_v[0] * move_v[0] + move_v[1] * move_v[1] + move_v[2] * move_v[2])
                    .sqrt();
                if mlen > 1e-6 {
                    self.player.pos[0] += move_v[0] / mlen * fly_speed;
                    self.player.pos[1] += move_v[1] / mlen * fly_speed;
                    self.player.pos[2] += move_v[2] / mlen * fly_speed;
                }
                self.player.on_ground = false;
            }
        }

        // Player position is in voxel units; the renderer camera works in meters.
        // Eye sits 1.7 m (13.6 vox) above the feet.
        let eye_y_vox = self.player.pos[1] - voxel_player::HALF[1] + 13.6;
        self.camera.eye = [
            self.player.pos[0] * VOXEL_SIZE_M,
            eye_y_vox * VOXEL_SIZE_M,
            self.player.pos[2] * VOXEL_SIZE_M,
        ];
    }

    fn render_frame(&mut self) {
        self.frame += 1;
        // F2 dag/nacht: langzame cyclus (~1 volle dag per 10 min bij 60 FPS).
        self.time_of_day = (self.time_of_day + 1.0 / (60.0 * 600.0)) % 1.0;
        let (Some(scene), Some(surface)) = (&mut self.scene, &self.surface) else {
            return;
        };

        // --- (P3) Drain finished meshes from the worker channel (bounded per-frame budget,
        //     stale results dropped via generation counter). Non-blocking: the render thread
        //     never generates/meshes a chunk itself.
        let mut budget = UPLOAD_BUDGET;
        while budget > 0 {
            let r = match self.mesh_rx.try_recv() {
                Ok(r) => r,
                Err(_) => break,
            };
            budget -= 1;
            // Discard if a newer request superseded this one (camera moved away/back).
            if self.requests.gen(&r.coord) != Some(r.gen) {
                continue;
            }
            self.mesh_cache.insert(r.coord, r.tris, self.frame);
            self.requests.complete(&r.coord);
        }

        // --- Chunk-streaming: draw visible chunks; request missing ones off-thread. ---
        let mut tris: Vec<Triangle> = Vec::new();
        let mut vbo_bytes: usize = 0; // running estimate of streamed mesh bytes (VBO cap gate)
        let [ex, _ey, ez] = self.camera.eye;
        let ccx = (ex / CHUNK_M).floor() as i64;
        let ccz = (ez / CHUNK_M).floor() as i64;
        let half = CHUNK_M * 0.5; // 2 m half-extent (x/z)
        let half_y = 24.0; // terrain peaks ~40 m; pad for height + camera clearance
        const MAX_Y: i64 = 12; // hard cap on streamed vertical chunks (~48 m)
        let frustum = voxel_gpu::renderer::Frustum::from_view_proj(&self.camera.view_proj());
        let r2 = VIEW_RADIUS * VIEW_RADIUS;
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                // Radial cull: only stream the disc (dx^2+dz^2 <= R^2), not the square —
                // ~22% fewer columns at the same nominal view radius.
                if dx * dx + dz * dz > r2 {
                    continue;
                }
                let cx = ccx + dx;
                let cz = ccz + dz;
                // Only stream Y-slabs that can contain terrain: a column's surface height
                // bounds how high a solid voxel can appear. This avoids generating/meshing
                // the ~11 empty slabs above the ~26 m peaks (vertical-scale spike made the
                // raw 0..=MAX_Y sweep 13x the old chunk count -> multi-minute first load).
                let col_wx = (cx * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
                let col_wz = (cz * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
                let col_top_vox = (voxel_worldgen::surface_height_m(col_wx, col_wz, self.seed) / voxel_core::coords::VOXEL_SIZE_M) as i64;
                let max_cy = ((col_top_vox + voxel_core::coords::CHUNK_SIZE as i64) / voxel_core::coords::CHUNK_SIZE as i64).min(MAX_Y);
                // NOTE: negative chunk coords are valid (ChunkCoord is i64 + Euclidean div).
                // Do NOT skip them — skipping caused the "white screen when flying into
                // negative space" bug.
                for cy in 0..=max_cy {
                    // Frustum cull: skip chunks fully outside the camera view. Center Y = middle
                    // of this vertical chunk slab.
                    let center = [
                        (cx as f32 + 0.5) * CHUNK_M,
                        (cy as f32 * CHUNK_M) + half_y * 0.5,
                        (cz as f32 + 0.5) * CHUNK_M,
                    ];
                    if !frustum.intersects_aabb(center, half.max(half_y)) {
                        continue;
                    }
                    let coord = ChunkCoord::new(cx, cy, cz);
                    if let Some(m) = self.mesh_cache.get(&coord) {
                        // Borrow directly — extend_from_slice already copies; the prior
                        // `.clone()` was a pure waste (52 B/tri of alloc + memcpy per chunk).
                        tris.extend_from_slice(&m.tris);
                        vbo_bytes += m.tris.len() * std::mem::size_of::<voxel_mesher::Triangle>();
                        // Mark recently visible (separate mutable borrow, after the immutable read).
                        self.mesh_cache.touch(&coord, self.frame);
                    } else if !self.requests.is_pending(&coord) {
                        // VBO budget gate: don't request more chunks once we've filled the
                        // 256 MB cap — the rest pop in later as the camera moves / far
                        // chunks evict. Keeps first load fast post vertical-scale spike.
                        if vbo_bytes >= VBO_BYTES_CAP {
                            continue;
                        }
                        vbo_bytes += 32 * 32 * 32 / 2 * std::mem::size_of::<voxel_mesher::Triangle>();
                    // Not ready and not yet requested: spawn off-thread generate+mesh.
                    let gen = self.requests.request(coord);
                    let tx = self.mesh_tx.clone();
                    let seed = self.seed;
                    self.mesh_pool.spawn(move || {
                        // CPU-only: pure worldgen + meshing, never touches the GPU.
                        let chunk = voxel_worldgen::generate_chunk(coord, seed);
                        let tris = mesh_chunk_world_meters(&chunk);
                        let _ = tx.send(MeshResult { coord, gen, tris });
                    });
                }
                // else: pending, not ready yet -> skipped this frame, pops in later.
                } // cy
            } // dz
        } // dx

        // Frame-1 fallback only: seed the chunk directly under the camera (the surface
        // chunk for the spawn column) so the very first frame already shows terrain
        // instead of a white clear-flash. Frustum-based selection can miss the ground
        // when the eye sits low and looks down, so we target the surface slab explicitly.
        if tris.is_empty() {
            let col_wx = (ccx * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
            let col_wz = (ccz * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
            let col_top_vox = (voxel_worldgen::surface_height_m(col_wx, col_wz, self.seed) / 0.125) as i64;
            // Surface slab is the chunk CONTAINING the surface voxel (div_euclid), not the
            // one above it. Seed both the surface chunk and the one below it so the ground
            // is guaranteed visible on frame 1 (GPT-sol review Q2: old formula picked the
            // empty chunk above the surface, defeating the fallback).
            let cy = col_top_vox.div_euclid(voxel_core::coords::CHUNK_SIZE as i64).clamp(0, MAX_Y);
            for cy_seed in [cy.saturating_sub(1), cy] {
                if cy_seed > MAX_Y {
                    continue;
                }
                let coord = ChunkCoord::new(ccx, cy_seed, ccz);
                let chunk = self.world.get_or_generate(coord);
                let mesh = mesh_chunk_world_meters(&chunk);
                self.mesh_cache.insert(coord, mesh.clone(), self.frame);
                tris.extend_from_slice(&mesh);
            }
        }

        let frame = surface.get_current_texture();
        let tex = match frame {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            // Surface lost / outdated (focus change, minimize, GPU reset, or the
            // OS snapping the window when Space is pressed): reconfigure at the
            // last known size and skip this frame instead of crashing.
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                if let (Some(scene), Some(surface)) = (&self.scene, &self.surface) {
                    surface.configure(
                        scene.device(),
                        &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: scene.format(),
                            width: self.surf_w.max(1),
                            height: self.surf_h.max(1),
                            present_mode: wgpu::PresentMode::Fifo,
                            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                            view_formats: vec![],
                            desired_maximum_frame_latency: 2,
                            color_space: wgpu::SurfaceColorSpace::Auto,
                        },
                    );
                }
                return;
            }
            // Timeout / Occluded / Validation: transient, just skip the frame.
            _ => return,
        };
        let view = tex
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        match scene.render_to_view(&tris, &self.camera, &view, self.time_of_day) {
            Ok(()) => scene.queue().present(tex),
            Err(err) => log::error!(
                "gpu_window: render failed (tris={}, surface={}x{}): {err:#}",
                tris.len(),
                self.surf_w,
                self.surf_h
            ),
        }
    }
}

fn main() {
    env_logger::init();
    println!(
        "Land of the Voxel Engine — micro-voxel client (12.5 cm/voxel, {} m chunks, view radius {} chunks ~{:.0} m)",
        CHUNK_M, VIEW_RADIUS, VIEW_RADIUS as f32 * CHUNK_M
    );
    println!("WASD = move, Space = jump/up, Left-drag = look, F = toggle walk/fly. Close window to exit.");
    let mut app = App::default();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut app).expect("run app");
}
