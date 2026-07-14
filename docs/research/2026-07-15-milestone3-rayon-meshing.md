# Milestone 3 (P3): Non-blocking chunk meshing via a rayon thread-pool

## Why this works here
`World::get_or_generate` (lib.rs:42) clones a chunk, and `World` is not `Sync`. But worldgen is a
**pure function** of `(ChunkCoord, seed)`: `voxel_worldgen::generate_chunk(coord, seed) -> Chunk`,
then `greedy_mesh(&chunk) -> Vec<Triangle>` (mesher.rs:156). Workers therefore need only `seed`
(Copy), never `World` or the GPU. This sidesteps every borrow/`Sync` problem.

## 1. Dedicated rayon pool
```rust
let pool = rayon::ThreadPoolBuilder::new()
    .num_threads(num_cpus::get().saturating_sub(1).max(1)) // keep 1 core for render
    .build().unwrap();
```
Not the global pool — `spawn` is scoped to this pool so CPU cost never collides with other rayon users.

## 2. Channel worker → render thread
`crossbeam_channel::unbounded()` (or `flume`). Sender is `Clone`.
```rust
struct MeshResult { coord: ChunkCoord, gen: u64, tris: Vec<Triangle> }
let (tx, rx) = crossbeam_channel::unbounded::<MeshResult>();
```

## 3. Per-frame upload budget
Render thread drains at most `UPLOAD_BUDGET` (e.g. 4) results per frame from the channel, validates,
and stores them. Remaining results stay buffered → uploads spread across frames, no spike.

## 4. Stale discard (generation counter)
`requested_gen: HashMap<ChunkCoord, u64>` stores the latest generation requested per coord. On
receive, `if requested_gen.get(&coord) != Some(&r.gen) { continue; }` — a newer request superseded
it. When the camera moves, we re-request the coord with `gen+1`; the old in-flight result no longer
matches and is dropped. `pending: HashSet<ChunkCoord>` is the backpressure gate: a coord is only
spawned when not already `pending`.

## 5. Where state lives
All in `App` (gpu_window.rs:26):
```rust
seed: u32,                        // copied from world.seed() at init
mesh_cache: HashMap<ChunkCoord, Vec<Triangle>>,   // unchanged
mesh_pool: rayon::ThreadPool,
mesh_tx: Sender<MeshResult>,
mesh_rx: Receiver<MeshResult>,
requested_gen: HashMap<ChunkCoord, u64>,
pending: HashSet<ChunkCoord>,
```
`World` stays on the render thread (used only for the spawn-height probe in `resumed()`).

## 6. Code skeleton (replaces the loop body in `render_frame`)
```rust
const UPLOAD_BUDGET: usize = 4;

fn render_frame(&mut self) {
    let (Some(scene), Some(surface)) = (&mut self.scene, &self.surface) else { return; };
    let frustum = Frustum::from_view_proj(&self.camera.view_proj());
    let half = CHUNK_M * 0.5; let half_y = CHUNK_M * 1.5;
    let [ex, _, ez] = self.camera.eye;
    let ccx = (ex / CHUNK_M).floor() as i64;
    let ccz = (ez / CHUNK_M).floor() as i64;

    // (1) Drain channel within budget; discard stale by gen.
    let mut budget = UPLOAD_BUDGET;
    while budget > 0 {
        let r = match self.mesh_rx.try_recv() { Ok(r) => r, Err(_) => break };
        budget -= 1;
        if self.requested_gen.get(&r.coord).copied() != Some(r.gen) { continue; }
        self.mesh_cache.insert(r.coord, r.tris);
        self.pending.remove(&r.coord);
    }

    // (2) Stream visible chunks; request missing ones off-thread.
    let mut tris: Vec<Triangle> = Vec::new();
    for dx in -VIEW_RADIUS..=VIEW_RADIUS {
        for dz in -VIEW_RADIUS..=VIEW_RADIUS {
            let (cx, cz) = (ccx + dx, ccz + dz);
            if cx < 0 || cz < 0 { continue; }
            let center = [(cx as f32+0.5)*CHUNK_M, half_y, (cz as f32+0.5)*CHUNK_M];
            if !frustum.intersects_aabb(center, half.max(half_y)) { continue; }
            let coord = ChunkCoord::new(cx, 0, cz);
            if let Some(m) = self.mesh_cache.get(&coord) {
                tris.extend_from_slice(m);                 // ready: draw (frustum-cull intact)
            } else if !self.pending.contains(&coord) {
                let g = self.requested_gen.entry(coord).or_insert(0); *g += 1; let gen = *g;
                self.pending.insert(coord);
                let tx = self.mesh_tx.clone(); let seed = self.seed;
                self.mesh_pool.spawn(move || {             // CPU ONLY — no GPU refs
                    let chunk = voxel_worldgen::generate_chunk(coord, seed);
                    let tris = greedy_mesh(&chunk);
                    let _ = tx.send(MeshResult { coord, gen, tris });
                });
            } // else: pending, not ready -> skipped this frame, pops in later
        }
    }
    if tris.is_empty() { return; }
    // ... unchanged surface/frame + scene.render_to_view(&tris, ...) below ...
}
```

## 7. Risks
- **Backpressure**: `pending` caps in-flight to the (frustum-limited) visible count, so unbounded
  channel can't flood. For a hard cap use `bounded(N)`.
- **HashMap borrows**: the worker closure captures only owned `tx/seed/coord/gen` (never `&self`),
  so no borrow conflict with `&mut self` mutation. `mesh_cache.get` (shared borrow) and
  `pending`/`requested_gen` mutation are sequential, not simultaneous.
- **rayon + wgpu**: workers do ONLY `generate_chunk` + `greedy_mesh`. The `Device`/`Queue` live in
  `scene` on the render thread and are never moved into the closure.
- **Startup flood**: frame 1 requests ~2401 coords; `UPLOAD_BUDGET` spreads uploads → meshes pop in
  progressively instead of one huge stall.
- **Gen ordering**: bump `requested_gen` *before* `spawn`; compare with `== Some(gen)` (not `>=`)
  so a re-request cleanly invalidates the older in-flight result.
- **Cache growth**: out-of-range coords stay in `mesh_cache`. Add a prune pass (drop entries whose
  coord is outside `VIEW_RADIUS` of the camera and not in `pending`) to bound memory.
- **Edits**: `World::set_voxel`/dirty chunks are not re-meshed by this change — wire `take_dirty()`
  into the same request path in a later milestone (DRY: reuse the spawn closure).
