//! Minimal CPU bitmap-font HUD: renders a short debug string to an RGBA texture and
//! draws it screen-space (top-right), pixel-perfect (1 texel = 1 screen pixel) so text
//! is crisp and never mirrored. No external font/text deps — a 5x7 glyph table lives
//! here, upscaled by `SCALE`. Rebuilt every frame from the client's live stats.
use wgpu::util::DeviceExt;

const GW: u32 = 440; // HUD texture width  (px)
const GH: u32 = 168; // HUD texture height (px)
const SCALE: usize = 2; // integer upscale of the 5x7 font (2 => ~10x14 glyphs)
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;
const CELL_W: usize = 6; // glyph advance before scaling (5px + 1px gap)
const CELL_H: usize = 8; // line advance before scaling (7px + 1px gap)
const PAD_X: usize = 6;
const PAD_Y: usize = 6;
const MARGIN_PX: f32 = 10.0; // inset from the screen corner

/// Foreground / background colors (RGBA, 0..255).
const FG: [u8; 4] = [225, 238, 255, 255];
const BG: [u8; 4] = [8, 12, 20, 190]; // translucent dark panel

/// 5x7 glyphs as 7 rows of 5-bit strings (MSB = left pixel).
fn glyph(c: char) -> Option<[&'static str; GLYPH_H]> {
    Some(match c {
        ' ' => ["00000", "00000", "00000", "00000", "00000", "00000", "00000"],
        ':' => ["00000", "00000", "00100", "00000", "00100", "00000", "00000"],
        '.' => ["00000", "00000", "00000", "00000", "00000", "00100", "00100"],
        '-' => ["00000", "00000", "00000", "11111", "00000", "00000", "00000"],
        '/' => ["00001", "00010", "00010", "00100", "01000", "01000", "10000"],
        '0' => ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
        '1' => ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
        '2' => ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
        '3' => ["11111", "00010", "00100", "00010", "00001", "10001", "01110"],
        '4' => ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
        '5' => ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
        '6' => ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
        '7' => ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
        '8' => ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
        '9' => ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
        'A' => ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
        'C' => ["01110", "10001", "10000", "10000", "10000", "10001", "01110"],
        'D' => ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
        'E' => ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
        'F' => ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
        'H' => ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
        'I' => ["01110", "00100", "00100", "00100", "00100", "00100", "01110"],
        'K' => ["10001", "10010", "10100", "11000", "10100", "10010", "10001"],
        'L' => ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
        'M' => ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
        'N' => ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
        'O' => ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
        'P' => ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
        'R' => ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
        'S' => ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
        'T' => ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
        'U' => ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
        'W' => ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
        'X' => ["10001", "10001", "01010", "00100", "01010", "10001", "10001"],
        'Y' => ["10001", "10001", "01010", "00100", "00100", "00100", "00100"],
        'Z' => ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
        _ => return None,
    })
}

/// Pixel-perfect top-right quad (x,y in clip space, u,v in [0,1], correctly
/// oriented so texel (0,0) lands at the top-left corner — no mirror/flip).
fn quad_verts(surf_w: u32, surf_h: u32) -> [f32; 24] {
    let w = 2.0 * GW as f32 / surf_w.max(1) as f32; // NDC width  of GW screen px
    let h = 2.0 * GH as f32 / surf_h.max(1) as f32; // NDC height of GH screen px
    let mx = 2.0 * MARGIN_PX / surf_w.max(1) as f32;
    let my = 2.0 * MARGIN_PX / surf_h.max(1) as f32;
    let x2 = 1.0 - mx; // right edge
    let x1 = x2 - w; // left edge
    let y2 = 1.0 - my; // top edge
    let y1 = y2 - h; // bottom edge
    #[rustfmt::skip]
    let v = [
        x1, y2, 0.0, 0.0, // TL
        x2, y2, 1.0, 0.0, // TR
        x2, y1, 1.0, 1.0, // BR
        x1, y2, 0.0, 0.0, // TL
        x2, y1, 1.0, 1.0, // BR
        x1, y1, 0.0, 1.0, // BL
    ];
    v
}

