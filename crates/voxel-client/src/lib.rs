//! S-12b / S-13 live GPU client: stream a micro-voxel world (12.5 cm/voxel) around
//! a first-person player avatar (1.90 m) that walks the terrain with voxel collision
//! (WASD + Space to jump + mouse-look). Chunks within `VIEW_RADIUS` of the camera are
//! generated + meshed on the fly (chunk-streaming).
//!
//! This crate holds the client application (game loop, streaming worker pool, input,
//! and the `App` that wires the renderer to the world). It was extracted from the old
//! `voxel-gpu/examples/gpu_window.rs` so the client is a proper, testable crate rather
//! than a monolithic example. Run the live window with:
//! `cargo run --release --example gpu_window_main -p voxel-client`
//!
//! Architecture notes (2026-07-15 refactor):
//! - Streaming is driven by `voxel_gpu::chunk_stream::ChunkScheduler` (close→far priority,
//!   LOD rings, air-skip) + a bounded worker pool (`job_tx` channel, N worker threads).
//! - Worker messages are two-phase (`voxel_gpu::WorkerMsg`): `Gen` (raw chunk for collision,
//!   shipped first) then `Mesh` (triangles for drawing) — collision-first (A3).

mod hud; // debug HUD: bitmap-font FPS/stats overlay, drawn after the voxel pass.
mod profiling; // Tracy real-time frame profiler (behind `tracy` feature flag).

#[cfg(feature = "tracy")]
use tracy_client::{plot, span};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use voxel_core::coords::{ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M, WorldVoxel};
use voxel_core::palette::MaterialId;
use voxel_edit::{raycast_voxel, EditTool};
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_gpu::{mesh_chunk_world_meters};
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
/// Spawn safety margin (voxels) ABOVE the heightfield estimate. The heightfield (`surface_height_m`)
/// can sit up to ~OVERHANG_AMP_CEIL (28 vox) BELOW the real solid top once the 3D density field
/// (overhangs/caves) is applied, so spawning exactly on the estimate buries the camera inside the
/// terrain — you see the inside of the ground (grey/blue), surface chunks get frustum-culled (few
/// TRIS), and the first frames flash white. A safe margin keeps the eye clearly above the real top
/// in both WALK (falls to the ground) and FLY (hovers just above).
const SPAWN_SAFE_VOX: i64 = 64; // 8 m headroom (> overhang bulge + buffer)
/// Max VBO bytes we will fill. Matches the renderer's 256 MB staging cap. Once the
/// streamed mesh set reaches this, we stop requesting new chunks for this frame — the
/// rest pop in later as the camera moves / far chunks evict. Prevents the vertical-scale
/// spike from building 1.5 GB of meshes that can never be drawn (and keeps first load fast).
const VBO_BYTES_CAP: usize = 256 * 1024 * 1024;
/// Max chunks whose meshes we ingest from the worker channel per frame (P3 upload budget).
const UPLOAD_BUDGET: usize = 64; // chunks uploaded/frame; raised from 4 after the
                                 // vertical-scale spike multiplied the streamed set

/// Number of background mesh workers. Kept just below core count so the render thread
/// keeps a responsive time slice (state-of-the-art back-pressure: a bounded job channel,
/// not an unbounded rayon spawn storm).
fn num_mesh_workers() -> usize {
    num_cpus::get().saturating_sub(1).max(2)
}

