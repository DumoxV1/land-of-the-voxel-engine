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

/// Max VBO bytes the renderer will allocate for the streamed terrain mesh (P0 spike,
/// 2026-07-15). Raised from the legacy 256 MB to 2 GB so the vertical-scale terrain
/// (multi-chunk-Y, thousands of chunks) draws without truncation. Must be mirrored in
/// `required_limits.max_buffer_size` at device creation (see gpu_window.rs).
pub const MAX_VBO_BYTES: usize = 2 * 1024 * 1024 * 1024;

/// Per-vertex data uploaded to the GPU.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub material: u32,
    pub ao: [f32; 3],
    pub sun: [f32; 3],
}

/// Camera uniforms (view-projection + params) for the shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub fog_color: [f32; 4],
    pub params: [f32; 4],  // x = fog_density, y = time_of_day (0..1, F2 dag/nacht), z/w reserved
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

/// Per-material albedo tile resolution in pixels (Taak 5: 16 -> 1024 = echte 4K-scale
/// detail per materiaal; 9 materialen * 1024² * 4 B ≈ 36 MB, binnen VRAM).
pub(crate) const TEXTURE_TILE: u32 = 1024;

/// Warm material tints (Lay of the Land vibe), indexed by material id 0..=15.
/// Canonical ids follow voxel-worldgen: 1 = DIRT, 2 = GRASS, 3 = STONE.
pub fn material_tint(mat: MaterialId) -> [f32; 3] {
    match mat.0 {
        // Taak 5 (2026-07-15): hoog-verzadigde, warme Lay-of-the-Land-palet. Steen is nu
        // warm beige/grijs (geen koud grijs meer), gras diep groen, aarde rijk bruin.
        0 => [0.0, 0.0, 0.0],     // air
        1 => [0.55, 0.34, 0.18],  // dirt  (rijk bruin)
        2 => [0.26, 0.56, 0.16],  // grass (diep verzadigd groen)
        3 => [0.62, 0.57, 0.48],  // stone (warm beige-grijs, niet koud grijs)
        4 => [0.74, 0.77, 0.82],  // metal (licht staal, koel maar helder)
        5 => [0.50, 0.31, 0.16],  // wood  (donker warm)
        6 => [0.20, 0.48, 0.14],  // leaf  (donker groen)
        7 => [0.90, 0.78, 0.40],  // sand  (goud-geel, verzadigd)
        8 => [0.96, 0.97, 0.99],  // snow   (warm wit)
        _ => [0.64, 0.60, 0.52],  // fallback (warm grijs)
    }
}

/// Per-material PBR parameters, indexed by the flat `material: u32` vertex attribute.
/// Must match the WGSL `MaterialPbr` layout exactly (std140-ish, 3x vec4 = 48 bytes).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaterialPbr {
    pub albedo_tint: [f32; 4],
    pub params: [f32; 4], // x=tiling, y=normal_scale, z=roughness, w=metallic
    pub emissive: [f32; 4],
}

impl MaterialPbr {
    /// Build the default palette from the warm `material_tint` set, with a tiling factor
    /// that gives triplanar projection visible variation on big greedy quads.
    pub fn defaults() -> Vec<MaterialPbr> {
        (0..=8u8)
            .map(|id| {
                let t = material_tint(voxel_core::palette::MaterialId::from(id));
                MaterialPbr {
                    albedo_tint: [t[0], t[1], t[2], 1.0],
                    // tiling: world-meters per texture repeat. Taak 5: 0.25 -> een 4 m tile op
                    // een 4 m quad, zodat de 1024² fBm-detail zichtbaar is (was 0.5 = 2 m).
                    params: [0.25, 1.0, 0.8, 0.0],
                    emissive: [0.0, 0.0, 0.0, 0.0],
                }
            })
            .collect()
    }
}

