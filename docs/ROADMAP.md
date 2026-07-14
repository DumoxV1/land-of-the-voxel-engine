# Routekaart — Land of the Voxel Engine

**Doel:** een technisch grensverleggende, filmische 3D micro-voxel openwereld-RPG-engine op
eigen Rust-fundament. Noordster: "de GTA VI / Crimson Desert onder micro-voxel-engines" —
rijke werelddichtheid, dynamiek en schaal, zónder beschermde assets/personages te kopiëren.

**Status (2026-07-15):** S-01..S-10 voltooid en op `origin/main`. De engine rendert nu op de
GPU (wgpu/Vulkan, RTX 4080). Eerste runbare checkpoint = vertical slice (headless server) +
eerste GPU-render.

---

## Wat er staat (bewijsbaar, strict TDD)

| Spike | Crate | Wat | Bewijs |
|-------|-------|-----|--------|
| S-01 | voxel-core | chunk-states (Uniform/PalettePacked/Dense), 4-bit bitpacking, per-chunk palette (≤16), byte-stabiel versie-2 formaat | 15 tests |
| S-03 | voxel-render | software-raster (Camera + z-buffer, per-normaal shading) → PNG | 3 tests, demo-PNGs |
| S-04 | voxel-worldgen | `generate_chunk(coord, seed)`: seeded value-noise heightmap, grass/dirt/stone lagen | 5 tests, demo_worldgen.png |
| S-05 | voxel-world | multi-chunk World-store (HashMap cache + seed + edits + dirty-set) | 4 tests |
| S-06 | voxel-edit | `Edit` + append-only `EditLog` + `EditTool` (place/remove) | 4 tests |
| S-07 | voxel-persist | eigen binair formaat (magic `VWL1` + seed + edits), save/load | 3 tests, demo_persist.png |
| S-08 | voxel-player | `Player` + `PlayerController` (axis-separated collision, sub-stepping, `resolve_floor_y`) | 4 tests, demo_player.png |
| S-09 | voxel-server | headless authoritative server (World + EditLog + spelers), geen GPU | 4 tests, headless_server example |
| S-10 | **voxel-gpu** | **wgpu/Vulkan GPU-renderer** (greedy_mesh → GPU, WGSL shading) | gpu_world.png, probe.png |

**Architectuurprincipes (ADR's):** renderer-agnostische core (ADR-0002), server-authority +
determinisme + versieerbare data + sparse/procedurele wereld (ADR-0003), client-shell = Rust +
Bevy/wgpu (ADR-0004, status Proposed).

---

## Routekaart (fasen)

### Fase 2 — GPU-client shell (BEZIG, S-10 gedaan als opmaat)
- [x] S-10: offscreen wgpu-renderer bewijst GPU-pad op RTX 4080 (Vulkan).
- [ ] **Interactieve GPU-client**: winit-venster + render-loop + camera-input (WASD + muis).
      Vervang offscreen-PNG door een live venster dat de `World` rendert.
- [ ] Chunk-streaming: render alleen chunks in view-range; background-mesh + upload naar GPU.
- [ ] Spelercontroller koppelen aan de GPU-camera (first-person/third-person).
- [ ] Fase-2 benchmark-gate (ADR-0004 lock-in-voorwaarde): B-06 determinisme-replay,
      B-07 headless 2–8 client soak, **FPS op 1 km²** op de RTX 4080. Pas daarna ADR-0004
      naar Accepted.

### Fase 3 — Werelddichtheid & content (opschalen)
- [ ] Grotere/warmere biome-diversiteit in worldgen (bossen, water, rotsformaties).
- [ ] Meshing-verbetering: smooth/beveled voxels optie naast blocky (visuele lat dichter
      bij *Lay of the Land*).
- [ ] Material/lighting pipeline: normals per-voxel, ambient occlusion, sky/zon-licht,
      schaduwkaarten (richting naar filmische look).
- [ ] Edit-tool live in de GPU-client (place/remove met muis).

### Fase 4 — Multiplayer (netwerk/protocol)
- [ ] Netwerklaag bovenop de headless `voxel-server`: 2–8 spelers, snapshot-interpolatie.
- [ ] Client-server protocol: input → server tick → authoritative state → client render.
- [ ] Determinisme-behoud over het netwerk (seed + edit-log replay, ADR-0003).

### Fase 5 — Schaal & persistentie (doel: ~150 km²)
- [ ] Sparse/procedurele wereldopslag voor 150 km² (chunk-streaming + regionele servers).
- [ ] Persistentie-laag uitbreiden (S-07) naar multi-region, versieerbaar.
- [ ] Late-game: Bevy-integratie voor scene/ECS, of wgpu-native loop (afhankelijk van
      Fase-2 gate).

### Fase 6 — Gameplay-RPG-dikte
- [ ] Entiteiten/agents, quests, economie bovenop de server-authoritative basis.
- [ ] Filmische presentatie: post-processing (bloom, tonemapping), audio, UI.

---

## Harde grenzen (governance)
- Geen push/publicatie/deployment zonder menselijke goedkeuring (grens A = push naar eigen
  repo is wél toegestaan, gedaan voor S-01..S-10).
- Geen betaalde LLM-calls; budget €40/3 maanden, stop onder $36. Gratis OpenRouter-modellen
  standaard voor research/review.
- Compiler/tests/benchmarks leidend; geen "klaar" zonder echte uitvoering + verificatie.
- Correctheid en speelbaarheid gaan vóór maximale dichtheid.

## Directe volgende stap (autonoom)
Fase 2 venster + input-loop zodat de GPU-client interactief wordt, daarna de Fase-2
benchmark-gate (FPS) vóór ADR-0004 lock-in.
