# Senior Engineering Review — F2 day/night, step-up bugfix, Walk/Fly
**Reviewer:** GPT-sol-level Rust/wgpu review · **Date:** 2026-07-15
**Scope read:** `voxel-gpu/src/renderer.rs` (`fs_main`, `record_pass`, `render_to_view`), `voxel-player/src/lib.rs` (`try_step`, `collides`, `resolve_floor_y`), `voxel-gpu/examples/gpu_window.rs` (streaming + frame-1 fallback + Walk/Fly), `voxel-world`/`voxel-worldgen`, `.hermes/PROJECT_STATE.md`.

## (a) Verdicts on the 4 open questions

**Q1 — `try_step` soundness.** Largely sound. The floor-exclusion (`target.y+1` check) correctly stops the per-frame +1 step that caused the NaN-eye → 0-chunk → white-screen regression, and the foot-ground check (`foot_y = y-HALF[1]-1`, solid-required) prevents floating. STEP_HEIGHT=1 blocks >1-voxel rises consistently. Caveats: (1) the foot-ground probe samples only the **center column** (`foot.floor()` single voxel) not the full ~5-voxel footprint, so you can step onto a 1-voxel-wide pillar and stand unsupported for a frame until gravity catches it. (2) The Z-axis `try_step` re-bases on the *already-raised* Y from the X-step, so on a diagonal slope you can gain ~2 vox/substep (double-step). Mild, but unbounded per substep. (3) Off-ledge is fine (flat move accepted, gravity pulls down via `resolve_floor_y`). (4) 1-voxel thin ceilings are handled (raised slot must be clear). Acceptable for a vertical slice; tighten the footprint check before physics.

**Q2 — frame-1 fallback seeds empty chunk.** **Real bug.** `cy = (col_top_vox + CHUNK_SIZE)/CHUNK_SIZE` (line 637) picks the chunk *above* the surface. `euclidean_div(210,32)=6` is the surface chunk; the formula gives `242/32=7` (voxels 224–255 = air). So when streaming hasn't populated, the fallback seeds an EMPTY chunk → still a clear-flash, defeating its own purpose. Correct: `cy = col_top_vox.div_euclid(CHUNK_SIZE)` (and ideally also seed `cy-1` so ground is guaranteed visible). Note the *streaming* `max_cy` uses the same `+CHUNK_SIZE` but iterates `0..=max_cy`, so it still requests chunk 6 — hence the bug is masked in steady state and only bites the single-chunk fallback. Low severity today, degenerate safety net.

**Q3 — streaming perf (~16 FPS @ r48).** Root cause is **not** chunk-gen (off-thread, ~0.26 ms/chunk, cached after first gen). It is, per frame: (1) O(R²) scan of ~7.2k columns, each calling `surface_height_m` (3 fBm calls) → ~21k fBm/frame just to compute `max_cy`; (2) **full re-upload** of all visible triangles every frame (`tris` rebuilt, `write_buffer` for the whole visible set — the 1.25M-tri re-upload is the 16 FPS killer); (3) scan re-runs every frame even when the camera hasn't moved a chunk. Fixes: memoize per-column `surface_height_m` in a `HashMap<(cx,cz),u8>`; only re-run the scan when the camera crosses a chunk boundary; switch to **persistent per-chunk GPU buffers** (upload-once on cache insert, draw a compact visible-list via a buffer of chunk-ids/offsets or indirect draw). This alone should recover most of the headroom.

**Q4 — structural smells.** Ranked in (b).

## (b) Top 5 best-practices / refactors (by impact)

1. **Unify terrain source for render + collision (correctness blocker for destruction).** Render worker calls raw `generate_chunk` (line 619) and ignores edits; collision uses `world.material_at` (edits honored); frame-1 uses `world.get_or_generate`. Three paths, two of which desync once edits exist. Make the mesh worker pull from the edit-aware `World` (or replay `EditLog` onto every generated chunk) so meshes and collision always agree.
2. **Persistent GPU buffers + visible-list draw** instead of per-frame full re-upload (see Q3). Highest FPS payoff.
3. **Memoize `surface_height_m` per column + scan-on-chunk-boundary** (see Q3). Removes the per-frame fBm storm.
4. **Promote client logic out of `examples/gpu_window.rs` into a `voxel-client` crate** (streaming, mesh-cache, player↔camera coupling). Examples should be thin; this is untestable where it lives and blocks reuse/raytracing integration.
5. **Reconcile vertical cap with terrain height.** `surface_height_m` can reach ~103 m (base 60 + mid 40 + micro 3) but `MAX_Y=12` caps streaming at 48 m → tall peaks are never rendered while collision still has them → invisible terrain + collision/render desync. Either clamp generation to the streamed band or raise `MAX_Y` and stream the true top chunk.

## (c) Latent bugs

- **Render/collision divergence** (item 1 above) — will break the destruction/edit phase.
- **`MAX_Y=12` clips peaks >48 m** while collision keeps full terrain — players can stand on invisible ground / walk off rendered edges.
- **Frame-1 fallback seeds wrong chunk** (Q2) — degenerate safety net.
- **`try_step` foot check samples only the center column** — can accept unsupported 1-wide pillars.
- **Double step-up per substep** on diagonal slopes (Z re-bases on X-raised Y).
- **`solid_at`/`material_at` take `&mut World` but don't mutate** — borrow friction; should be `&self` (or split a read API). `get_or_generate` still clones a 32 KB `Chunk` in the frame-1 path.
- **`time_of_day` via magic `cam.params.y`** index — fragile; prefer a named field/struct.
- Day/night has no moonlight term; at `tod≈0` scene goes near-black (ambient 0.10). Aesthetic, not a bug.

**White-screen note:** confirmed the reported white screen was a capture artifact (leftover killed-window); live captures show UNIQUE_COLORS 7.5k, NEAR_WHITE 0.002%. No real white-screen in the running client. The fallback bug (Q2) is the only white-flash risk and only if streaming fully stalls.