pub struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    scene: Option<GpuScene>,
    world: World,
    seed: u32,
    mesh_cache: voxel_gpu::cache::LruMeshCache,
    frame: u64,
    // Bounded worker pool: jobs pushed onto `job_tx` (capacity = workers*2), N worker
    // threads `recv()` and stream results back via `mesh_tx`. The render thread can never
    // outrun the workers (real back-pressure vs the old unbounded rayon spawn).
    job_tx: Option<crossbeam_channel::Sender<voxel_gpu::chunk_stream::ChunkJob>>,
    workers: Vec<std::thread::JoinHandle<()>>,
    mesh_tx: crossbeam_channel::Sender<voxel_gpu::WorkerMsg>,
    mesh_rx: crossbeam_channel::Receiver<voxel_gpu::WorkerMsg>,
    requests: RequestTracker,
    // Streaming scheduler: close→far priority, LOD rings, air-skip.
    scheduler: voxel_gpu::chunk_stream::ChunkScheduler,
    heights: voxel_gpu::chunk_stream::HeightCache,
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
    // Debug HUD (top-right): FPS + live stats, drawn after the voxel pass.
    hud: Option<hud::Hud>,
    // Autonomous perf metrics: Hermes reads this to track regressions without the Tracy GUI.
    // Writes a one-line sample to profile_metrics.log every ~1s (feature-independent so the
    // normal build also produces telemetry Hermes can ingest).
    perf_log_timer: std::time::Instant,
    // I1 (live edit): edit-tool (records edits) + set van chunks die de speler heeft bewerkt,
    // zodat de streaming-worker die niet overschrijft met verse worldgen.
    edit_tool: EditTool,
    edited: std::collections::HashSet<ChunkCoord>,
}