/// Clamp an i32 to a u8 byte (used when baking material tiles).
fn clamp8(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Owns the GPU device/queue and the voxel pipeline. The pipeline is built lazily once the
/// target texture format is known (offscreen vs window surface can differ).
pub struct GpuScene {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    /// Mijlpaal 4 P0: per-material data + albedo texture array + sampler, bound as group(1).
    material_bg: wgpu::BindGroup,
    camera_buf: wgpu::Buffer,
    depth_view: wgpu::TextureView,
    /// F1 post-FX: linear HDR scene target (rgba16float) the voxel pass renders into.
    hdr_view: wgpu::TextureView,
    /// F1 post-FX: fullscreen filmic pass pipeline (targets the surface/present format).
    post_pipeline: wgpu::RenderPipeline,
    post_bgl: wgpu::BindGroupLayout,
    post_bg: wgpu::BindGroup,
    post_params_buf: wgpu::Buffer,
    post_sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    /// Pooled vertex buffer (S-12c deel 2): reused across frames via
    /// `queue.write_buffer` instead of re-allocating a fresh VBO every frame.
    vbo: Option<wgpu::Buffer>,
    vbo_capacity: usize,
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
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
                label: None,
                experimental_features: wgpu::ExperimentalFeatures::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| anyhow::anyhow!("no device: {e:?}"))?;
        Ok((Arc::new(device), Arc::new(queue)))
    }

    fn build_scene_pipeline(
        device: &wgpu::Device,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
        wgpu::BindGroupLayout,
    ) {
        // Scene pass always renders to a linear HDR target (rgba16float) so the
        // post-FX pass (F1) can tonemap >1 highlights. Surface encoding happens later.
        let format = wgpu::TextureFormat::Rgba16Float;
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
        // Mijlpaal 4 P0: group(1) = material storage buffer + albedo 2D array + sampler.
        let material_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("voxel-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&material_bgl)],
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
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 28,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 40,
                    shader_location: 4,
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
        (pipeline, bind_group_layout, material_bgl)
    }

    /// F1 post-FX: build the fullscreen filmic pass pipeline. Targets `surface_format`
    /// (the swapchain/present format); reads the linear HDR scene target.
    fn build_post_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post-fx-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(POST_WGSL)),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("post-fx-pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_post"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_post"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        (pipeline, bgl)
    }

    /// Linear HDR scene target (rgba16float), reused across frames.
    fn make_hdr_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("hdr-target"),
                size: wgpu::Extent3d {
                    width: width.max(1),
                    height: height.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Build the post-FX bind group (HDR texture view + sampler + params) for the
    /// given HDR target view. Rebuilt on resize since it references the HDR view.
    fn build_post_resources(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        post_bgl: &wgpu::BindGroupLayout,
        hdr_view: &wgpu::TextureView,
    ) -> (wgpu::BindGroup, wgpu::Buffer, wgpu::Sampler) {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("post-params"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        // Filmic defaults: exposure 1.1, saturation 1.15, teal-orange grade 0.6.
        queue.write_buffer(
            &params_buf,
            0,
            bytemuck::cast_slice(&[1.1_f32, 1.15_f32, 0.6_f32, 0.0_f32]),
        );
        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post-bg"),
            layout: post_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(hdr_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &params_buf,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        });
        (bg, params_buf, sampler)
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

    /// Mijlpaal 4 P0: build the material storage buffer, the albedo `texture_2d_array`
    /// (one layer per material, each a small tinted + noise tile so triplanar projection
    /// yields visible surface variation) and an anisotropic sampler. Returns the group(1)
    /// bind group. Kept small (TILE x TILE) on purpose — VRAM scales with #materials, not voxels.
    fn build_material_resources(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        material_bgl: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        let materials = MaterialPbr::defaults();
        let mat_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material-pbr-buf"),
            size: (materials.len() * std::mem::size_of::<MaterialPbr>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&mat_buf, 0, bytemuck::cast_slice(&materials));

        // Taak 5 (2026-07-15): echte 4K-scale albedo-tiles (1024² per materiaal) met
        // meerdere-octaaf fBm-value-noise voor natuurlijke steen/gras/grond-structuur
        // (geen 16² hash-noise meer). Tint = basiskleur, noise = luminantie-variatie.
        // Mipmaps (generate_mipmaps) → scherpe nabij-detail, geen moiré op afstand.
        const TILE: u32 = TEXTURE_TILE;
        let layer_count = materials.len() as u32;
        // fBm value-noise (lokale CPU-implementatie voor tile-gen).
        let hash2 = |x: i32, y: i32, seed: i32| -> f32 {
            // i64 om overflow te voorkomen bij grote TILE (1024²).
            let n = (x as i64 * 374761393 + y as i64 * 668265263 + seed as i64 * 2147483647) as f32;
            let s = (n.sin() * 43758.5453).fract().abs();
            s
        };
        let vnoise = |x: f32, y: f32, seed: i32| -> f32 {
            let xi = x.floor() as i32;
            let yi = y.floor() as i32;
            let xf = x - x.floor();
            let yf = y - y.floor();
            let u = xf * xf * (3.0 - 2.0 * xf);
            let v = yf * yf * (3.0 - 2.0 * yf);
            let a = hash2(xi, yi, seed);
            let b = hash2(xi + 1, yi, seed);
            let c = hash2(xi, yi + 1, seed);
            let d = hash2(xi + 1, yi + 1, seed);
            let ab = a + (b - a) * u;
            let cd = c + (d - c) * u;
            ab + (cd - ab) * v
        };
        let fbm = |mut x: f32, mut y: f32, seed: i32| -> f32 {
            let mut amp = 0.5f32;
            let mut freq = 1.0f32;
            let mut sum = 0.0f32;
            let mut norm = 0.0f32;
            for _ in 0..5 {
                sum += amp * vnoise(x * freq, y * freq, seed);
                norm += amp;
                amp *= 0.5;
                freq *= 2.0;
            }
            sum / norm
        };
        let mut rgba = Vec::with_capacity((TILE * TILE * layer_count) as usize * 4);
        for (li, _m) in materials.iter().enumerate() {
            let mseed = (li as i32) * 1013 + 7;
            // Per-materiaal noise-schaal: steen/fout fijnere korrel, gras/dirt bredere vlekken.
            let scale = if li == 2 || li == 6 { 24.0 } else { 10.0 };
            for y in 0..TILE {
                for x in 0..TILE {
                    let n = fbm(x as f32 / scale, y as f32 / scale, mseed); // 0..1
                    // Grijswaarden-luminantie: de tile bevat ALLEEN structuur (noise), de
                    // shader vermenigvuldigt met `albedo_tint` voor de kleur. Zo wordt de
                    // tint niet dubbel aangebracht (base² → te donker).
                    let lum = (0.72 + 0.42 * n) * 255.0; // 0.72..1.14 -> 184..291
                    let v = clamp8(lum as i32);
                    rgba.extend_from_slice(&[v, v, v, 255]);
                }
            }
        }
        let mip_levels = 1u32; // Taak 5 TODO: echte mipmap-blit voor moiré-vrije afstand; 1024²
                               // tile + anisotropy-16 geeft nu al scherp nabij-detail.
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("albedo-array"),
            size: wgpu::Extent3d {
                width: TILE,
                height: TILE,
                depth_or_array_layers: layer_count,
            },
            mip_level_count: mip_levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(TILE * 4),
                rows_per_image: Some(TILE),
            },
            wgpu::Extent3d {
                width: TILE,
                height: TILE,
                depth_or_array_layers: layer_count,
            },
        );
        let albedo_view = tex.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("material-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material-bg"),
            layout: material_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &mat_buf,
                        offset: 0,
                        size: None,
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&albedo_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        })
    }

    /// Initialize an offscreen (headless) GPU scene that renders to PNG at the given size.
    pub async fn new_offscreen(width: u32, height: u32) -> anyhow::Result<Self> {
        let (device, queue) = Self::bootstrap().await?;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        // Scene pass renders to HDR; post pass writes to the offscreen (PNG) format.
        let (pipeline, bind_group_layout, material_bgl) = Self::build_scene_pipeline(&device);
        let (post_pipeline, post_bgl) = Self::build_post_pipeline(&device, format);
        let material_bg = Self::build_material_resources(&device, &queue, &material_bgl);
        let depth_view = Self::make_depth(&device, width, height);
        let hdr_view = Self::make_hdr_target(&device, width, height);
        let (post_bg, post_params_buf, post_sampler) =
            Self::build_post_resources(&device, &queue, &post_bgl, &hdr_view);
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
            material_bg,
            camera_buf,
            depth_view,
            hdr_view,
            post_pipeline,
            post_bgl,
            post_bg,
            post_params_buf,
            post_sampler,
            width,
            height,
            format,
            vbo: None,
            vbo_capacity: 0,
        })
    }

    /// The device/queue (used by the winit window path to build a surface pipeline).
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Resize all render-target-dependent state together. Surface callers must configure the
    /// surface to the same dimensions immediately after this call.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.depth_view = Self::make_depth(&self.device, self.width, self.height);
        self.hdr_view = Self::make_hdr_target(&self.device, self.width, self.height);
        let (post_bg, post_params_buf, post_sampler) = Self::build_post_resources(
            &self.device,
            &self.queue,
            &self.post_bgl,
            &self.hdr_view,
        );
        self.post_bg = post_bg;
        self.post_params_buf = post_params_buf;
        self.post_sampler = post_sampler;
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
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
        let (pipeline, bind_group_layout, material_bgl) = Self::build_scene_pipeline(&device);
        let (post_pipeline, post_bgl) = Self::build_post_pipeline(&device, format);
        let material_bg = Self::build_material_resources(&device, &queue, &material_bgl);
        let depth_view = Self::make_depth(&device, width, height);
        let hdr_view = Self::make_hdr_target(&device, width, height);
        let (post_bg, post_params_buf, post_sampler) =
            Self::build_post_resources(&device, &queue, &post_bgl, &hdr_view);
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
            material_bg,
            camera_buf,
            depth_view,
            hdr_view,
            post_pipeline,
            post_bgl,
            post_bg,
            post_params_buf,
            post_sampler,
            width,
            height,
            format,
            vbo: None,
            vbo_capacity: 0,
        })
    }

    /// Upload vertices + camera, run the render pass into `target_view`, then return the
    /// vertex buffer handle (kept alive by the caller for the duration of the pass).
    fn record_pass<'a>(
        &'a mut self,
        encoder: &'a mut wgpu::CommandEncoder,
        tris: &[Triangle],
        camera: &GpuCamera,
        target_view: &'a wgpu::TextureView,
        time_of_day: f32,
    ) -> anyhow::Result<wgpu::Buffer> {
        let mut verts: Vec<GpuVertex> = Vec::with_capacity(tris.len() * 3);
        for t in tris {
            for v in [&t.a, &t.b, &t.c] {
                verts.push(GpuVertex {
                    pos: [v.x, v.y, v.z],
                    normal: [t.normal.x, t.normal.y, t.normal.z],
                    material: t.material.0 as u32,
                    ao: t.ao,
                    sun: t.sun,
                });
            }
        }
        if verts.is_empty() {
            anyhow::bail!("no triangles to render");
        }
        // --- Buffer pooling (S-12c deel 2): reuse one VBO across frames via
        // write_buffer instead of re-allocating a fresh buffer every frame.
        let needed = verts.len() * std::mem::size_of::<GpuVertex>();
        let max_buf = MAX_VBO_BYTES.min(self.device.limits().max_buffer_size as usize);
        let vbuf = match &self.vbo {
            Some(b) if b.size() >= needed as u64 => b.clone(),
            _ => {
                // Grow to the smallest multiple of 1 MiB that covers `needed`, capped to
                // `max_buf`. If a single frame needs more than `max_buf`, clamp to `max_buf`
                // and the draw will simply be truncated rather than panic the app.
                let cap = if needed <= max_buf {
                    (needed.max(1 << 20) * 2).next_multiple_of(1 << 20)
                } else {
                    max_buf
                }
                .min(max_buf) as u64;
                let b = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("voxel-vbo-pool"),
                    size: cap,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.vbo_capacity = cap as usize;
                self.vbo = Some(b.clone());
                b
            }
        };
        // Hard cap the uploaded/drawn vertices to the buffer capacity so an oversized
        // terrain frame can never overrun write_buffer (was a launch panic once the
        // vertical-scale spike pushed the streamed mesh set past 256 MB).
        let cap_verts = self.vbo_capacity / std::mem::size_of::<GpuVertex>();
        if verts.len() > cap_verts {
            log::warn!(
                "VBO budget exceeded: {} verts > {} cap, truncating draw (raise VBO cap or add LOD)",
                verts.len(),
                cap_verts
            );
            verts.truncate(cap_verts);
        }
        self.queue
            .write_buffer(&vbuf, 0, bytemuck::cast_slice(&verts));

        let cu = CameraUniform {
            view_proj: camera.view_proj(),
            fog_color: [0.62, 0.66, 0.74, 1.0],
            params: [0.012, time_of_day, 0.0, 0.0],
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
            pass.set_bind_group(1, &self.material_bg, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..verts.len() as u32, 0..1);
        }
        Ok(vbuf)
    }

    /// Render triangles to a PNG file (offscreen path — unchanged behaviour from S-10).
    pub async fn render_triangles_png(
        &mut self,
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
        // Scene pass -> HDR target, then filmic post pass -> offscreen PNG target.
        let hdr = self.hdr_view.clone();
        self.record_pass(&mut encoder, tris, camera, &hdr, 0.3)?;
        self.post_pass(&mut encoder, &target_view);
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
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
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
        &mut self,
        tris: &[Triangle],
        camera: &GpuCamera,
        surface_view: &wgpu::TextureView,
        time_of_day: f32,
    ) -> anyhow::Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("voxel-window-enc"),
            });
        // Scene pass -> HDR target.
        let hdr = self.hdr_view.clone();
        self.record_pass(&mut encoder, tris, camera, &hdr, time_of_day)?;
        // Post pass HDR -> surface (filmic tonemap + grade).
        self.post_pass(&mut encoder, surface_view);
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Fullscreen filmic post-FX pass: HDR target -> `target_view` (surface or offscreen).
    fn post_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post-fx-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.post_pipeline);
        pass.set_bind_group(0, &self.post_bg, &[]);
        pass.draw(0..3, 0..1);
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Render triangles into an offscreen target and block until the GPU finishes
    /// (`device.poll(wait)`). Unlike `render_triangles_png` this performs **no**
    /// readback and **no** PNG save, so it is a measurable frame unit for
    /// benchmarks: encode + submit + GPU execution + present-sync.
    pub fn render_triangles(
        &mut self,
        tris: &[Triangle],
        camera: &GpuCamera,
    ) -> anyhow::Result<()> {
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench-color-target"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bench-enc"),
            });
        // Scene pass -> HDR, then filmic post pass -> bench target.
        let hdr = self.hdr_view.clone();
        self.record_pass(&mut encoder, tris, camera, &hdr, 0.3)?;
        self.post_pass(&mut encoder, &target_view);
        self.queue.submit(Some(encoder.finish()));
        // Block until the GPU has actually executed the submitted work. This is the
        // frame's real cost proxy (no surface present, no readback).
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        Ok(())
    }
}

