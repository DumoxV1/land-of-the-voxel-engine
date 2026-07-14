# Routekaart — Land of the Voxel Engine

**Doel:** een technisch grensverleggende, filmische 3D micro-voxel openwereld-RPG-engine op
eigen Rust-fundament. Noordster: "de GTA VI / Crimson Desert onder micro-voxel-engines" —
rijke werelddichtheid, dynamiek en schaal, zónder beschermde assets/personages te kopiëren.

**Status (2026-07-15):** S-01..S-11 voltooid. S-11 = audit-hardening: onafhankelijke code-audit
(6 hoge + 13 middel bevindingen) → alle hoge bevindingen gefixt onder strict TDD (57 tests groen).
De engine rendert op de GPU (wgpu, RTX 4080) mét backface-culling en correcte geometrie.
**S-12 deel 1** (clone-fix, 58/58 tests) en **S-12 deel 2** (Fase 2a: wgpu 0.17→30 + winit 0.30
interactieve client) zijn af. Onderzoeksadvies vervolg: `docs/research/2026-07-15-sota-advies-vervolg.md`.

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
| S-10 | **voxel-gpu** | **wgpu GPU-renderer** (greedy_mesh → GPU, WGSL shading, backface-culling) | gpu_world.png, probe.png |
| S-11 | (alle) | **audit-hardening**: mesher-vlakken op juiste planes + CCW-winding, serialize v3 (i64-coords, nibble-validatie), player terminal-velocity + footprint-floor-resolve, deterministische tick-volgorde, atomaire saves, gpu-kleuren/fog/backends gefixt | 9 nieuwe tests (RED→GREEN), 57 totaal |
| S-12a | voxel-world | **clone-fix (audit #12)**: `World::material_at` zonder chunk-clone; player collision gebruikt cheap reader (geen 32KB-clone per voxel-sample) | 1 test, 58 totaal |
| S-12b | **voxel-gpu** | **Fase 2a**: wgpu 0.17→30 + winit 0.30 interactieve client — `gpu_window` example (ApplicationHandler, surface-render, WASD+muis-look), gedeelde pipeline met offscreen-pad | gpu_window draait op GPU, gpu_world PNG intact (16.270 tris) |

**Architectuurprincipes (ADR's):** renderer-agnostische core (ADR-0002), server-authority +
determinisme + versieerbare data + sparse/procedurele wereld (ADR-0003), client-shell = Rust +
Bevy/wgpu (ADR-0004, status Proposed).

---

## Routekaart (fasen)

### Fase 2 — GPU-client shell (BEZIG, S-10/S-11/S-12b gedaan)
- [x] S-10: offscreen wgpu-renderer bewijst GPU-pad op RTX 4080.
- [x] S-11: audit-hardening (geometrie, robustheid, determinisme, physics) — zie tabel.
- [x] **S-12b (Fase 2a): wgpu 0.17 → 30 + winit 0.30** `ApplicationHandler`-patroon. Nieuw
      `gpu_window` example: live venster, WASD+muis-look free-fly camera, gedeelde pipeline
      met offscreen-pad. Geverifieerd: "GPU scene initialized (Bgra8UnormSrgb)".
- [x] **Interactieve GPU-client**: winit-venster + render-loop + camera-input (WASD + muis).
      Vervangt offscreen-PNG door een live venster dat de `World` rendert (S-12b).
- [ ] **Fase-2 benchmark-gate**: B-06 replay + B-07 soak + FPS op 1 km² vóór ADR-0004 lock-in.
- [ ] Chunk-streaming (advies #2): dedicated rayon-pool + kanalen, afstand-geprioriteerde
      queue met generation-counters, upload-budget per frame, buffer-pooling.
      Chunk-key alvast `(x, y, z, lod)` zodat LOD later geen herschrijf vergt (advies #5).
- [ ] Meshing-versnelling (advies #3): binary greedy meshing (Tantan / `binary-greedy-meshing`
      crate) als drop-in; eerst benchmark oude vs nieuwe mesher. Neighbor-aware meshing
      (audit #19: dubbele faces op chunk-naden) hierin meenemen.
- [ ] Spelercontroller koppelen aan de GPU-camera (first-person/third-person).
- [ ] Fase-2 benchmark-gate (ADR-0004 lock-in-voorwaarde): B-06 determinisme-replay,
      B-07 headless 2–8 client soak, **FPS op 1 km²** op de RTX 4080. Pas daarna ADR-0004
      naar Accepted. De 1 km²-meting bepaalt óók de LOD-strategie (advies #5).

### Fase 2b — Technische schuld (uit audit, middel-prioriteit; oppakken bij aanraken van de code)
- [ ] `World::get`/`material_at` zonder chunk-clone (audit #12: nu 32 KB clone per voxel-sample
      in collision — grootste bekende perf-lek; fixen vóór de FPS-benchmark).
- [ ] Server fixed-timestep (audit #8: `dt`-parameter is determinisme-lek over netwerk).
- [ ] Persist: apart versiebyte + trailing-garbage afkeuren + CRC32 (audit #11); revision
      behouden of schrappen (audit #9).
- [ ] `voxel_core::edit` vs `voxel_edit::Edit` fuseren (audit #18, dubbel concept).
- [ ] Fuzz-target `Chunk::from_bytes` (AGENTS.md belooft fuzzing; nibble-check was een gat).
- [ ] voxel-gpu unit-tests voor CPU-kant (`view_proj`, `material_tint`, vertex-layout).

### Fase 3 — Werelddichtheid & content (opschalen)
- [ ] Worldgen-verdieping (advies #6): gelaagde noise-velden (continentalness/erosie/moisture)
      + spline-mapping + biome-lookup-tabel; rivieren via globale pre-simulatie (Veloren-model).
- [ ] Belichting (advies #4): vertex voxel-AO (0fps-methode, let op quad-flip pitfall) +
      één cascaded shadow map voor de zon. Voxel-GI/raytracing uitstellen tot dit staat.
- [ ] Meshing-verbetering: smooth/beveled voxels optie naast blocky (visuele lat dichter
      bij *Lay of the Land*).
- [ ] Edit-tool live in de GPU-client (place/remove met muis).

### Fase 4 — Multiplayer (netwerk/protocol)
- [ ] Netwerklaag bovenop de headless `voxel-server` (advies #7): snapshot-interpolatie voor
      entiteiten (20–30 Hz + ~100 ms buffer, Gaffer-model); betrouwbare voxel-edit-log met
      tick-nummers; delta's i.p.v. chunk-resync; zstd-compressie.
- [ ] Client-server protocol: input → server tick → authoritative state → client render.
- [ ] Determinisme-behoud over het netwerk (seed + edit-log replay, ADR-0003; vereist
      Fase-2b fixed-timestep).

### Fase 5 — Schaal & persistentie (doel: ~150 km²)
- [ ] LOD (advies #5): chunked octree/clipmap-ringen, 2× downsampling per niveau; naadstrategie
      (skirts) vroeg kiezen — naden zijn het echte risico, niet de datastructuur. Ontwerp
      data-gedreven op basis van de Fase-2 1 km²-benchmark.
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
1. wgpu/winit-upgrade (Fase 2, advies #1) — voorwaarde voor de interactieve client.
2. Interactieve GPU-client (venster + input) met `World::get`-zonder-clone fix (Fase 2b #1)
   ervóór of ertijdens, zodat de FPS-benchmark niet vertekend wordt.
3. Fase-2 benchmark-gate (FPS op 1 km²) vóór ADR-0004 lock-in.

## Workflow-verbeteringen (vastgesteld 2026-07-15, S-11-les)
- **Tests moeten posities/adversarial input asserteren, niet alleen aantallen** — de twee
  ernstigste bugs (mesher-planes, corrupte-payload-panic) waren onzichtbaar voor count-based
  tests. Nieuwe spikes: minimaal één golden-test op exacte waarden + één malformed-input test.
- **Onafhankelijke audit na elke fase-afronding** (gedelegeerd aan gratis model, bevindingen
  door implementer in de bron geverifieerd vóór fixen). S-11 bewees de waarde: 6 echte bugs.
- **Runtime-artifacts horen niet in git**: `.hermes/reports/` staat nu in `.gitignore`.
- **Visuele verificatie is geen bewijs van geometrische correctheid**: gpu_world.png zag er
  "goed" uit met een platgeslagen kubusgeometrie (fout gemaskeerd door cull_mode: None).
