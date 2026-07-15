//! S-12b / S-13 live GPU client: stream a micro-voxel world (12.5 cm/voxel) around
//! a first-person free-fly camera (WASD + mouse-look). Chunks within `VIEW_RADIUS`
//! of the camera are generated + meshed on the fly (chunk-streaming), so you can
//! walk/fly through a real, open world — not a 2x2 stub.
//!
//! Run with: cargo run --release --example gpu_window -p voxel-gpu

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use voxel_core::coords::{chunk_m_size, ChunkCoord, CHUNK_SIZE, VOXEL_SIZE_M};
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_gpu::{mesh_chunk_world_meters, mesh_pool, spawn_eye_y_m, MeshResult};
use voxel_mesher::Triangle;
use voxel_world::World;

/// View distance in chunks. On the 12.5 cm scale a 4 m chunk -> 32 chunks ~= 128 m view.
const CHUNK_M: f32 = CHUNK_SIZE as f32 * VOXEL_SIZE_M; // 4 m (ADR-0005)
const VIEW_RADIUS: i64 = 24; // ~96 m view radius
/// Max chunks whose meshes we ingest from the worker channel per frame (P3 upload budget).
const UPLOAD_BUDGET: usize = 4;

/// Find the chunk nearest the camera (Manhattan distance in chunk space) that passes the
/// frustum test — used by the "never go white" guard to seed at least one mesh on frame 1.
/// Free function (no `&self`) so it can run while `scene` holds a `&mut self.scene` borrow.
fn nearest_visible_chunk(
    view_proj: &[[f32; 4]; 4],
    half: f32,
    half_y: f32,
    ccx: i64,
    ccz: i64,
) -> Option<ChunkCoord> {
    const MAX_Y: i64 = 12;
    let frustum = voxel_gpu::renderer::Frustum::from_view_proj(view_proj);
    let mut best: Option<(i64, ChunkCoord)> = None;
    for dx in -VIEW_RADIUS..=VIEW_RADIUS {
        for dz in -VIEW_RADIUS..=VIEW_RADIUS {
            let cx = ccx + dx;
            let cz = ccz + dz;
            // Negative chunk coords are valid (i64 + Euclidean div) — do not skip.
            for cy in 0..=MAX_Y {
                let center = [
                    (cx as f32 + 0.5) * CHUNK_M,
                    (cy as f32 * CHUNK_M) + half_y * 0.5,
                    (cz as f32 + 0.5) * CHUNK_M,
                ];
                if !frustum.intersects_aabb(center, half.max(half_y)) {
                    continue;
                }
                let dist = dx.abs() + dz.abs();
                if best.map_or(true, |(bd, _)| dist < bd) {
                    best = Some((dist, ChunkCoord::new(cx, cy, cz)));
                }
            }
        }
    }
    best.map(|(_, c)| c)
}

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
    requested_gen: HashMap<ChunkCoord, u64>,
    pending: HashSet<ChunkCoord>,
    camera: GpuCamera,
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
            requested_gen: HashMap::new(),
            pending: HashSet::new(),
            // First-person spawn: eye height set after we know the terrain in resumed().
            camera: GpuCamera::new([40.0, 50.0, 40.0], -std::f32::consts::FRAC_PI_2, -0.4, 1.0),
            keys: HashSet::new(),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.4,
            dragging: false,
            last_mouse: None,
            max_dim: 2048,
            surf_w: 1280,
            surf_h: 800,
            last_frame: std::time::Instant::now(),
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
                required_limits: wgpu::Limits::downlevel_defaults(),
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
        // Use the canonical surface height (world-Y meters) so we spawn above real peaks.
        let spawn = ChunkCoord::new(1, 0, 1);
        let center_wx = (spawn.x * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
        let center_wz = (spawn.z * voxel_core::coords::CHUNK_SIZE + voxel_core::coords::CHUNK_SIZE / 2) as i64;
        let surface_m = voxel_worldgen::surface_height_m(center_wx, center_wz, self.seed);
        let top_vox = (surface_m / 0.125) as i64;
        // Eye ~120 voxels (15 m) above the surface so we look *over* the terrain,
        // not into a cliff face (vertical-scale spike: peaks reach ~40 m now).
        let eye_x = 1.5 * chunk_m_size();
        let eye_z = 1.5 * chunk_m_size();
        self.camera.eye = [eye_x, spawn_eye_y_m(top_vox, 120), eye_z];
        println!(
            "spawn: terrain top = {} voxels (~{:.2} m), eye_y = {:.2} m",
            top_vox,
            surface_m,
            spawn_eye_y_m(top_vox, 120)
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
        // Frame-rate independent free-fly: integrate movement with a real dt (seconds) so
        // speed is in world-m/s regardless of FPS. Without dt, WASD added a fixed step every
        // frame → "super fast at high FPS". See `voxel_gpu::free_fly_step` (unit-tested).
        let now = std::time::Instant::now();
        let dt = self.last_frame.elapsed().as_secs_f32().clamp(0.0, 0.1);
        self.last_frame = now;
        // Build the key bitmask: W=1, S=2, D=4, A=8.
        let mut keys = 0u8;
        if self
            .keys
            .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyW))
        {
            keys |= 1;
        }
        if self
            .keys
            .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyS))
        {
            keys |= 2;
        }
        if self
            .keys
            .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyD))
        {
            keys |= 4;
        }
        if self
            .keys
            .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::KeyA))
        {
            keys |= 8;
        }
        // Comfortable fly speed on the 12.5 cm scale: 8 m/s base, Shift = 4x sprint.
        let mut speed = 8.0;
        if self
            .keys
            .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftLeft))
            || self
                .keys
                .contains(&winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ShiftRight))
        {
            speed *= 4.0;
        }
        self.camera.eye =
            voxel_gpu::free_fly_step(self.camera.eye, self.yaw, self.pitch, dt, speed, keys);
    }

    fn render_frame(&mut self) {
        self.frame += 1;
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
            if self.requested_gen.get(&r.coord).copied() != Some(r.gen) {
                continue;
            }
            self.mesh_cache.insert(r.coord, r.tris, self.frame);
            self.pending.remove(&r.coord);
        }

        // --- Chunk-streaming: draw visible chunks; request missing ones off-thread. ---
        let mut tris: Vec<Triangle> = Vec::new();
        let [ex, _ey, ez] = self.camera.eye;
        let ccx = (ex / CHUNK_M).floor() as i64;
        let ccz = (ez / CHUNK_M).floor() as i64;
        let half = CHUNK_M * 0.5; // 2 m half-extent (x/z)
        let half_y = 24.0; // terrain peaks ~40 m; pad for height + camera clearance
        const MAX_Y: i64 = 12; // stream chunks y=0..=12 (~48 m of vertical world)
        let frustum = voxel_gpu::renderer::Frustum::from_view_proj(&self.camera.view_proj());
        for dx in -VIEW_RADIUS..=VIEW_RADIUS {
            for dz in -VIEW_RADIUS..=VIEW_RADIUS {
                let cx = ccx + dx;
                let cz = ccz + dz;
                // NOTE: negative chunk coords are valid (ChunkCoord is i64 + Euclidean div).
                // Do NOT skip them — skipping caused the "white screen when flying into
                // negative space" bug.
                for cy in 0..=MAX_Y {
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
                        let slice = m.tris.clone();
                        tris.extend_from_slice(&slice); // ready: draw (frustum-cull intact)
                        // Mark recently visible (separate mutable borrow, after the immutable read).
                        self.mesh_cache.touch(&coord, self.frame);
                    } else if !self.pending.contains(&coord) {
                    // Not ready and not yet requested: spawn off-thread generate+mesh.
                    let g = self.requested_gen.entry(coord).or_insert(0);
                    *g += 1;
                    let gen = *g;
                    self.pending.insert(coord);
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

        // Frame-1 fallback only: seed one visible chunk while async jobs are still pending.
        if tris.is_empty() {
            let vp = self.camera.view_proj();
            if let Some(coord) = nearest_visible_chunk(&vp, half, half_y, ccx, ccz) {
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
        match scene.render_to_view(&tris, &self.camera, &view) {
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
    println!("WASD = fly, Left-drag = look. Close window to exit.");
    let mut app = App::default();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut app).expect("run app");
}
