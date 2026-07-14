# Spike S-12 deel 2 — wgpu 0.17 → 30 + winit 0.30 (Fase 2a interactieve client)

## Aanleiding
Fase-2 volgorde (ROADMAP "Directe volgende stap" a): de interactieve GPU-client vereist een
moderne wgpu + een winit-event-loop. Huidige `voxel-gpu` zit op `wgpu = "0.17"`, géén winit,
offscreen PNG. S-12 deel 1 (clone-fix) is al gecommit.

## Onderzoek (gedelegeerd, gratis model, door implementer live geverifieerd)
- Versie-pin geverifieerd via crates.io live API: wgpu laatst stabiel = **30.0.0**, winit
  laatst stabiel = **0.30.13** (0.31 is beta). Plan: `wgpu = "30"`, `winit = "0.30"`.
- Volledig migratieplan + API-breuken: `C:\Users\keere\wgpu-winit-migratieplan.md`.

## API-breuken die we aanpakken (0.17 → 30)
1. `device.poll(Maintain::Wait)` → `device.poll(wgpu::PollType::Wait)`.
2. `StoreOp`: `store: true` → `store: StoreOp::Store` in RenderPass Operations.
3. `VertexState`: `entry_point: Option<&str>` (was `&str`), nieuw `compilation_options`,
   `buffers: &[Option<VertexBufferLayout>]` (wrap in `Some`).
4. WGSL: integer vertex-in (`Uint32` block-id) krijgt `@interpolate(flat)`.
5. `BindingResource::from`/`BufferBinding::from` → `TryFrom` (gebruik `BufferBinding::try_from`).
6. Surface-config (nieuw): `color_space: SurfaceColorSpace::Auto` bij handmatig bouwen.
7. Offscreen-PNG-pad behouden; map_async + PollType::Wait expliciet (geen fire-and-forget).

## Doel
- Offscreen render-to-PNG blijft werken (bestaande `examples/gpu_world.rs`).
- NIEUW: `examples/gpu_window.rs` — winit-venster met `ApplicationHandler`, surface-render
  naar een live venster, WASD/muis-camera besturing, continue render-loop (RedrawRequested).
- Camera-input koppelt aan de bestaande `GpuCamera` (yaw/pitch/eye), geen eigen matrix-fouten.

## Aanpak (minimal change, één pipeline)
- `GpuScene` refactoren: pipeline + depth in `Option`, geïnitialiseerd zodra het target-format
  bekend is. Render-functie neemt een `&TextureView` (doel) + depth-view + camera.
- Offscreen: format `Rgba8Unorm` (blijft bit-exact PNG). Window: surface-preferred format.
  Voor minimal change bouwen we de pipeline lazy per format (twee kleine init-paden).
- winit `App` struct: `Arc<Window>` + `Surface<'static>` (zie valkuil §7: Arc<Window>:
  SurfaceTarget<'static>).

## Acceptance criteria
- `cargo build --workspace --examples` slaagt.
- `cargo run --example gpu_world -p voxel-gpu` produceert nog steeds gpu_world.png (offscreen
  ongewijzigd qua uitkomst).
- `cargo run --example gpu_window -p voxel-gpu` opent een venster, rendert de voxel-wereld,
  camera reageert op WASD + muis, sluiten via het venster werkt. (Visueel geverifieerd.)
- `cargo test --workspace` blijft groen (GPU-tests gemarkeerd met cfg/ignore waar nodig).
- Geen nieuwe dependencies buiten winit.

## Buiten scope (eigen spikes later)
Chunk-streaming (S-12 deel 3), binary greedy meshing, AO/CSM, LOD. Die volgen na de
benchmark-gate.
