//! Feasibility probe: initialize wgpu on the host GPU, render a simple colored triangle
//! offscreen, read the texture back, and save it as a PNG. If this runs, wgpu works on the
//! target hardware and offscreen readback is viable for the voxel renderer.

use std::sync::Arc;

use wgpu::util::DeviceExt;

/// Vertex for the probe (position + color).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vtx {
    pos: [f32; 3],
    color: [f32; 3],
}

/// Render a colored triangle offscreen and return the PNG bytes.
pub async fn render_probe_png(path: &str) -> anyhow::Result<()> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    // Request an adapter (the GPU). Force Vulkan so we exercise the RTX 4080 path.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| anyhow::anyhow!("no adapter found (GPU/Vulkan unavailable?)"))?;

    let info = adapter.get_info();
    log::info!(
        "adapter: {} | backend={:?} | vendor={} device={}",
        info.name,
        info.backend,
        info.vendor,
        info.device
    );

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

    // Offscreen render target.
    let size = wgpu::Extent3d {
        width: 512,
        height: 512,
        depth_or_array_layers: 1,
    };
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe-target"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());

    // Shader: pass-through position, output color.
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("probe-shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(PROBE_WGSL)),
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("probe-pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vtx>() as u64,
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
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let verts = [
        Vtx { pos: [0.0, 0.6, 0.0], color: [0.95, 0.75, 0.35] },
        Vtx { pos: [-0.6, -0.4, 0.0], color: [0.35, 0.75, 0.95] },
        Vtx { pos: [0.6, -0.4, 0.0], color: [0.55, 0.85, 0.45] },
    ];
    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe-vbo"),
        contents: bytemuck::cast_slice(&verts),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("probe-encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("probe-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05,
                        g: 0.07,
                        b: 0.12,
                        a: 1.0,
                    }),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..3, 0..1);
    }
    queue.submit(Some(encoder.finish()));

    // Read back the texture and save to PNG.
    let bytes_per_row = 512 * 4;
    let buf_size = (bytes_per_row * 512) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe-readback"),
        size: buf_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc2 = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("probe-readback-enc"),
    });
    enc2.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &staging,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: Some(512),
            },
        },
        size,
    );
    queue.submit(Some(enc2.finish()));

    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();
    let mut img = image::RgbaImage::new(512, 512);
    for y in 0..512 {
        for x in 0..512 {
            let i = (y * bytes_per_row + x * 4) as usize;
            img.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([data[i], data[i + 1], data[i + 2], data[i + 3]]),
            );
        }
    }
    drop(data);
    staging.unmap();
    img.save(path)?;
    log::info!("probe PNG written to {path}");
    Ok(())
}

const PROBE_WGSL: &str = r#"
struct VtxIn {
    @location(0) pos: vec3<f32>,
    @location(1) color: vec3<f32>,
};
struct VtxOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec3<f32>,
};
@vertex
fn vs_main(in: VtxIn) -> VtxOut {
    var o: VtxOut;
    o.clip = vec4<f32>(in.pos, 1.0);
    o.color = in.color;
    return o;
}
@fragment
fn fs_main(in: VtxOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

// `Arc` import kept for parity with later real renderer; silence unused warning in probe.
#[allow(dead_code)]
fn _use_arc(_: Arc<()>) {}