impl Default for App {
    fn default() -> Self {
        let seed = 7u32;
        let (mesh_tx, mesh_rx) = crossbeam_channel::unbounded::<voxel_gpu::WorkerMsg>();
        let n = num_mesh_workers();
        // Bounded job channel: capacity = workers*2 gives real back-pressure. Workers block
        // on recv(); the render thread's `try_send` simply drops the job (re-issued next
        // frame) once the channel is full, instead of spawning unbounded work.
        let (job_tx, job_rx) = crossbeam_channel::bounded::<voxel_gpu::chunk_stream::ChunkJob>(n * 2);
        let workers = (0..n)
            .map(|_| {
                let rx = job_rx.clone();
                let tx = mesh_tx.clone();
                std::thread::spawn(move || {
                    while let Ok(job) = rx.recv() {
                        // A3 (collision-first): run the job as two phases — Gen (raw chunk for
                        // collision) then Mesh (triangles for drawing) — so player collision
                        // can run on freshly streamed terrain immediately, without waiting for
                        // the mesh or re-generating the chunk on the render thread.
                        let _span = span!("worker_job");
                        voxel_gpu::run_mesh_job(job, seed, &tx);
                    }
                })
            })
            .collect();
        // The original job_rx is dropped so only the worker clones keep it alive; when all
        // workers exit the channel closes (handled by the while-let above).
        drop(job_rx);
        Self {
            window: None,
            surface: None,
            scene: None,
            hud: None,
            world: World::new(seed),
            seed,
            // LRU mesh cache: cap 200k chunks (~RAM-light) or 12 GB estimated, whichever first.
            mesh_cache: voxel_gpu::cache::LruMeshCache::new(200_000, 12 * 1024 * 1024 * 1024),
            frame: 0,
            job_tx: Some(job_tx),
            workers,
            mesh_tx,
            mesh_rx,
            requests: RequestTracker::default(),
            scheduler: voxel_gpu::chunk_stream::ChunkScheduler::new(
                voxel_gpu::chunk_stream::StreamConfig {
                    view_radius: VIEW_RADIUS as i64,
                    max_y: 12,
                    requests_per_frame: 4,
                    lod_half_radius: 8,
                    // Imposter tier disabled: set == view_radius so no loaded chunk uses
                    // it. Isolated flat imposter quads read as squares floating in the sky
                    // from high altitude; the far ring falls back to Half (connected mesh).
                    lod_imposter_radius: VIEW_RADIUS as i64,
                    air_margin: 1,
                },
            ),
            heights: voxel_gpu::chunk_stream::HeightCache::new(2048),
            // First-person spawn: place the 1.90 m avatar's feet on the terrain surface at
            // the origin; camera eye is derived from the player each frame in update_camera.
            // NB: the initial eye MUST match the spawn position, not a placeholder — otherwise
            // the first frames stream + fall back around the wrong location and flash white
            // until the camera snaps to the player (seen as a "white screen" at startup).
            camera: {
                let top_vox = (surface_height_m(48, 48, seed) / VOXEL_SIZE_M) as i64;
                let eye_y_vox = top_vox as f32 + 1.0 + 13.6 + SPAWN_SAFE_VOX as f32; // feet+eye+headroom above real top
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
                // (1,0,1) center is world (6 m, 6 m) = voxel (48, 48). Feet sit SPAWN_SAFE_VOX
                // above the heightfield estimate so the eye (derived from pos each frame) clears
                // the real overhang-bulged top — matches the camera init above.
                let top_vox = (surface_height_m(48, 48, seed) / VOXEL_SIZE_M) as i64;
                Player::new([48.0, (top_vox as f32 + 1.0) + SPAWN_SAFE_VOX as f32, 48.0])
            },
            controller: PlayerController::new(),
            mode: voxel_player::PlayerMode::Walk,
            keys: HashSet::new(),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.4,
            dragging: false,
            last_mouse: None,
            max_dim: 2048, // overwritten at scene init from adapter.limits()
            surf_w: 1280,
            surf_h: 800,
            last_frame: std::time::Instant::now(),
            time_of_day: 0.32, // F2 dag/nacht: start in de vroege ochtend (gouden uur)
            perf_log_timer: std::time::Instant::now(),
            edit_tool: EditTool::new(),
            edited: std::collections::HashSet::new(),
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
                // Limits: take the adapter's real capabilities as the base (so texture
                // dimensions match what the GPU actually supports), then raise the buffer
                // size to 2 GB so vertical-scale terrain can be drawn without VBO truncation.
                // P0 spike (2026-07-15): previously downlevel_defaults() forced a 2048px
                // texture cap, which crashed at fullscreen (2240px) and clipped the surface.
                required_limits: wgpu::Limits {
                    max_buffer_size: voxel_gpu::renderer::MAX_VBO_BYTES as u64,
                    max_texture_dimension_2d: adapter.limits().max_texture_dimension_2d,
                    max_texture_dimension_1d: adapter.limits().max_texture_dimension_1d,
                    max_texture_dimension_3d: adapter.limits().max_texture_dimension_3d,
                    ..wgpu::Limits::default()
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

        // Clip the surface size to the adapter's real texture limit (never exceed what the
        // GPU supports — the depth texture is created at exactly surf_w x surf_h, so going
        // past max_texture_dimension_2d makes device.create_texture fatal).
        let max_dim: u32 = adapter.limits().max_texture_dimension_2d;
        let surf_w = size.width.min(max_dim);
        let surf_h = size.height.min(max_dim);
        self.max_dim = max_dim;

        let scene = GpuScene::new_for_surface(device, queue, surf_w, surf_h, format)
            .expect("scene init failed");
        log::info!("gpu_window: GPU scene initialized (format={:?})", format);

        // Debug HUD: build once we have a device/queue/format (after scene takes
        // ownership of the Arc<Device>/Arc<Queue> — borrow back from the scene).
        let hud = hud::Hud::new(scene.device(), scene.queue(), format);

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
        self.hud = Some(hud);

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
                } else if state == ElementState::Pressed {
                    // I1 live edit: rechts = plaats blok, midden = verwijder blok.
                    match button {
                        MouseButton::Right => self.edit_at_look(true),
                        MouseButton::Middle => self.edit_at_look(false),
                        _ => {}
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
                let _span = span!("frame");
                self.update_camera();
                self.render_frame();
                // Mark the end of the frame for Tracy's frame-time graph.
                frame_mark!();
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
                // Strafe axis is 90° yaw-perpendicular to forward (NOT the forward dir itself,
                // which made A/D duplicate W/S). right = up × fwd_horizontal.
                let right = [-sp, 0.0, cp];
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

        // --- (P3 + A3) Drain finished worker messages (bounded per-frame budget). ---
        //     Non-blocking: the render thread never generates/meshes a chunk itself.
        //     - Gen  (phase-1): insert the raw chunk into the client World so player
        //       collision can run on freshly streamed terrain immediately — no re-generate,
        //       no wait for the mesh.
        //     - Mesh (phase-2): insert the triangles into the GPU mesh cache for drawing.
        //     The scheduler's `seen` guard (checked by the `ready` closure against the cache)
        //     prevents re-requesting an already-cached chunk, so no gen-counter is needed.
        let mut budget = UPLOAD_BUDGET;
        while budget > 0 {
            let r = match self.mesh_rx.try_recv() {
                Ok(r) => r,
                Err(_) => break,
            };
            budget -= 1;
            match r {
                voxel_gpu::WorkerMsg::Gen { coord, chunk } => {
                    // Collision-first: feed the client World (player physics) without waiting
                    // for the mesh. Skip chunks the player has edited — do not clobber the
                    // edit with fresh worldgen (I1).
                    if !self.edited.contains(&coord) {
                        self.world.insert(coord, chunk);
                    }
                }
                voxel_gpu::WorkerMsg::Mesh { coord, tris } => {
                    self.mesh_cache.insert(coord, tris, self.frame);
                }
            }
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
        let _cam_slab = ((ex / VOXEL_SIZE_M / CHUNK_M as f32) as i64).clamp(0, MAX_Y);

        // --- Pass A: request streamed chunks (close→far priority, LOD rings, air-skip). ---
        // The scheduler plans at most `requests_per_frame` jobs this frame, closest first.
        // We then frustum-cull EACH job BEFORE handing it to the bounded worker pool, so we
        // never generate/mesh a chunk the camera can't see (the old loop only frustum-culled
        // at draw time, still paying for chunks behind the player).
        let _span = span!("scheduler_plan");
        let jobs = self.scheduler.plan(
            ccx,
            ccz,
            _cam_slab,
            &mut self.heights,
            self.seed,
            |c| self.mesh_cache.get(c).is_some(),
        );
        if let Some(tx) = &self.job_tx {
            for job in jobs {
                // Frustum cull the chunk center (full chunk AABB) before queueing work.
                let center = [
                    (job.coord.x as f32 + 0.5) * CHUNK_M,
                    (job.coord.y as f32 * CHUNK_M) + half_y * 0.5,
                    (job.coord.z as f32 + 0.5) * CHUNK_M,
                ];
                if !frustum.intersects_aabb(center, half.max(half_y)) {
                    // Not visible this frame: drop the reservation so it can be re-planned
                    // (and re-frustum-tested) when the camera turns toward it.
                    self.scheduler.forget(&job.coord);
                    continue;
                }
                // Stap 2 (inter-chunk occlusie, LxVL): skip chunks hidden behind taller
                // terrain along the view yaw. Pure height-wall check; drops work the player
                // literally cannot see (e.g. the far side of a hill). Re-planned next frame.
                if voxel_gpu::chunk_stream::is_occluded_by_terrain(
                    ccx, ccz, self.yaw, job.coord, _ey, &mut self.heights, self.seed,
                ) {
                    self.scheduler.forget(&job.coord);
                    continue;
                }
                // Bounded channel: if the workers are saturated, drop the job (re-issued next
                // frame). This is the real back-pressure that keeps the CPU responsive.
                let _ = tx.try_send(job);
            }
        }

        // --- Pass B: draw every cached chunk inside the view disc that is in-frustum. ---
        let r2 = VIEW_RADIUS * VIEW_RADIUS;
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                if dx * dx + dz * dz > r2 {
                    continue; // radial disc, not square
                }
                let cx = ccx + dx;
                let cz = ccz + dz;
                // Exact vertical band that can hold geometry (footprint-max surface down to
                // the bedrock floor). Skips the deep all-AIR slabs below; the scheduler's
                // air-skip already bounds the requested set, this just avoids drawing empties.
                let (lo_cy, hi_cy) = voxel_worldgen::column_solid_cy_range(cx, cz, self.seed);
                let lo_cy = lo_cy.max(0);
                let hi_cy = hi_cy.min(MAX_Y);
                for cy in lo_cy..=hi_cy {
                    let coord = ChunkCoord::new(cx, cy, cz);
                    let Some(m) = self.mesh_cache.get(&coord) else {
                        continue; // not ready yet this frame; pops in once the worker replies
                    };
                    // Frustum cull at draw time too (cheap, avoids binding off-screen chunks).
                    let center = [
                        (cx as f32 + 0.5) * CHUNK_M,
                        (cy as f32 * CHUNK_M) + half_y * 0.5,
                        (cz as f32 + 0.5) * CHUNK_M,
                    ];
                    if !frustum.intersects_aabb(center, half.max(half_y)) {
                        continue;
                    }
                    tris.extend_from_slice(&m.tris);
                    vbo_bytes += m.tris.len() * std::mem::size_of::<voxel_mesher::Triangle>();
                    self.mesh_cache.touch(&coord, self.frame);
                }
            }
        }

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
                let mesh = mesh_chunk_world_meters(&chunk, voxel_gpu::chunk_stream::Lod::Full, false, &[], 1024);
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
            Ok(()) => {
                // Debug HUD: update stats + draw top-right, then present.
                if let Some(hud) = &mut self.hud {
                    let dt = self.last_frame.elapsed().as_secs_f32().clamp(1e-4, 0.1);
                    let fps = 1.0 / dt;
                    let p = self.player.pos;
                    let yaw_deg = self.yaw.to_degrees();
                    let mode = match self.mode {
                        voxel_player::PlayerMode::Walk => "WALK",
                        voxel_player::PlayerMode::Fly => "FLY",
                    };
                    hud.update(
                        scene.device(),
                        scene.queue(),
                        fps,
                        p,
                        yaw_deg,
                        self.mesh_cache.len(),
                        tris.len() / 3,
                        self.seed,
                        mode,
                        self.time_of_day,
                        self.surf_w,
                        self.surf_h,
                    );
                    let mut enc = scene.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("hud-enc"),
                        ..Default::default()
                    });
                    hud.draw(&mut enc, &view);
                    #[cfg(feature = "tracy")]
                    let _gpu_span = span!("gpu_submit");
                    scene.queue().submit(Some(enc.finish()));
                    }
                #[cfg(feature = "tracy")]
                {
                    // Live Tracy plots: key streaming/perf metrics surfaced in the profiler UI.
                    let frame_dt = self.last_frame.elapsed().as_secs_f32().clamp(1e-4, 0.1);
                    plot!("fps", (1.0 / frame_dt) as f64);
                    plot!("chunks", self.mesh_cache.len() as f64);
                    plot!("tris", tris.len() as f64 / 3.0);
                }
                scene.queue().present(tex);
                // Autonomous perf telemetry: Hermes reads profile_metrics.log to track
                // regressions without the Tracy GUI. One sample/sec, feature-independent.
                if self.perf_log_timer.elapsed().as_secs_f32() >= 1.0 {
                    self.perf_log_timer = std::time::Instant::now();
                    let dt = self.last_frame.elapsed().as_secs_f32().clamp(1e-4, 0.1);
                    let fps = 1.0 / dt;
                    let _ = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("profile_metrics.log")
                        .and_then(|mut f| {
                            use std::io::Write;
                            writeln!(
                                f,
                                "ts={:.0} fps={:.1} chunks={} tris={} frame_ms={:.2}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0),
                                fps,
                                self.mesh_cache.len(),
                                tris.len() / 3,
                                dt * 1000.0,
                            )
                        });
                }
            }
            Err(err) => log::error!(
                "gpu_window: render failed (tris={}, surface={}x{}): {err:#}",
                tris.len(),
                self.surf_w,
                self.surf_h
            ),
        }
    }
    /// I1 (live edit): ray-cast from the camera through the look direction, then place or
    /// remove a voxel at the hit. Re-meshes the affected chunk(s) from the edited `World` so
    /// the change is visible immediately. Marks the chunk edited so the streaming worker does
    /// not overwrite it with fresh worldgen.
    fn edit_at_look(&mut self, place: bool) {
        let eye_m = self.camera.eye;
        // Eye in voxel units. floor() is vereist: negatieve coords zouden anders naar 0
        // trunken (i64-cast van een negatief float is truncation, niet floor).
        let origin = WorldVoxel::new(
            (eye_m[0] / VOXEL_SIZE_M).floor() as i64,
            (eye_m[1] / VOXEL_SIZE_M).floor() as i64,
            (eye_m[2] / VOXEL_SIZE_M).floor() as i64,
        );
        // Look direction from yaw/pitch (matches the fly-mode forward vector).
        let (sy, cy) = self.pitch.sin_cos();
        let (sp, cp) = self.yaw.sin_cos();
        let dir = [cp * cy, sy, sp * cy];
        let max_dist = 200.0 / VOXEL_SIZE_M; // 200 m reach
        let Some((hit, normal)) = raycast_voxel(&mut self.world, origin, dir, max_dist) else {
            return; // nothing hit within reach
        };
        if place {
            // Place on the empty face in front of the hit (hit + normal).
            let target = WorldVoxel::new(hit.x + normal.x, hit.y + normal.y, hit.z + normal.z);
            self.edit_tool.place(&mut self.world, target, MaterialId::from(3), 1, self.frame);
        } else {
            self.edit_tool.remove(&mut self.world, hit, 1, self.frame);
        }
        // Re-mesh every chunk touched by the edit (hit + placed neighbour) from the edited
        // World, and mark them edited so the streaming worker won't clobber them. Also re-mesh
        // the 6 face-neighbours of each edited chunk so edits on a chunk boundary show correct
        // (no missing faces / holes at the seam).
        let mut to_remesh: std::collections::HashSet<ChunkCoord> =
            self.world.take_dirty();
        let mut with_neighbours = to_remesh.clone();
        for c in &to_remesh {
            for dx in -1..=1i64 {
                for dy in -1..=1i64 {
                    for dz in -1..=1i64 {
                        if dx == 0 && dy == 0 && dz == 0 {
                            continue;
                        }
                        // Only the 6 face-neighbours (not the 20 edge/corner neighbours).
                        if dx.abs() + dy.abs() + dz.abs() != 1 {
                            continue;
                        }
                        with_neighbours.insert(ChunkCoord::new(c.x + dx, c.y + dy, c.z + dz));
                    }
                }
            }
        }
        for coord in with_neighbours {
            self.edited.insert(coord);
            let chunk = self.world.get_or_generate(coord);
            let tris = voxel_gpu::mesh_chunk_world_meters(
                &chunk,
                voxel_gpu::chunk_stream::Lod::Full,
                false,
                &[],
                1024,
            );
            self.mesh_cache.insert(coord, tris, self.frame);
        }
    }
}

/// Start the live client: build the app, the winit event loop, and run until the window
/// closes. Kept thin on purpose — all client logic lives in `App` (this `lib.rs`).
pub fn run() {
    env_logger::init();
    // Start the Tracy client if the `tracy` feature is enabled (no-op otherwise).
    crate::profiling::start();
    println!(
        "Land of the Voxel Engine — micro-voxel client (12.5 cm/voxel, {} m chunks, view radius {} chunks ~{:.0} m)",
        CHUNK_M, VIEW_RADIUS, VIEW_RADIUS as f32 * CHUNK_M
    );
    println!("WASD = move, Space = jump/up, Left-drag = look, F = toggle walk/fly, Right = place block, Middle = remove block. Close window to exit.");
    let mut app = App::default();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut app).expect("run app");
}
