//! S-12b demo: interactive winit window rendering the voxel world on the GPU (wgpu 30).
//!
//! Opens a live window, meshes a chunk block, and renders it with a free-fly camera
//! controlled by WASD + mouse-look. Proves the engine runs interactively on the GPU.
//!
//! Run with: cargo run --example gpu_window -p voxel-gpu

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

use voxel_core::coords::ChunkCoord;
use voxel_gpu::renderer::{GpuCamera, GpuScene};
use voxel_mesher::greedy_mesh;
use voxel_world::World;

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    scene: Option<GpuScene>,
    tris: Vec<voxel_mesher::Triangle>,
    camera: GpuCamera,
    // Input state.
    keys: std::collections::HashSet<winit::keyboard::PhysicalKey>,
    yaw: f32,
    pitch: f32,
    // Mouse-look drag state.
    dragging: bool,
    last_mouse: Option<(f64, f64)>,
    // Max texture dimension of the adapter (surface size must stay within this).
    max_dim: u32,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            scene: None,
            tris: Vec::new(),
            camera: GpuCamera::new([16.0, 55.0, 90.0], -std::f32::consts::FRAC_PI_2, -0.5, 1.0),
            keys: std::collections::HashSet::new(),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: -0.5,
            dragging: false,
            last_mouse: None,
            max_dim: 2048,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Land of the Voxel Engine — GPU client")
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 768.0));
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
        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            },
        ))
        .expect("no adapter");
        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                label: None,
                trace: wgpu::Trace::Off,
            },
        ))
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

        // Clip the surface size to a safe maximum. The default downlevel surface on this
        // adapter does not accept textures larger than 2048 per dimension (DPI scaling can
        // push the physical window size well past that and make Surface::configure panic).
        let max_dim: u32 = 2048;
        let surf_w = size.width.min(max_dim);
        let surf_h = size.height.min(max_dim);
        self.max_dim = max_dim;

        let scene = GpuScene::new_for_surface(
            device,
            queue,
            surf_w,
            surf_h,
            format,
        )
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

        self.window = Some(window);
        self.surface = Some(surface);
        self.scene = Some(scene);
        // Start the render loop.
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
                if let (Some(scene), Some(surface)) = (&self.scene, &self.surface) {
                    let config = wgpu::SurfaceConfiguration {
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        format: scene.format(),
                        width: size.width.max(1).min(self.max_dim),
                        height: size.height.max(1).min(self.max_dim),
                        present_mode: wgpu::PresentMode::Fifo,
                        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                        view_formats: vec![],
                        desired_maximum_frame_latency: 2,
                        color_space: wgpu::SurfaceColorSpace::Auto,
                    };
                    surface.configure(scene.device(), &config);
                    self.camera.aspect = size.width as f32 / size.height as f32;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
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
        // Free-fly movement from WASD along the camera's forward/right vectors.
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let forward = [cy * cp, sp, sy * cp];
        let right = [cy, 0.0, sy];
        let speed = 0.6;
        if self.keys.contains(&winit::keyboard::PhysicalKey::Code(
            winit::keyboard::KeyCode::KeyW,
        )) {
            self.camera.eye[0] += forward[0] * speed;
            self.camera.eye[1] += forward[1] * speed;
            self.camera.eye[2] += forward[2] * speed;
        }
        if self.keys.contains(&winit::keyboard::PhysicalKey::Code(
            winit::keyboard::KeyCode::KeyS,
        )) {
            self.camera.eye[0] -= forward[0] * speed;
            self.camera.eye[1] -= forward[1] * speed;
            self.camera.eye[2] -= forward[2] * speed;
        }
        if self.keys.contains(&winit::keyboard::PhysicalKey::Code(
            winit::keyboard::KeyCode::KeyD,
        )) {
            self.camera.eye[0] += right[0] * speed;
            self.camera.eye[2] += right[2] * speed;
        }
        if self.keys.contains(&winit::keyboard::PhysicalKey::Code(
            winit::keyboard::KeyCode::KeyA,
        )) {
            self.camera.eye[0] -= right[0] * speed;
            self.camera.eye[2] -= right[2] * speed;
        }
    }

    fn render_frame(&mut self) {
        let (Some(scene), Some(surface)) = (&self.scene, &self.surface) else {
            return;
        };
        let frame = surface.get_current_texture();
        let tex = match frame {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return,
        };
        let view = tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
        if scene.render_to_view(&self.tris, &self.camera, &view).is_ok() {
            scene.queue().present(tex);
        }
    }
}

fn main() {
    env_logger::init();
    // Build the world + mesh once.
    let mut world = World::new(7);
    let mut tris = Vec::new();
    for cx in 0..2i64 {
        for cz in 0..2i64 {
            let coord = ChunkCoord::new(cx, 0, cz);
            let chunk = world.get_or_generate(coord);
            for t in greedy_mesh(&chunk) {
                tris.push(t);
            }
        }
    }
    println!("meshed {} triangles across 4 chunks", tris.len());

    let mut app = App::default();
    app.tris = tris;

    let event_loop = EventLoop::new().expect("event loop");
    event_loop.run_app(&mut app).expect("run app");
}
