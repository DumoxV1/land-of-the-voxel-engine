# Land of the Voxel Engine

Een technisch ambitieuze, filmische 3D **micro-voxel** openwereld-RPG-engine in pure Rust.
Server-authoritative, deterministisch, persistent — en een **interactieve GPU-client**
(wgpu/Vulkan op je RTX 4080) die een 12,5 cm-voxel-wereld streamt en rendert.

## Status (2026-07-15)

Vertical slice bereikt (spikes S-01..S-13, allen onder strict TDD). Micro-voxel-resolutie
**12,5 cm/voxel** (ADR-0005); interactieve wgpu-client met **state-of-the-art chunk-streaming**
(scheduler, bounded worker-pool, frustum-first, 3-tier LOD: Full/Half/Imposter), vertex-AO,
en dag/nacht. Recent onderzoek (18 YouTube-bronnen + papers/repos) leverde een
`retain / update / replace / reject`-matrix op — zie hieronder en
[`docs/research/voxel-engine-survey-2026/`](docs/research/voxel-engine-survey-2026/).

| Crate | Wat het doet |
|-------|--------------|
| `voxel-core` | Chunk (3 states + 4-bit bitpacking), byte-stabiele serialisatie, coördinaten (12,5 cm-schaal) |
| `voxel-mesher` | Greedy meshing (chunk → driehoeken) + vertex-AO (0fps-methode) |
| `voxel-render` | Pure-Rust software-rasterizer → PNG (geen GPU) |
| `voxel-worldgen` | Deterministische, seeded terrain-generatie (fBm + 3-tier biomes) |
| `voxel-world` | Multi-chunk wereldstore (cache + edits) |
| `voxel-edit` | Edit-events + append-only log (replay/persistentie) |
| `voxel-persist` | Save/load wereld als (seed + edit-log) |
| `voxel-player` | First-person spelercontroller + voxel-collision (step-up, sub-stepping) |
| `voxel-server` | **Headless authoritative server (geen GPU)** |
| `voxel-gpu` | **wgpu GPU-renderer** (mesh/LOD/worker-pool/scheduler) |
| `voxel-client` | **Interactieve client-crate** (App + event-loop + streaming-glue). Geëxtraheerd uit de oude `gpu_window.rs`-example. |

## Wat je nu kunt runnen

### 1. De interactieve GPU-client (12,5 cm micro-voxels, rondlopen!)
```bash
cargo run --release --example gpu_window_main -p voxel-client
```
Opent een venster en streamt een 12,5 cm-voxel-wereld rond een first-person camera.
**WASD** = lopen, **muis** = rondkijken, **Space** = springen (alleen op grond),
**F** = Walk/Fly, **F2** = dag/nacht. Sluit het venster om te stoppen.
Spawn = op de terrain-hoogte; view-distance ~96 m (24 chunks). Vereist een GPU (RTX 4080 / Vulkan).

### 2. De headless server (GPU-vrij — het runbare artifact)
```bash
cargo run --example headless_server -p voxel-server
```
Spawn 3 spelers, simuleert 600 ticks, plaatst een "beacon"-edit, en print een
state-samenvatting. Bewijst dat de engine een server draait zónder renderer/GPU.

### 3. Tests (hele workspace)
```bash
cargo test --workspace
```

### 4. Demo-PNG's genereren (software-rasterizer / GPU)
```bash
cargo run --example demo        -p voxel-render   # enkele chunk
cargo run --release --example gpu_world -p voxel-gpu   # GPU-render naar gpu_world.png
cargo run --example demo_world   -p voxel-render   # 2x2 chunks + toren-edit
cargo run --example demo_persist -p voxel-render   # save -> load -> render
cargo run --example demo_player  -p voxel-render   # first-person grond-view
```
Output: `crates/voxel-render/demo*.png`.