pub struct Hud {
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    #[allow(dead_code)]
    sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    vbuf: wgpu::Buffer,
    surf: (u32, u32), // last surface size the vbuf was built for
    buf: Vec<u8>, // RGBA8, GW*GH*4
}

impl Hud {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud-texture"),
            size: wgpu::Extent3d { width: GW, height: GH, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hud-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hud-verts"),
            contents: bytemuck::cast_slice(&quad_verts(1280, 800)),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud-bgl"),
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
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud-shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("hud.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud-pipe-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                        wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 8, shader_location: 1 },
                    ],
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let buf = vec![0u8; (GW * GH * 4) as usize];
        let hud = Hud { texture, view, sampler, bind_group, pipeline, vbuf, surf: (1280, 800), buf };
        hud.clear(queue);
        hud
    }

    fn clear(&self, queue: &wgpu::Queue) {
        let px = vec![0u8; (GW * GH * 4) as usize];
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(GW * 4), rows_per_image: Some(GH) },
            wgpu::Extent3d { width: GW, height: GH, depth_or_array_layers: 1 },
        );
    }

    /// Rasterize `text` (newline-separated lines) into the RGBA buffer, upscaled by SCALE.
    fn rasterize(&mut self, text: &str) {
        let mut px = vec![0u8; (GW * GH * 4) as usize];
        for p in px.chunks_exact_mut(4) { p.copy_from_slice(&BG); }
        let cw = CELL_W * SCALE;
        let ch = CELL_H * SCALE;
        for (li, line) in text.lines().enumerate() {
            let y0 = PAD_Y + li * ch;
            if y0 + GLYPH_H * SCALE > GH as usize { break; }
            let mut cx = PAD_X;
            for c in line.chars() {
                if cx + GLYPH_W * SCALE > GW as usize { break; }
                if let Some(g) = glyph(c) {
                    for (row, bits) in g.iter().enumerate() {
                        for (col, bit) in bits.chars().enumerate() {
                            if bit != '1' { continue; }
                            // paint a SCALE x SCALE block per font pixel
                            for dy in 0..SCALE {
                                for dx in 0..SCALE {
                                    let x = cx + col * SCALE + dx;
                                    let y = y0 + row * SCALE + dy;
                                    let o = (y * GW as usize + x) * 4;
                                    px[o..o + 4].copy_from_slice(&FG);
                                }
                            }
                        }
                    }
                }
                cx += cw;
            }
        }
        self.buf = px;
    }

    /// Rebuild the HUD from live stats and upload to the GPU. `surf_w/surf_h` keep the
    /// panel pixel-perfect (and correctly oriented) across window resizes.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        fps: f32,
        p: [f32; 3],
        yaw_deg: f32,
        chunks: usize,
        tris: usize,
        seed: u32,
        mode: &str,
        time: f32,
        surf_w: u32,
        surf_h: u32,
    ) {
        let yaw = ((yaw_deg % 360.0) + 360.0) % 360.0;
        let text = format!(
            "FPS: {:.0}\nX: {:.1}  Y: {:.1}  Z: {:.1}\nYAW: {:.0}\nCHUNKS: {}  TRIS: {}\nSEED: {}  MODE: {}\nTIME: {:.2}",
            fps, p[0], p[1], p[2], yaw, chunks, tris, seed, mode, time
        );
        self.rasterize(&text);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &self.buf,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(GW * 4), rows_per_image: Some(GH) },
            wgpu::Extent3d { width: GW, height: GH, depth_or_array_layers: 1 },
        );
        // Rebuild the quad only when the surface size changed (cheap; keeps it pixel-perfect).
        if self.surf != (surf_w, surf_h) {
            let _ = device; // device kept in the signature for symmetry / future needs
            queue.write_buffer(&self.vbuf, 0, bytemuck::cast_slice(&quad_verts(surf_w, surf_h)));
            self.surf = (surf_w, surf_h);
        }
    }

    /// Draw the HUD into `target_view` (must be the same format as the pipeline).
    pub fn draw<'a>(&'a self, encoder: &'a mut wgpu::CommandEncoder, target_view: &'a wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hud-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.vbuf.slice(..));
        pass.draw(0..6, 0..1);
    }
}
