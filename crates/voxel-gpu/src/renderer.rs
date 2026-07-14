//! Real voxel GPU renderer (S-10/S-12b): greedy-mesh triangles -> wgpu on the GPU.
//!
//! Renders voxel chunks with per-normal directional lighting, warm sky/fog (Lay of the Land
//! vibe) and warm material tints. Two render targets share one pipeline:
//!   - offscreen render-to-PNG (headless / CLI), and
//!   - a live winit window surface (Fase-2 interactive client).
//! The pipeline format is chosen lazily once the target format is known (offscreen = Rgba8Unorm,
//! window = the surface's preferred format).

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use voxel_core::palette::MaterialId;
use voxel_mesher::Triangle;
use wgpu::util::DeviceExt;

/// Per-vertex data uploaded to the GPU.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub material: u32,
}

/// Camera uniforms (view-projection + params) for the shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub fog_color: [f32; 4],
    pub params: [f32; 4], // x = fog_density
    pub eye_pos: [f32; 4], // xyz = camera eye (fog distance reference), w unused
}

/// Minimal perspective camera (matches voxel-render conventions for reuse of the math).
pub struct GpuCamera {
    pub eye: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl GpuCamera {
    pub fn new(eye: [f32; 3], yaw: f32, pitch: f32, aspect: f32) -> Self {
        Self {
            eye,
            yaw,
            pitch,
            fov_y: std::f32::consts::FRAC_PI_3,
            aspect,
            near: 0.1,
            far: 1000.0,
        }
    }

    /// Build the view-projection matrix using glam (WebGPU clip space, z in [0,1]).
    pub fn view_proj(&self) -> [[f32; 4]; 4] {
        let eye = glam::Vec3::new(self.eye[0], self.eye[1], self.eye[2]);
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let fwd = glam::Vec3::new(cy * cp, sp, sy * cp);
        let target = eye + fwd;
        let view = glam::Mat4::look_at_rh(eye, target, glam::Vec3::new(0.0, 1.0, 0.0));
        let proj = glam::Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far);
        let vp = proj * view;
        vp.to_cols_array_2d()
    }
}

/// Warm material tints (Lay of the Land vibe), indexed by material id 0..=15.
/// Canonical ids follow voxel-worldgen: 1 = DIRT, 2 = GRASS, 3 = STONE.
pub fn material_tint(mat: MaterialId) -> [f32; 3] {
    match mat.0 {
        0 => [0.0, 0.0, 0.0],        // air
        1 => [0.52, 0.36, 0.22],     // dirt (warm brown)
        2 => [0.42, 0.62, 0.28],     // grass (warm green)
        3 => [0.50, 0.50, 0.52],     // stone (cool grey)
        4 => [0.78, 0.80, 0.85],     // metal (light steel)
        5 => [0.45, 0.30, 0.18],     // wood (dark warm)
        6 => [0.30, 0.55, 0.25],     // leaf (green)
        7 => [0.85, 0.78, 0.55],     // sand (warm)
        _ => [0.6, 0.6, 0.65],       // fallback
    }
}

/// Owns the GPU device/queue and the voxel pipeline. The pipeline is built lazily once the
/// target texture format is known (offscreen vs window surface can differ).
pub struct GpuScene {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    camera_buf: wgpu::Buffer,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl GpuScene {
    /// Shared device/queue/adapter bootstrap (no surface yet — usable headless).
    async fn bootstrap() -> anyhow::Result<(Arc<wgpu::Device>, Arc<wgpu::Queue>)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .expect("no adapter (GPU unavailable?)");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                    label: None,
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await
            .map_err(|e| anyhow::anyhow!("no device: {e:?}"))?;
        Ok((Arc::new(device), Arc::new(queue)))
    }