## Architectuur-beslissingen (ADR's)
- **ADR-0002** — renderer-agnostische core (geen Godot/Bevy/wgpu in de core-crates).
- **ADR-0003** — GPU-vrije authoritative server.
- **ADR-0004** — client-shell = **Rust + Bevy/wgpu** (eerste slice). Godot GDExtension
  verworpen voor de eerste slice. Fase-2 benchmark-gate (B-06/B-07 + FPS) nog te lopen
  vóór definitieve lock-in. Zie `docs/architecture/adr/`.
- **ADR-0005** — micro-voxel-resolutie **12,5 cm/voxel** (verticale wereldschaal ~40 m).
- **ADR-0006** — **Voxel ray marching als additieve filmische laag** (geen HW-DXR-primair,
  géén octree-rewrite). RTAO → voxel GI → volumetric clouds → filmische post-stack.
  Zie `docs/architecture/adr/0006-voxel-ray-marching-layer.md`.

## Onderzoek: retain / update / replace / reject
18 YouTube-bronnen (video's + kanalen) + papers/repos onderzocht.
Kernconclusie: **onze architectuur is niet inferieur**; winst zit in specifieke technieken,
niet in taal-/opslag-rewrites.
- **Behouden:** greedy meshing, 3-tier LOD, bounded worker-pool, vertex-AO, AABB-collision.
- **Aanpassen (prioriteit):** crack-free skirts (LOD-kieren-bug), inter-chunk occlusie,
  BFS zonglift-lighting, god-rays, raycast block-pick, memory-locality (Z-order/3D-texture).
- **Verwerpen:** SVO/DAG/AMR/OpenVDB, cubic chunks, full voxel-rigid-body (O(N³)),
  Veloren (GPL-3.0), Transvoxel (proprietary — eigen stitching schrijven).
- **Late fase:** GPU-driven/indirect (Fase 5, pas bij >100k chunks), octree (schaal-fase).
Volledige matrix: [`docs/research/voxel-engine-survey-2026/RETAIN_UPDATE_REPLACE_MATRIX.md`](docs/research/voxel-engine-survey-2026/RETAIN_UPDATE_REPLACE_MATRIX.md).

## Roadmap (wat gedaan is)
- **S-01..S-13** — vertical slice: core, mesher, worldgen, world, edit, persist, player,
  server, GPU-renderer, chunk-streaming, client (strict TDD, 83/83+ tests groen).
- **Perf Fase A** — chunk-loader speedups (empty/air-skip, 4.1×), per-column height-cache (6.1×),
  chunk-gen hot-path (14×).
- **F2** — dag/nacht + Walk/Fly + step-up collision (TDD mid-air-jump fix).
- **F5** — vertex-AO gebakken in mesher, naar shader (geen runtime-kost).
- **Chunk-streaming Fase 1+2** — ChunkScheduler (close→far prioriteit, LOD-rings, air-skip),
  bounded worker-pool, frustum-cull vóór aanvraag (`4c528ad`).
- **A3** — collision-first streaming: 2-phase worker (Gen voor collision → Mesh) (`1c4f53f`).
- **B2** — distant-chunk imposters: 3-tier LOD Full/Half/Imposter (`4b334d8`).
- **Client-extractie** — `gpu_window.rs` → `voxel-client` crate (`02167ff`).
- **Onderzoek** — 18-bron survey + ray-marching ADR (`docs/research/...`, `adr/0006`).

## Volgende stap (verificatie vóór uitvoer)
1. Crack-free skirts (LOD-bug fix) — klein, veilig, hoogste ROI.
2. Inter-chunk occlusie (grootste chunk-reductie bij 12,5cm/r48).
3. BFS zonglift-lighting (cave-schaduw).
4. Voxel RTAO/RTGI compute-spike (ADR-0006 — filmische laag).
5. Volumetric clouds/weather (latere filmische fase).
Daarna: Fase 4 netwerk/protocol-laag voor 2–8 spelers multiplayer.

## Budget
Geen betaalde API-calls voor de engine-ontwikkeling; alles lokaal + gratis modellen.