/// Frustum culling for chunk streaming (S-12c deel 2).
/// Extracts the 6 clip-space planes from a view-projection matrix (WebGPU
/// clip space, z in [0,1]) and tests an axis-aligned bounding box (chunk AABB).
/// Lets the client skip chunks entirely behind / outside the camera view,
/// instead of meshing + uploading the whole view-radius every frame.
pub struct Frustum {
    /// (a, b, c, d) with plane equation a*x + b*y + c*z + d >= 0 = inside.
    planes: [glam::Vec4; 6],
}

impl Frustum {
    /// Build the 6 frustum planes from a column-major view-projection matrix
    /// (as returned by `GpuCamera::view_proj`).
    pub fn from_view_proj(vp: &[[f32; 4]; 4]) -> Self {
        // vp is to_cols_array_2d(): vp[col][row]. Row-major extraction:
        let m = vp;
        // left, right, bottom, top, near, far (WebGPU z in [0,1])
        let planes = [
            // left  = row3 + row0
            glam::Vec4::new(
                m[0][3] + m[0][0],
                m[1][3] + m[1][0],
                m[2][3] + m[2][0],
                m[3][3] + m[3][0],
            ),
            // right = row3 - row0
            glam::Vec4::new(
                m[0][3] - m[0][0],
                m[1][3] - m[1][0],
                m[2][3] - m[2][0],
                m[3][3] - m[3][0],
            ),
            // bottom= row3 + row1
            glam::Vec4::new(
                m[0][3] + m[0][1],
                m[1][3] + m[1][1],
                m[2][3] + m[2][1],
                m[3][3] + m[3][1],
            ),
            // top   = row3 - row1
            glam::Vec4::new(
                m[0][3] - m[0][1],
                m[1][3] - m[1][1],
                m[2][3] - m[2][1],
                m[3][3] - m[3][1],
            ),
            // near  = row2            (WebGPU z in [0,1])
            glam::Vec4::new(m[0][2], m[1][2], m[2][2], m[3][2]),
            // far   = row3 - row2
            glam::Vec4::new(
                m[0][3] - m[0][2],
                m[1][3] - m[1][2],
                m[2][3] - m[2][2],
                m[3][3] - m[3][2],
            ),
        ];
        Self { planes }
    }

