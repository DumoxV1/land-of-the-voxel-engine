# Project State

**Canoniek plan:** `.hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md`  
**Status:** researchreview en plansynthese actief  
**Actieve fase:** Fase 2 — GPU-client shell (S-10/S-11/S-12a/S-12b gereed; volgende: Fase-2 benchmark-gate vóór ADR-0004 lock-in)  
**Laatste update:** 2026-07-15

## North star
Een filmische, zeer rijke en dynamische openwereld-RPG op een eigen micro-voxelfundament — ambitieus als "de GTA VI / Crimson Desert onder micro-voxel-engines", maar ontwikkeld via meetbare technische gates.

## Huidige beslissingen
- Geen volledige MMO in de eerste twaalf weken; eerst een vertical slice.
- Procedurele basiswereld + sparse persistente wijzigingen.
- Eigen voxel/world/network/persistence-kern; commodity-platformfuncties mogen uit open source komen.
- Blocky versus smooth en clientshell worden beslist via gelijke benchmarks, niet voorkeur (client-shell = Rust+Bevy/wgpu per ADR-0004, status Proposed, lock-in pas na Fase-2 benchmark-gate).
- Gratis OpenRouter-modellen zijn standaard voor research en eerste reviews.

## Werkprotocol
Na elke derde voltooide uitvoeringsstap wordt de vorige stap opnieuw gecontroleerd en wordt plan-alignment expliciet vastgelegd in `docs/governance/alignment-log.md`.

**Status:** researchreview en plansynthese VOLTOOID  
**Actieve fase:** Fase 2 — GPU-client shell (interactieve wgpu-client draait op RTX 4080; Fase-2 benchmark-gate is de volgende harde gate vóór ADR-0004 lock-in)  
**Laatste update:** 2026-07-15