    fn build_pipeline(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("voxel-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(VOXEL_WGSL)),
        });
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("voxel-pipeline-layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let vbuf_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: 24,
                    shader_location: 2,
                },
            ],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("voxel-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vbuf_layout)],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                // Mesher emits CCW-from-outside winding (S-11), so cull back faces.
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        (pipeline, bind_group_layout)
    }

    fn make_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Initialize an offscreen (headless) GPU scene that renders to PNG at the given size.
    pub async fn new_offscreen(width: u32, height: u32) -> anyhow::Result<Self> {
        let (device, queue) = Self::bootstrap().await?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let (pipeline, bind_group_layout) = Self::build_pipeline(&device, format);
        let depth_view = Self::make_depth(&device, width, height);
        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            camera_buf,
            depth_view,
            width,
            height,
            format,
        })
    }

    /// The device/queue (used by the winit window path to build a surface pipeline).
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Build a window pipeline variant for the given surface format. The caller must supply
    /// the `device`/`queue` obtained from an adapter that is compatible with the surface
    /// (the surface and device must share the same wgpu `Instance`).
    pub fn new_for_surface(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> anyhow::Result<Self> {
        let (pipeline, bind_group_layout) = Self::build_pipeline(&device, format);
        let depth_view = Self::make_depth(&device, width, height);
        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            camera_buf,
            depth_view,
            width,
            height,
            format,
        })
    }

    /// Upload vertices + camera, run the render pass into `target_view`, then return the
    /// vertex buffer handle (kept alive by the caller for the duration of the pass).
    fn record_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        tris: &[Triangle],
        camera: &GpuCamera,
        target_view: &'a wgpu::TextureView,
    ) -> anyhow::Result<wgpu::Buffer> {
        let mut verts: Vec<GpuVertex> = Vec::with_capacity(tris.len() * 3);
        for t in tris {
            for v in [&t.a, &t.b, &t.c] {
                verts.push(GpuVertex {
                    pos: [v.x, v.y, v.z],
                    normal: [t.normal.x, t.normal.y, t.normal.z],
                    material: t.material.0 as u32,
                });
            }
        }
        if verts.is_empty() {
            anyhow::bail!("no triangles to render");
        }
        let vbuf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("voxel-vbo"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let cu = CameraUniform {
            view_proj: camera.view_proj(),
            fog_color: [0.62, 0.66, 0.74, 1.0],
            params: [0.012, 0.0, 0.0, 0.0],
            eye_pos: [camera.eye[0], camera.eye[1], camera.eye[2], 0.0],
        };
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::cast_slice(&[cu]));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.camera_buf,
                    offset: 0,
                    size: None,
                }),
            }],
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("voxel-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.62,
                            g: 0.66,
                            b: 0.74,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0_f32),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..verts.len() as u32, 0..1);
        }
        Ok(vbuf)
    }

    /// Render triangles to a PNG file (offscreen path — unchanged behaviour from S-10).
    pub async fn render_triangles_png(
        &self,
        tris: &[Triangle],
        camera: &GpuCamera,
        path: &str,
    ) -> anyhow::Result<()> {
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("color-target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("voxel-enc"),
            });
        self.record_pass(&mut encoder, tris, camera, &target_view)?;
        self.queue.submit(Some(encoder.finish()));

        // Read back.
        let bytes_per_row = (self.width * 4).next_multiple_of(256);
        let buf_size = bytes_per_row as u64 * self.height as u64;
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: buf_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc2 = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback-enc"),
            });
        enc2.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(enc2.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = self
            .device
            .poll(wgpu::PollType::wait_indefinitely());
        rx.recv()
            .map_err(|_| anyhow::anyhow!("map channel closed"))?
            .map_err(|e| anyhow::anyhow!("map failed: {e:?}"))?;
        let data = slice.get_mapped_range()?;
        let mut img = image::RgbaImage::new(self.width, self.height);
        let is_bgra = matches!(self.format, wgpu::TextureFormat::Bgra8Unorm);
        for y in 0..self.height {
            for x in 0..self.width {
                let i = (y * bytes_per_row + x * 4) as usize;
                let [r, g, b, a] = [data[i], data[i + 1], data[i + 2], data[i + 3]];
                if is_bgra {
                    img.put_pixel(x, y, image::Rgba([b, g, r, a]));
                } else {
                    img.put_pixel(x, y, image::Rgba([r, g, b, a]));
                }
            }
        }
        drop(data);
        staging.unmap();
        img.save(path)?;
        Ok(())
    }

    /// Render triangles into an existing surface texture view (window path).
    pub fn render_to_view(
        &self,
        tris: &[Triangle],
        camera: &GpuCamera,
        surface_view: &wgpu::TextureView,
    ) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("voxel-window-enc"),
            });
        self.record_pass(&mut encoder, tris, camera, surface_view)?;
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

const VOXEL_WGSL: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    params: vec4<f32>,
    eye_pos: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: CameraUniform;

struct VtxIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) material: u32,
};
struct VtxOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) world_pos: vec3<f32>,
};

@vertex
fn vs_main(in: VtxIn) -> VtxOut {
    var o: VtxOut;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.material = in.material;
    o.world_pos = in.pos;
    return o;
}

@fragment
fn fs_main(in: VtxOut) -> @location(0) vec4<f32> {
    let base = mat_tint(in.material);
    let L = normalize(vec3<f32>(0.4, 0.9, 0.3));
    let n = normalize(in.normal);
    let diff = max(dot(n, L), 0.0);
    let ambient = 0.45;
    var col = base * (ambient + 0.75 * diff);
    col = mix(col, col * vec3<f32>(1.05, 0.98, 0.90), 0.3);
    let dist = length(in.world_pos - cam.eye_pos.xyz);
    let fog = 1.0 - exp(-cam.params.x * dist);
    col = mix(col, cam.fog_color.xyz, clamp(fog, 0.0, 0.85));
    return vec4<f32>(col, 1.0);
}

fn mat_tint(id: u32) -> vec3<f32> {
    if (id == 1u) { return vec3<f32>(0.52, 0.36, 0.22); }
    if (id == 2u) { return vec3<f32>(0.42, 0.62, 0.28); }
    if (id == 3u) { return vec3<f32>(0.50, 0.50, 0.52); }
    if (id == 4u) { return vec3<f32>(0.78, 0.80, 0.85); }
    if (id == 5u) { return vec3<f32>(0.45, 0.30, 0.18); }
    if (id == 6u) { return vec3<f32>(0.30, 0.55, 0.25); }
    if (id == 7u) { return vec3<f32>(0.85, 0.78, 0.55); }
    return vec3<f32>(0.6, 0.6, 0.65);
}
"#;