    /// True if the AABB centered at `center` with half-extent `half` intersects
    /// (or is inside) the frustum. Uses the standard "test all 8 corners per plane"
    /// — a chunk is culled only if it is fully outside at least one plane.
    pub fn intersects_aabb(&self, center: [f32; 3], half: f32) -> bool {
        let c = glam::Vec3::new(center[0], center[1], center[2]);
        for p in &self.planes {
            let n = glam::Vec3::new(p.x, p.y, p.z);
            let d = p.w;
            // closest point on the AABB to the plane normal
            let px = c.x + (if n.x >= 0.0 { half } else { -half });
            let py = c.y + (if n.y >= 0.0 { half } else { -half });
            let pz = c.z + (if n.z >= 0.0 { half } else { -half });
            // if the closest corner is outside this plane, the whole box is outside
            if n.x * px + n.y * py + n.z * pz + d < 0.0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P0 spike (2026-07-15): the VBO staging cap must exceed the legacy 256 MB limit so
    /// the vertical-scale terrain (multi-chunk-Y) can actually be drawn without truncation.
    /// RED until `MAX_VBO_BYTES` is raised above 256 MB.
    #[test]
    fn vbo_cap_exceeds_legacy_256mb() {
        assert!(
            MAX_VBO_BYTES > 256 * 1024 * 1024,
            "VBO cap {MAX_VBO_BYTES} must exceed the legacy 256 MB limit"
        );
    }

    #[test]
    fn frustum_culls_behind_camera() {
        // Camera at origin, yaw=-pi/2 -> forward = (0,0,-1) (looks down -Z), fov 60°, aspect 1.
        let cam = GpuCamera::new([0.0, 0.0, 0.0], -std::f32::consts::FRAC_PI_2, 0.0, 1.0);
        let vp = cam.view_proj();
        let f = Frustum::from_view_proj(&vp);
        // Chunk 10 m in front (-Z) -> visible.
        assert!(
            f.intersects_aabb([0.0, 0.0, -10.0], 2.0),
            "chunk in front (-Z) should be visible"
        );
        // Chunk 10 m behind (+Z) -> culled.
        assert!(
            !f.intersects_aabb([0.0, 0.0, 10.0], 2.0),
            "chunk behind (+Z) should be culled"
        );
        // Chunk far to the side (+X 50 m) -> culled.
        assert!(
            !f.intersects_aabb([50.0, 0.0, -10.0], 2.0),
            "chunk far to the side should be culled"
        );
    }

    #[test]
    fn live_spawn_frustum_contains_at_least_one_world_chunk() {
        // Exact current gpu_window spawn. This catches meter/voxel unit drift that can leave
        // the live client with zero selected chunks and therefore a permanently white surface.
        let cam = GpuCamera::new(
            [6.0, crate::spawn_eye_y_m(28, 3), 6.0],
            -std::f32::consts::FRAC_PI_2,
            -0.4,
            1280.0 / 800.0,
        );
        let f = Frustum::from_view_proj(&cam.view_proj());
        let visible = (0..=25i64)
            .flat_map(|cx| (0..=25i64).map(move |cz| (cx, cz)))
            .filter(|&(cx, cz)| {
                f.intersects_aabb([(cx as f32 + 0.5) * 4.0, 6.0, (cz as f32 + 0.5) * 4.0], 6.0)
            })
            .count();
        assert!(visible > 0, "live spawn frustum selected zero world chunks");
    }

    #[test]
    fn resize_recreates_matching_depth_attachment() {
        futures::executor::block_on(async {
            let mut scene = GpuScene::new_offscreen(64, 64).await.expect("gpu scene");
            scene.resize(96, 80);
            assert_eq!(scene.size(), (96, 80));
            let chunk = {
                let cx = 0i64;
                let cz = 0i64;
                let cy = (voxel_worldgen::surface_height_m(cx * 32 + 16, cz * 32 + 16, 7)
                    / voxel_core::coords::VOXEL_SIZE_M) as i64
                    / 32;
                voxel_worldgen::generate_chunk(voxel_core::coords::ChunkCoord::new(cx, cy, cz), 7)
            };
            let tris = crate::mesh_chunk_world_meters(&chunk, crate::chunk_stream::Lod::Full, false, &[], 1024);
            let cam = GpuCamera::new(
                [2.0, 4.0, 6.0],
                -std::f32::consts::FRAC_PI_2,
                -0.4,
                96.0 / 80.0,
            );
            scene
                .render_triangles(&tris, &cam)
                .expect("render after resize");
        });
    }

    /// Mijlpaal 4 P0 (failing → passing): a flat-shaded grass surface shows exactly one
    /// green tint; after the texture-array + triplanar pipeline it must show MULTIPLE distinct
    /// green tints on the same surface (procedural/texture variation). This is the pixel-oracle
    /// that proves the renderer actually samples a material texture instead of a flat tint.
    #[test]
    fn grass_surface_shows_texture_variation_not_flat_tint() {
        futures::executor::block_on(async {
            let mut scene = GpuScene::new_offscreen(128, 128).await.expect("gpu scene");
            // A single flat grass quad at world y=0 (4x4 m), material GRASS (id 2).
            // Winding is CCW as seen from +Y (matches greedy_mesh / S-11), so the
            // top face is front-facing for the back-face cull.
            let tris = vec![
                voxel_mesher::Triangle {
                    a: voxel_mesher::Vec3::new(0.0, 0.0, 0.0),
                    b: voxel_mesher::Vec3::new(0.0, 0.0, 4.0),
                    c: voxel_mesher::Vec3::new(4.0, 0.0, 4.0),
                    normal: voxel_mesher::Vec3::new(0.0, 1.0, 0.0),
                    material: voxel_core::palette::MaterialId::from(2u8),
                    ao: [1.0; 3],
                    sun: [1.0; 3],
                },
                voxel_mesher::Triangle {
                    a: voxel_mesher::Vec3::new(0.0, 0.0, 0.0),
                    b: voxel_mesher::Vec3::new(4.0, 0.0, 4.0),
                    c: voxel_mesher::Vec3::new(4.0, 0.0, 0.0),
                    normal: voxel_mesher::Vec3::new(0.0, 1.0, 0.0),
                    material: voxel_core::palette::MaterialId::from(2u8),
                    ao: [1.0; 3],
                    sun: [1.0; 3],
                },
            ];
            // Camera at the proven live-client angle (yaw=-pi/2 looks down -Z), placed
            // above and in front of the quad so the grass top face is clearly in view.
            let cam = GpuCamera::new(
                [2.0, 6.0, 6.0],
                -std::f32::consts::FRAC_PI_2,
                -0.5,
                1.0,
            );
            let path = std::env::temp_dir().join("m4_grass_p0.png");
            scene
                .render_triangles_png(&tris, &cam, path.to_str().unwrap())
                .await
                .expect("png render");
            // Count distinct green tints on the grass surface (ignore near-black background).
            let img = image::open(&path).expect("open png").to_rgb8();
            let mut greens: std::collections::HashSet<(u8, u8, u8)> = std::collections::HashSet::new();
            for p in img.pixels() {
                let [r, g, b] = [p[0], p[1], p[2]];
                // grass-green mask: green dominant, not background (dark), not white.
                if g > 60 && g > r && g > b && !(r < 20 && g < 20 && b < 20) {
                    // quantize to 8-step bins so lighting noise isn't mistaken for texture
                    greens.insert((r / 16, g / 16, b / 16));
                }
            }
            assert!(
                greens.len() > 1,
                "grass surface showed only {} distinct green tint(s) — flat shading, no texture",
                greens.len()
            );
        });
    }

    /// Taak 5 (2026-07-15): het palette moet hoog-verzadigd zijn (geen grijstinten meer).
    /// Meet de gemiddelde verzadiging van de biome-tints (grass/dirt/stone/sand) in HSV;
    /// die moet boven een drempel liggen.
    #[test]
    fn material_palette_is_saturated() {
        // Hulp: RGB [0..1] -> verzadiging (HSV s, 0..1).
        fn sat(r: f32, g: f32, b: f32) -> f32 {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            if max <= 1e-4 {
                0.0
            } else {
                (max - min) / max
            }
        }
        let ids = [1u8, 2, 3, 5, 6, 7]; // dirt, grass, stone, wood, leaf, sand (geen air/metal/snow)
        let mut avg = 0.0f32;
        for &id in &ids {
            let t = material_tint(voxel_core::palette::MaterialId::from(id));
            avg += sat(t[0], t[1], t[2]);
        }
        avg /= ids.len() as f32;
        assert!(
            avg > 0.25,
            "palette avg saturation {avg:.2} too low (grijstinten) — want > 0.25"
        );
    }

    /// Taak 5 (2026-07-15): albedo-tiles moeten echte 4K-scale resolutie hebben (>= 1024 px)
    /// in plaats van de oude 16 px hash-noise.
    #[test]
    fn texture_tiles_are_4k_scale() {
        assert!(
            TEXTURE_TILE >= 1024,
            "albedo tile {} px is niet 4K-scale (want >= 1024)",
            TEXTURE_TILE
        );
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

// Mijlpaal 4 P0: per-material PBR params, indexed by the flat material id.
struct MaterialPbr {
    albedo_tint: vec4<f32>, // rgb tint, a unused
    params: vec4<f32>,      // x=tiling, y=normal_scale, z=roughness, w=metallic
    emissive: vec4<f32>,    // rgb emissive
};
@group(1) @binding(0) var<storage, read> materials: array<MaterialPbr>;
@group(1) @binding(1) var albedo_array: texture_2d_array<f32>;
@group(1) @binding(2) var mat_sampler: sampler;

struct VtxIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) @interpolate(flat) material: u32,
    @location(3) @interpolate(flat) ao: vec3<f32>,
    @location(4) @interpolate(flat) sun: vec3<f32>,
};
struct VtxOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) @interpolate(flat) material: u32,
    @location(2) world_pos: vec3<f32>,
    @location(3) @interpolate(flat) ao: vec3<f32>,
    @location(4) @interpolate(flat) sun: vec3<f32>,
};

