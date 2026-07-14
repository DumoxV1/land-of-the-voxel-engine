//! Real voxel GPU renderer (S-10): greedy-mesh triangles -> wgpu on the GPU.
//!
//! Renders voxel chunks with per-normal directional lighting, warm sky/fog (Lay of the Land
//! vibe) and warm material tints. Offscreen render-to-PNG so the engine runs on the GPU
//! without a window; a winit window can be added later.

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

/// Warm material tints (Lay of the Land vibe): grass, dirt, stone, metal, wood, leaf, ...
/// Indexed by material id 0..=15. Air (0) is unused.
pub fn material_tint(mat: MaterialId) -> [f32; 3] {
    match mat.0 {
        0 => [0.0, 0.0, 0.0],        // air
        1 => [0.42, 0.62, 0.28],     // grass (warm green)
        2 => [0.52, 0.36, 0.22],     // dirt (warm brown)
        3 => [0.50, 0.50, 0.52],     // stone (cool grey)
        4 => [0.78, 0.80, 0.85],     // metal (light steel)
        5 => [0.45, 0.30, 0.18],     // wood (dark warm)
        6 => [0.30, 0.55, 0.25],     // leaf (green)
        7 => [0.85, 0.78, 0.55],     // sand (warm)
        _ => [0.6, 0.6, 0.65],       // fallback
    }
}

/// A wgpu-based scene renderer. Builds the pipeline once, then renders vertex slices.
pub struct GpuScene {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    camera_buf: wgpu::Buffer,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
}

impl GpuScene {
    /// Initialize the GPU scene for an offscreen target of the given size.
    pub async fn new(width: u32, height: u32) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("no adapter (GPU/Vulkan unavailable?)"))?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                    label: None,
                },
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("no device: {e:?}"))?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("voxel-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(VOXEL_WGSL)),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("voxel-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("voxel-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
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
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let depth_view = device
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
            .create_view(&wgpu::TextureViewDescriptor::default());

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            camera_buf,
            depth_view,
            width,
            height,
        })
    }

    /// Render a list of triangles (already in world space) with the given camera to a PNG.
    pub async fn render_triangles_png(
        &self,
        tris: &[Triangle],
        camera: &GpuCamera,
        path: &str,
    ) -> anyhow::Result<()> {
        // Upload vertices.
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

        // Camera uniform.
        let cu = CameraUniform {
            view_proj: camera.view_proj(),
            fog_color: [0.62, 0.66, 0.74, 1.0],
            params: [0.012, 0.0, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.camera_buf, 0, bytemuck::cast_slice(&[cu]));

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.camera_buf.as_entire_binding(),
            }],
        });

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
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("voxel-enc"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("voxel-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.62,
                            g: 0.66,
                            b: 0.74,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0_f32),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..verts.len() as u32, 0..1);
        }
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
            wgpu::ImageCopyTexture {
                texture: &target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout {
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
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range();
        let mut img = image::RgbaImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let i = (y * bytes_per_row + x * 4) as usize;
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([data[i], data[i + 1], data[i + 2], data[i + 3]]),
                );
            }
        }
        drop(data);
        staging.unmap();
        img.save(path)?;
        Ok(())
    }
}

const VOXEL_WGSL: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    fog_color: vec4<f32>,
    params: vec4<f32>,
};
@group(0) @binding(0) var<uniform> cam: CameraUniform;

struct VtxIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) material: u32,
};
struct VtxOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) material: u32,
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
    // Warm key light from upper-front.
    let L = normalize(vec3<f32>(0.4, 0.9, 0.3));
    let n = normalize(in.normal);
    let diff = max(dot(n, L), 0.0);
    let ambient = 0.45;
    var col = base * (ambient + 0.75 * diff);
    // Warm tint at top, cool in shadows.
    col = mix(col, col * vec3<f32>(1.05, 0.98, 0.90), 0.3);
    // Distance fog for depth/atmosphere.
    let dist = length(in.world_pos - vec3<f32>(0.0, 0.0, 0.0));
    let fog = 1.0 - exp(-cam.params.x * dist);
    col = mix(col, cam.fog_color.xyz, clamp(fog, 0.0, 0.85));
    return vec4<f32>(col, 1.0);
}

fn mat_tint(id: u32) -> vec3<f32> {
    if (id == 1u) { return vec3<f32>(0.42, 0.62, 0.28); }
    if (id == 2u) { return vec3<f32>(0.52, 0.36, 0.22); }
    if (id == 3u) { return vec3<f32>(0.50, 0.50, 0.52); }
    if (id == 4u) { return vec3<f32>(0.78, 0.80, 0.85); }
    if (id == 5u) { return vec3<f32>(0.45, 0.30, 0.18); }
    if (id == 6u) { return vec3<f32>(0.30, 0.55, 0.25); }
    if (id == 7u) { return vec3<f32>(0.85, 0.78, 0.55); }
    return vec3<f32>(0.6, 0.6, 0.65);
}
"#;
