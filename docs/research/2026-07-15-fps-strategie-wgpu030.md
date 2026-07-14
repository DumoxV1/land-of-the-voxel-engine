# wgpu 0.30 Voxel Engine FPS Strategy (RTX 4080 Super, 1km² @ 12.5cm micro-voxels)

## P0 — Stop reallocating + fix the per-frame mega-upload (biggest win)
Root cause: one 8M-tri `Vec<Triangle>` rebuilt and reuploaded every frame.

1. **Persistent resident buffers.** Create once at startup:
   ```rust
   let vbuf = device.create_buffer(&BufferDescriptor {
       size, usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
       mapped_at_creation: false, ..Default::default() });
   ```
   Suballocate per-chunk slots from a `BufferArena`; only `create_buffer` (grow) when the arena is full. **Never `create_buffer` per frame.**

2. **Avoid the `queue.write_buffer` hitch** (wgpu #1242: ~25ms spike every 676 frames due to internal reallocation). Upload through a small ring of `MAP_WRITE | COPY_SRC` staging buffers; then `encoder.copy_buffer_to_buffer(staging, 0, vbuf, slot_offset, bytes)` and `queue.submit`. Reuse ring buffers — never reallocate them.

   *Expected:* eliminates the 25ms spikes plus the per-frame alloc/serialize of 8M tris. **~5–20x** on the render path alone.

## P1 — Per-chunk frustum culling (rayon on CPU)
Extract 6 planes from `camera.view_proj()` (transpose + normalize). Test each chunk AABB against the planes; build `Vec<VisibleChunk{slot, offset, len}>` with `rayon::par_iter`.

*Cheaper GPU variant:* compute pass culls AABBs and writes `wgpu::util::DrawIndexedIndirectArgs` into an `INDIRECT` buffer; render via `render_pass.draw_indexed_indirect(&indirect, i*20)` with `Features::MULTI_DRAW_INDIRECT`.

*Expected:* 70–90% of the 2401 chunks are behind you / outside FOV → **3–10x** on draw + vertex fetch.

## P2 — Per-frame triangle / distance budget + LOD
After culling, sort visible by `distance²`, accumulate `tris += chunk.tris`, and stop at a budget (e.g. 2M on 4080S). Beyond the radius, drop chunks or swap to coarse LOD meshes (half/quarter-res greedy). Dynamically shrink view-radius under load.

*Expected:* caps worst-case GPU/vertex-fetch load → stable frame budget; **1.5–3x** where vertex-bound.

## P3 — Non-blocking streaming (rayon + async channel)
Mesh/chunk-gen on a `rayon::ThreadPool` (or async task). Hand results to the render thread via `crossbeam::channel` (or tokio mpsc). On receipt, `copy_buffer_to_buffer` into that chunk's arena slot (resident). Mark dirty; **only reupload changed chunks.** Render thread never blocks on meshing.

*Expected:* removes meshing stalls across the 49×49 set → smooth 60+ FPS vs stutter.

## Priority & combined expectation
P0 (pooling/upload) → P1 (culling) → P2 (budget/LOD) → P3 (streaming).
Catastrophic <1 FPS → target **60–144 FPS**. P0+P1 alone: **~10–30x**; full stack removes the structural stalls and OOM/panic risk.