@vertex
fn vs_main(in: VtxIn) -> VtxOut {
    var o: VtxOut;
    o.clip = cam.view_proj * vec4<f32>(in.pos, 1.0);
    o.normal = in.normal;
    o.material = in.material;
    o.world_pos = in.pos;
    o.ao = in.ao;
    o.sun = in.sun;
    return o;
}

// Triplanar weights from the world normal (no UVs needed -> no stretch on greedy quads).
fn triplanar_weights(n: vec3<f32>) -> vec3<f32> {
    let w = pow(abs(n), vec3<f32>(4.0, 4.0, 4.0));
    let s = w.x + w.y + w.z;
    return w / max(s, 1e-4);
}

fn sample_albedo(id: u32, p: vec3<f32>, n: vec3<f32>, tiling: f32) -> vec3<f32> {
    let uvw = p * tiling;
    let w = triplanar_weights(n);
    let cx = textureSample(albedo_array, mat_sampler, uvw.yz, id).rgb * w.x;
    let cy = textureSample(albedo_array, mat_sampler, uvw.xz, id).rgb * w.y;
    let cz = textureSample(albedo_array, mat_sampler, uvw.xy, id).rgb * w.z;
    return cx + cy + cz;
}

@fragment
fn fs_main(in: VtxOut) -> @location(0) vec4<f32> {
    let m = materials[in.material];
    let n = normalize(in.normal);
    var albedo = sample_albedo(in.material, in.world_pos, n, m.params.x) * m.albedo_tint.rgb;

    // --- Filmische kleurvariatie (Lay of the Land-stijl), per-fragment op world_pos. ---
    // Goedkope waarde-ruis voor subtiele per-voxel jitter (breekt het 'plastic' effekt).
    let p = in.world_pos;
    let h = fract(sin(dot(floor(p * 4.0), vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
    let jitter = 0.88 + 0.24 * h;            // 0.88..1.12 licht/helderheid jitter
    albedo *= jitter;

    // Zachte hoogte/helling banding (Lay of the Land-stijl): groen laag, steen op steile
    // hellingen, sneeuw boven de boomgrens. Wereld-pos in meters (mesh is in meters).
    let slope = 1.0 - abs(n.y);                 // 0 vlak, 1 verticaal
    let sky = clamp(n.y * 0.5 + 0.5, 0.0, 1.0);
    // Taak 5: warme rots (beige-bruin, geen koud grijs) + warme sneeuw, zodat hellingen
    // kleur houden in plaats van grijs weg te vallen.
    let rock = vec3<f32>(0.58, 0.46, 0.32);
    let snow = vec3<f32>(0.96, 0.97, 0.99);
    // Taak 5 fix: sneeuw pas BOVEN de boomgrens (~90 m), en alleen op vlakke toppen
    // (steile hellingen blijven rots). Vorige drempel (24-30 m) overspoelde alle heuvels
    // nu de surface tot ~199 m gaat -> witte wereld, geen groene heuvels.
    let rock_mix = smoothstep(0.45, 0.85, slope) * 0.35;
    let snow_mix = smoothstep(90.0, 120.0, p.y) * (1.0 - slope);
    albedo = mix(albedo, rock, rock_mix);
    albedo = mix(albedo, snow, snow_mix);
    // Toon-map naar warme, filmische saturatie.
    albedo = pow(albedo, vec3<f32>(0.85, 0.9, 0.95));

    // --- Dag/nacht-cyclus (F2): time_of_day in cam.params.y (0..1 = 1 dag). ---
    let tod = cam.params.y;
    // Zon-hoogte: -1 (middernacht) .. 1 (noon). Azimut draait rond.
    let sun_phase = tod * 6.2831853;
    let sun_elev = sin(sun_phase - 1.5707963);          // -1..1
    let sun_dir = normalize(vec3<f32>(
        cos(sun_phase) * 0.5,
        max(0.04, sun_elev),
        sin(sun_phase) * 0.5,
    ));
    // Dag/licht-fractie: 0 nacht, 1 middag. Zachte schemering bij opkomst/ondergang.
    let day = smoothstep(-0.15, 0.25, sun_elev);
    // Warmte bij schemering (gouden uur): piek rond sun_elev ~ 0.
    let golden = exp(-pow(sun_elev / 0.35, 2.0));       // 0..1, max bij horizon
    // Lucht-gradient: horizon -> zenith, koeler bij dag, warm bij schemering.
    let horizon = mix(vec3<f32>(0.20, 0.22, 0.30), vec3<f32>(0.62, 0.74, 0.92), day);
    let zenith  = mix(vec3<f32>(0.05, 0.06, 0.12), vec3<f32>(0.28, 0.45, 0.85), day);
    let horizon_warm = mix(horizon, vec3<f32>(0.95, 0.55, 0.30), golden * 0.8);
    // Verticale lucht-tint voor de achtergrond (gebruikt indien fragment = lucht).
    let bg_sky = mix(horizon_warm, zenith, clamp(n.y * 0.5 + 0.5, 0.0, 1.0));

    // --- Zachte hemel-lighting i.p.v. harde directional (filmischer, LoL-achtig). ---
    let sky_tint = mix(bg_sky, vec3<f32>(0.62, 0.74, 0.92), day);   // oude tint bij dag, warm bij schemering
    let ground_tint = vec3<f32>(0.35, 0.28, 0.22); // warme bounce vanonder
    let hemi = sky_tint * sky + ground_tint * (1.0 - sky);
    let ambient = mix(0.10, 0.38, day);            // donkerder 's nachts
    // Zachte key-light voor vorm, geen harde schaduwranden.
    let L = sun_dir;
    let diff = max(dot(n, L), 0.0) * day;
    // Per-vertex AO (F5, baked in the mesher) darkens crevices/contact shadows; the
    // fragment AO is the average of the 3 corner values. Keep the cheap value-noise ONLY
    // as subtle per-voxel brightness jitter (breaks the 'plastic' look), not as AO.
    let ao_corner = (in.ao.x + in.ao.y + in.ao.z) / 3.0;
    let ao = ao_corner * (0.9 + 0.2 * h);   // AO modulated by subtle jitter

    // Stap 3 (BFS zonlicht-lighting): `in.sun` is the baked per-voxel sky-light in [0,1]
    // (1 = open sky, 0 = deep cave / under a roof). It replaces the old uniform "open sky"
    // assumption so caves and overhangs go dark while exposed terrain stays bright. The
    // hemi term below already models sky-vs-ground bounce; `sun` scales that bounce so a
    // shadowed voxel receives only the dim ambient floor (no sky contribution).
    let sun_corner = (in.sun.x + in.sun.y + in.sun.z) / 3.0;
    let sun = clamp(sun_corner, 0.0, 1.0);

    var col = albedo * (hemi * (ambient + 0.55) * sun + vec3<f32>(1.0, 0.96, 0.88) * 0.35 * diff) * ao;
    col += m.emissive.rgb * day;

    let dist = length(in.world_pos - cam.eye_pos.xyz);
    let fog = 1.0 - exp(-cam.params.x * dist);
    // Fog-kleur volgt de lucht (warm bij schemering, koel bij dag, donker bij nacht).
    let fog_col = mix(bg_sky, vec3<f32>(0.10, 0.12, 0.20), (1.0 - day) * 0.6);
    col = mix(col, fog_col, clamp(fog, 0.0, 0.85));
    return vec4<f32>(col, 1.0);
}
"#;

/// F1 post-FX: fullscreen filmic pass. Reads the linear HDR scene target and
/// applies exposure + ACES tonemap + teal-orange grade + saturation, writing
/// display-ready linear colour into the surface (srgb-encoded by the present).
const POST_WGSL: &str = r#"
struct PostParams {
    exposure: f32,
    saturation: f32,
    grade: f32,
    _pad: f32,
};
@group(0) @binding(0) var hdr_tex: texture_2d<f32>;
@group(0) @binding(1) var hdr_sampler: sampler;
@group(0) @binding(2) var<uniform> pp: PostParams;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_post(@builtin(vertex_index) vi: u32) -> VOut {
    // Fullscreen triangle (covers the viewport with one primitive).
    var p = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var o: VOut;
    o.pos = vec4<f32>(p[vi], 0.0, 1.0);
    var uv = p[vi] * 0.5 + vec2<f32>(0.5, 0.5);
    uv.y = 1.0 - uv.y;
    o.uv = uv;
    return o;
}

// ACES filmic tonemap (Narkowicz approximation) — smooth highlight rolloff.
fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_post(in: VOut) -> @location(0) vec4<f32> {
    var hdr = textureSample(hdr_tex, hdr_sampler, in.uv).rgb;
    hdr = hdr * pp.exposure;
    var col = aces(hdr);
    // Teal-orange split-tone: shadows cool, highlights warm (filmic look).
    let l = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    let tint = mix(vec3<f32>(0.85, 1.05, 1.15), vec3<f32>(1.12, 0.95, 0.75), l);
    col = col * mix(vec3<f32>(1.0), tint, pp.grade * 0.5);
    // Saturation around luma.
    let g = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = mix(vec3<f32>(g), col, pp.saturation);
    return vec4<f32>(col, 1.0);
}
"#;
