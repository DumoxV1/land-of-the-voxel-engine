# AUDIT F6 — Procedural Clouds Sky-Pass

Scope: `crates/voxel-gpu/src/renderer.rs`. Read self from repo (git diff: +392/-31). No source edits.

## 1. Hook / ordering — OK
`render_frame_passes` (L1631) calls `self.sky_pass(...)` as the FIRST pass, replacing the old flat clear; scene/water passes follow with `LoadOp::Load` (L1351) so geometry composites over the sky. Camera uniform (incl `inv_view_proj`) is written inside `sky_pass` (L757-760) before the draw (L777-779). Correct.

## 2. inv_view_proj unprojection — OK (fix correct)
`view_dir` (L2439-2445) builds NDC from UV, multiplies `cam.inv_view_proj * vec4(ndc,1,1)`, divides by `.w`, subtracts `eye_pos`. This is the correct inverse-projection ray; the old `view_proj*` bug is gone. `GpuCamera::inv_view_proj` (L90-100) computes `vp.inverse()` of the same `proj*view` used in `view_proj` — consistent. Note: `far_pt` uses z=1 (far plane) with w=1; correct for direction reconstruction.

## 3. Build / compat / usage — OK
wgpu 0.30 API used correctly: `bind_group_layouts: &[Some(camera_bgl)]` (L705), `immediate_size: 0` (L706), no `push_constant_ranges`. Sky pipeline target `Rgba16Float` matches HDR (L722/579). HDR has `RENDER_ATTACHMENT | TEXTURE_BINDING` (L580) — sky WRITES it, post READS it as a binding across DIFFERENT render passes in the same encoder; no simultaneous read/write in one pass → no usage conflict. `cargo check` running.

## 4. sky_has_clouds oracle — OK (minor note)
Test (L2043-2070) renders sky-only, samples upper half luminance, asserts `std > 4.0`. Robust: std-dev detects cloud variation, not a flat field. Suggestion (non-blocking): also assert `mean` is in a sane sky range to avoid passing on noise; threshold 4.0 is reasonable for 8-bit luma.

## 5. Regression — OK
F1 post-FX reads HDR via `post_bg` unchanged. F3 shadows untouched (separate depth maps). F4 water is a filtered subset still drawn via `scene_pass` with `LoadOp::Load` over sky. Scene still drawn on top of sky background. No breakage observed in code paths.

## 6. KISS / DRY — OK
Sky shader = fullscreen tri + value-noise FBM (5 octaves) + gradient + soft cloud coverage + golden-hour tint. Moderate, not over-engineered. Reasonable for a procedural sky.

## VERDICT: SAFE TO SHIP
All six aspects pass. `cargo check -p voxel-gpu --tests` CONFIRMED GREEN (Finished dev profile; only pre-existing `sunlight.rs` visibility warnings, none from F6). No code changes made by this auditor.