## Voltooide gates & spikes (strict TDD)
1. ✅ Onafhankelijke gratis reviewer corrigeerde bewijs, licenties, verzonnen/ongeverifieerde metrics en scope (review-initial-bundle.md, B-01…B-08).
2. ✅ Gratis architect synthetiseerde uitsluitend geverifieerde resultaten naar ADR-spikes (adr/0001–0003) en planupdates.
3. ✅ Exact implementatieplan voor de eerste `voxel-core` tracer bullet (S-01) geschreven.
4. ✅ S-01 onder strict TDD: failing tests eerst (rood), dan implementatie (groen). Repo scaffold + `voxel-core` crate.
5. ✅ S-01-hardening: drie chunk-states (`Uniform`/`PalettePacked`/`Dense`) + 4-bit bitpacking + per-chunk palette (≤16), byte-stabiel versie-2 formaat.
6. ✅ S-02 mesher-spike: `voxel-mesher` (naive→culled→greedy), waterdichte geometrie, geen renderer-dep.
7. ✅ S-03 software-raster spike: `voxel-render` crate, `Camera` (perspectief) + `render_scene` (z-buffer, per-normaal shading) → PNG. Renderer-agnostisch.
8. ✅ S-04 deterministische worldgen spike: `voxel-worldgen` crate, `generate_chunk(coord, seed)` (seeded value-noise heightmap, grass/dirt/stone lagen).
9. ✅ S-05 multi-chunk world-store spike: `voxel-world` crate, `World` (HashMap cache + seed-generatie + edits + dirty-set).
10. ✅ S-06 edit/place-remove tool + edit-events: `voxel-edit` crate, `Edit` + `EditLog` (append-only) + `EditTool::place/remove`.
11. ✅ S-07 persistence (save/load seed+edits): `voxel-persist` crate, eigen binair formaat (`VWL1`).
12. ✅ S-08 spelercontroller + voxel-collision: `voxel-player` crate, `Player` + `PlayerController` (axis-separated collision, sub-stepping, `resolve_floor_y`).
13. ✅ ADR-0004 (client-shell): subagent-dossier → **Rust + Bevy/wgpu** gekozen (pure-Rust core native, geen FFI; Godot GDExtension afgewezen voor eerste slice). Status Proposed; Fase-2 benchmark-gate (B-06/B-07 + FPS) blijft verplicht voor lock-in.
14. ✅ S-09 headless dedicated server: `voxel-server` crate. **RUNNABLE ARTIFACT**: `cargo run --example headless_server -p voxel-server`. Vertical slice bereikt (S-01..S-09).
15. ✅ S-10 GPU-renderer (wgpu/Vulkan, RTX 4080): `voxel-gpu` crate. `probe` bewijst wgpu init + offscreen readback; `renderer` neemt `greedy_mesh`-triangles → GPU, WGSL-shader met per-normaal belichting + fog + materiaal-tinten. `examples/gpu_world.rs` → `gpu_world.png` (16.270 tris). Eerste GPU-render van de engine. wgpu gepind op 0.17.2 destijds.
16. ✅ S-11 audit-hardening: onafhankelijke code-audit (6 hoog + 13 middel) → alle hoge gefixt onder strict TDD (9 nieuwe tests RED→GREEN, workspace 57/57): mesher-vlakken+winding, serialize v3 (i64-coords, nibble-validatie), player terminal-velocity + footprint-floor, server tick-volgorde, atomaire saves, gpu tints/fog/backends.
17. ✅ S-12a (Fase-2b #1): `World::material_at` zonder chunk-clone (audit #12); player collision gebruikt cheap reader i.p.v. 32KB-clone per voxel-sample. 58/58 tests groen.
18. ✅ S-12b (Fase 2a): wgpu 0.17→30 + winit 0.30 interactieve client. `gpu_window` example (ApplicationHandler, surface-render, WASD+muis-look free-fly camera), gedeelde pipeline met offscreen-pad. Geverifieerd: "GPU scene initialized (Bgra8UnormSrgb)". Fase 2a voltooid.
19. 🎯 **Volgende harde gate: Fase-2 benchmark-gate** (B-06 determinisme-replay, B-07 headless 2–8 client soak, **FPS op 1 km²** op RTX 4080) vóór ADR-0004 lock-in. De 1 km²-meting bepaalt óók de LOD-strategie (advies #5, data-gedreven). Daarna: chunk-streaming (S-12 deel 3, advies #2), binary greedy meshing (advies #3), spelercontroller↔GPU-camera-koppeling.
20. ✅ S-12c deel 1 (Fase-2 FPS-gate, 2026-07-15): GPU-benchmark-harness (`examples/gpu_bench.rs` + `GpuScene::render_triangles` offscreen-pad) meet FPS op 1 km² (1024 chunks, RTX 4080). **Uitslag: 8,8 avg FPS bij 1,25 M zichtbare tris/frame (p50 129 ms / p95 141 ms) — gate NIET gehaald.** Oorzaak = scene-samenstelling (geen frustum-culling, geeen distance-budget, per-frame VBO), niet de renderer-pipeline. S-12c deel 2: frustum-culling + triangle-distance-budget + buffer-pooling in de streamer, dan hermeten. Zie `docs/benchmarks/2026-07-15-fase2-fps-1km2.md`. ADR-0004 lock-in uitgesteld totdat de scene efficienter is.
21. ✅ S-13 micro-voxel resolutie (2026-07-15, ADR-0005): **1 voxel = 12,5 cm (0,125 m)**, binnen gebruikersband 9,5–13,5 cm (Lay of the Land / Voxtopolis / John Lin / Tantan-richting). Chunk blijft 32³ voxels → **chunk = 4 m** (was 32 m); 1 km² = **62.500 chunks** (was 1.024). Strict TDD: `spike_s13.rs` (2 failing tests → groen). Camera's + bench op 12,5 cm-schaal; `gpu_world.png` geregenereerd; 60/60 tests groen. "Micro"-effect zichtbaar bij first-person/1 km² (S-12c deel 2 frustum-culling vereist).
22. ✅ S-13b live micro-voxel client (2026-07-15): `gpu_window` herschreven naar **live chunk-streaming first-person client** op 12,5 cm. Wereld = lazy (on-demand chunks), view-radius 24 (~96 m), spawn op terrain-hoogte, WASD + links-drag look. Headless smoke-test (`client_smoke`) bevestigt: 120/120 frames, géén panic. Run: `cargo run --release --example gpu_window -p voxel-gpu` (vereist GPU/scherm — gebruiker draait zelf). README + ROADMAP + PROJECT_STATE bijgewerkt.
23. ✅ Mijlpaal 1 (2026-07-15): **space-crash gefixt**. `gpu_window` crashte bij Space (focus-wissel → `get_current_texture()` `Lost`/`Outdated`, surface nooit hersteld). Nu: herconfigureert bij `Lost`/`Outdated` op laatst bekende grootte + skipt; `Timeout`/`Occluded` → skipt. Gepushed (3d2157e).
24. ✅ Mijlpaal 2 / S-12c deel 2 (2026-07-15): **lag fix P0+P1**. `Frustum` (6 planes uit view_proj, AABB-test) + unit-test (Rood→Groen) in `renderer.rs`; GPU buffer-pooling (één herbruikbare VBO via `write_buffer`, géén per-frame `create_buffer_init`). Client past frustum-culling per chunk toe. **61/61 tests groen** (60+1 nieuw). Bench 1 km²: **8,8 → 15,8 avg FPS** (p50 129→55 ms = 2,3×). Onderzoek (3 subagents): P0+P1 samen ~10–30× potentieel.

## Auditwaarschuwing
Researchmemo's zijn input, geen waarheid. Een steekproef vond foutieve actualiteitsclaims en niet-onderbouwde benchmarkgetallen. Geen cijfer of stackadvies wordt overgenomen zonder onafhankelijke broncontrole of lokaal experiment.

## Actieve automatisering
- Dagelijkse no-agent plan-alignmentguard.
- Wekelijkse no-agent OpenRouter-budget- en free-modelguard.
- Wekelijkse read-only governance-review op `openrouter/free`.
- Vier gespecialiseerde profielen, alle gepind op `openrouter/free`.

## Menselijke input
Alleen noodzakelijke vragen worden als geblokkeerde Kanban-kaarten gesteld. Geen menselijke code-review vereist; wel menselijke toestemming voor uitgaven, publicatie, accounts, grote scopewijzigingen en destructieve acties.
