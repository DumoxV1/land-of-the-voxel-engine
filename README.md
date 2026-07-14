# Land of the Voxel Engine

Een technisch ambitieuze, filmische 3D **micro-voxel** openwereld-RPG-engine in pure Rust.
Server-authoritative, deterministisch, persistent — en (voor de eerste slice) volledig
headless runbaar **zonder GPU**.

## Status (2026-07-15)
Vertical slice bereikt (spikes S-01..S-09, allen onder strict TDD). Zie
[`.hermes/PROJECT_STATE.md`](.hermes/PROJECT_STATE.md) en [`docs/governance/alignment-log.md`](docs/governance/alignment-log.md).

| Crate | Wat het doet |
|-------|--------------|
| `voxel-core` | Chunk (3 states + 4-bit bitpacking), byte-stabiele serialisatie |
| `voxel-mesher` | Greedy meshing (chunk → driehoeken) |
| `voxel-render` | Pure-Rust software-rasterizer → PNG (geen GPU) |
| `voxel-worldgen` | Deterministische, seeded terrain-generatie |
| `voxel-world` | Multi-chunk wereldstore (cache + edits) |
| `voxel-edit` | Edit-events + append-only log (replay/persistence) |
| `voxel-persist` | Save/load wereld als (seed + edit-log) |
| `voxel-player` | First-person spelercontroller + voxel-collision |
| `voxel-server` | **Headless authoritative server (geen GPU)** |

## Wat je nu kunt runnen

### 1. De headless server (GPU-vrij — het runbare artifact)
```bash
cargo run --example headless_server -p voxel-server
```
Spawn 3 spelers, simuleert 600 ticks, plaatst een "beacon"-edit, en print een
state-samenvatting. Bewijs dat de engine een server draait zónder renderer/GPU.

### 2. Tests (hele workspace)
```bash
cargo test --workspace
```
Verwacht: 48 tests groen.

### 3. Demo-PNG's genereren (software-rasterizer)
```bash
cargo run --example demo        -p voxel-render   # enkele chunk
cargo run --example demo_worldgen -p voxel-render # terrain
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

## Volgende stap (Fase 4)
Netwerk/protocol-laag voor 2–8 spelers multiplayer, daarna de echte Bevy/wgpu-client.

## Budget
Geen betaalde API-calls voor de engine-ontwikkeling; alles lokaal + gratis modellen.
