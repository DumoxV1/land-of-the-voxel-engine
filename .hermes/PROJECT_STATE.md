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
25. ✅ Mijlpaal 3 / P3 (2026-07-15): **non-blocking rayon-meshing**. Dedicated `rayon::ThreadPool` (1 core vrij voor render) doet `generate_chunk`+`greedy_mesh` off-thread; `crossbeam_channel` stuurt kant-en-klare `Vec<Triangle>` + `generation` terug; `render_frame` vult `mesh_cache` uit de channel binnen per-frame `UPLOAD_BUDGET` (4), discardt stale via gen-counter (camera-beweging → gen+1 → oude in-flight result weg). Strict TDD: unit-test `mesh_chunk_offthread_streams_result` (Rood→Groen). **62/62 tests groen** (60+2 nieuw). Plan: docs/research/2026-07-15-milestone3-rayon-meshing.md. Render-thread blokkeert nooit meer op chunk-gen/mesh.
26. 🐛 Hotfix #1 (2026-07-15): **wit scherm na P3**. Root cause: eerste frames alle chunks `pending`
      → `tris.is_empty()` → `return` vóór surface-clear → wit scherm. Fix: "never go white" guard
      (sync fallback frame 1). **Deze fix was ONVOLLEDIG:** gebruiker bleef wit scherm zien
      (PrintWindow-capture: 92,9% puur wit). Zie item 27 voor de echte rootcause.
27. 🐛 Audit + Hotfix #2 (2026-07-15): **echte wit-scherm rootcause gevonden en gefixt**.
      Onafhankelijke audit (2 subagents) + native window-capture bewezen dat 63/63 groene tests
      een false positive waren voor de live client. Twee bewezen oorzaken, beiden Rood→Groen:
      (a) coördinatenmix (camera-Y in voxels, AABB's in meters → frustum selecteert 0 chunks →
      lege tris → render faalt → fout ingeslikt → nooit gepresenteerd → wit); (b) resize wijzigde
      surface maar niet depth-texture. Fix: centraal contract `mesh_chunk_world_meters()` +
      `spawn_eye_y_m()`; `GpuScene::resize()` vernieuwt width/height/depth samen; live renderfouten
      gelogd i.p.v. ingeslikt. Nieuwe tests: `live_spawn_frustum_contains_at_least_one_world_chunk`,
      `streamed_mesh_is_in_chunk_world_meters`, `resize_recreates_matching_depth_attachment`.
      **Bewijs na fix: PrintWindow-capture 0,002% wit, 434 unieke kleuren, lucht+terrain zichtbaar.**
      **65/65 tests groen** (60+5 nieuw). Benchmark herschreven naar echte 12,5 cm-schaal:
      1 km² = 250×250 = 62.500 chunks, avg_fps≈3750 (RTX 4080). Volgende: Mijlpaal 4 (4K-textures).
27b. ✅ Mijlpaal 4 — P0 (2026-07-15): **4K-texture-system (texture-array + triplanar + PBR)**.
      `GpuScene` krijgt `MaterialPbr`-storage-buffer + albedo `texture_2d_array` + anisotropic
      sampler (clamp 16); WGSL doet triplanar-sample langs wereld X/Y/Z. Strikte TDD: failing test
      `grass_surface_shows_texture_variation_not_flat_tint` bewijst >1 groentint op één vlak
      oppervlak (textuur) i.p.v. flat tint. Bijkomende bug gefixt: VBO-pool groeide voorbij
      `max_buffer_size` (256 MB) bij grote view-radius → client-panic; nu gekapt op device-limiet.
      **Geverifieerd:** live capture 3072 unieke kleuren (was 434), gras/steen met zichtbare
      textuur. Benchmark 1 km² p50=0,24ms, avg_fps≈3636 — géén FPS-daling t.o.v. pre-texture 3753.
      Workspace 36 test-binaires groen. Volgende: P1 (BCn+mipmaps), P2 (echte 2K/4K textures),
      P3 (specular/normal).
27c. ✅ WASD-bewegingsbug gefixt (2026-07-15): **(1) te snel** — `update_camera` telde speed
      per-frame op (geen dt), ~384 m/s bij hoge FPS. Nu `voxel_gpu::free_fly_step(eye,yaw,pitch,
      dt,speed,keys)` met echte frame-dt (8 m/s basis, Shift=4× sprint). **(2) wit bij vliegen**
      — streaming-loop én `nearest_visible_chunk` skipten negatieve chunks (`cx<0||cz<0`),
      terwijl `ChunkCoord` i64 + Euclidean div is en negatieve coords geldig zijn → 0 chunks →
      tris=0 → wit. Guards verwijderd. Strikte TDD: `free_fly_speed_is_frame_rate_independent`
      + `negative_chunk_coords_yield_nonempty_mesh` (Rood→Groen). **Geverifieerd:** live capture
      bij camera 200 m in negatieve ruimte = 7116 kleuren, NEAR_WHITE_PCT=0.1% (niet wit);
      normale spawn = 6575 kleuren, 0.002% wit. 36/36 test-binaires groen.

27d. ✅ Worldgen fBm + biomes (2026-07-15): user-brainstorm over meerdere voxel-groottes →
      onderzoek (deleg_8c81c16d) wees uit: één basis (12,5 cm) + betere gen + texturing wint.
      `height()` = multi-octaaf fBm (64/16/4/2, gewogen), `biome_at()` = climate-noise
      (Meadow/Desert/Snow/Rock), `classify()` biome-bewust + blote rots op steile helling.
      Nieuw SNOW(8) materiaal in worldgen + renderer PBR. TDD: `biomes_vary_across_regions`
      + `terrain_has_fractal_relief` (Rood→Groen); `material_layers_are_sane` aangepast naar
      geldige oppervlaktematerialen. Geverifieerd: capture spawn 8366 kleuren (gras+rots),
      Snow-regio lichtblauw-wit; 36/36 groen. Volgende: M4 P1 (BCn+mipmaps) of Fase-3
      vertex-AO + schaduw + smooth voxels.

27e. ✅ Fase 2 cache-spike (2026-07-15): `mesh_cache` was onbegrensde `HashMap` (RAM-lek).
      Nieuw `cache.rs`: `LruMeshCache` (200k entries / 12 GB RAM, LRU-evict op laatst-zichtbaar)
      + `gpu_resident_set()` (view-LRU stand-in). `App` gebruikt LRU + `frame`-counter + `touch`.
      TDD: 3 nieuwe tests (Rood→Groen). Geverifieerd: live autopilot 15s cirkelvlucht → 9119
      kleuren, NEAR_WHITE=2.4%, CLEAR=0% (LRU evict zonder crash/wit). 36/36 groen. Volgende:
      M4 P1 (BCn+mipmaps) of Fase-3 vertex-AO + schaduw + smooth voxels. Raytracing genoteerd
      als latere fase (zie roadmap — vereist andere pipeline dan huidige mesh-renderer).

27i. ✅ P1 optimalisatie (2026-07-15): chunk-gen hot-path fix. `classify` herberekende
      slope per voxel-Y (32x/kolom) = echte bottleneck. Slope één keer per kolom in
      `generate_chunk`, als param aan `classify`. **3.185→0.226 ms/chunk (14x)**.
      `chunk_gen_stays_fast` regression-test groen; 36/36 groen. Volgende: P2.
      + device `required_limits.max_buffer_size` gespiegeld. Geverifieerd: UNIQUE_COLORS
      3316→6366 (2× variatie), VBO-warn=0, 36/36 groen. Volgende: P1 fBm-profiling,
      P2 requested_gen-groeiguard (zie docs/OPTIMIZATION_BACKLOG.md).
      VBO-panic. Y-streaming per-kolom gebound (max_cy), UPLOAD_BUDGET 4->64, VBO_BYTES_CAP
      256MB gate in streaming-loop, renderer hard-truncate verts op vbo_capacity (crash-proof).
      Geverifieerd: geen panic, UNIQUE_COLORS=3316, NEAR_WHITE=0.002%, CLEAR=0%, 36/36 groen.
      #1 optimalisatie-target volgende sessie: VBO-vergroting OF LOD/clipmap voor volledige
      view-distance + 150-200m filmische schaal.
      → mens oogde reusachtig. Nieuw `surface_height_m` (fBm, ampl 40m, octaves
      2048..4 voxels). `generate_chunk` itereert nu WORLD-Y (multi-chunk-Y), `gpu_window`
      streamt Y-lagen 0..=12 + spawn-eye ~15m boven surface. TDD: 3 nieuwe tests
      (Rood→Groen) + 3 gecascade tests gefixt (player/server/world). Geverifieerd: spawn
      "terrain top=210 vox (~26m)" (was 3,875m); live capture UNIQUE 863→3379 (4× variatie),
      NEAR_WHITE=0.27%, CLEAR=0% (geen wit). 36/36 groen. Echte 150-200m filmische
      schaal = Fase 5 (LOD/clipmap), niet nu (blaaast VBO zonder LOD).

27j. ✅ Eerste loopbaar karakter (2026-07-15): 1.90 m avatar met voxel-collision.
      `voxel-player` HALF[1] 0.9→0.95 (PLAYER_HEIGHT_M=1.90 pub). `gpu_window` switcht van
      free-fly naar `Player`/`PlayerController`: WASD→Input, Space=jump, mouse-look→yaw,
      camera.eye = player.pos + ooghoogte (~1.7m). Spawn: voeten op surface (center_wx/z).
      Test `player_is_1_90_m_tall` + bestaande collision-audits groen; 36/36 groen.
      Live: spawn eye_y ~= 28.08 m (was 41.25 fly). Geen regressie (CLEAR=0%, VBO-warn=0).
27k. 📋 Onderzoek voxel-loading (2026-07-15): memo docs/research/voxel-loading-standard.md.
      Bevinding: ondergrond wordt volledig gegenereerd (tot y=0) maar NIET getekend (greedy
      mesh = face-culling, alleen shell). Standaard = volledige storage + shell-mesh.
      Verspilling zit in generate (ondergrond tot bodem). Aanbeveling: BEDROCK_DEPTH=8
      (1m) in generate_chunk (P1-verbetering, geen render-regressie). Kanban-vraag open.

27l. ✅ Performance + 150 km²-nacht (2026-07-15, autonome sessie): 3 research-subagents
      (chunk-loader perf, 150km²+biomes-3tier, per-file review) + uitvoering. Geverifieerd
      (83/83 tests groen, live capture UNIQUE=3123 / NEAR_WHITE=0.1% bij view-radius 48):
      - T2 LruMeshCache bytes_per_tri 32→52 (RAM-cap correctheid).
      - T3 coords.rs euclidean→std div_euclid/rem_euclid.
      - T4 gpu_window redundante tris.clone() verwijderd (per-frame churn weg).
      - T5 LruMeshCache O(N²) eviction→incrementele total_tris-teller.
      - T6 greedy_mesh nested Vec<Vec<>>→flat buffer (alloc-reductie).
      - T8 **3-TIER BIOMES**: Region(klimaat)→Biome(7 types)→LocalParams(rock/dune/
        forest/snow), surface_height_m 3-tier, classify aangepast, gen geoptimaliseerd
        (height-buffer + gedeelde region-fBm). Walkability behouden (<1 m/vox).
      - T9 view-radius 24→48 (~192 m) + radiale cull (schijf i.p.v. vierkant, ~22%
        minder kolommen). 150 km² = addressable wereld (i64), LOD/clipmap nog Fase 5.
      - T1 (hash-constante "fix") TERUGGEROEPEN: nieuwe constante gaf vlakkere terrain
        (fine-detail 0.5→0.0 m) — kwantiteit≠kwaliteit, oude hash beter voor variatie.
      - Shader-spike in renderer.rs: ONGEcommit (wacht op user A/B/C/D).

27m. ✅ FASE A vervolg (2026-07-15, autonome CRON-herstart na stilgevallen hoofd-sessie):
      chunk-loader hot-path A1+A2, strict TDD, 88/88 groen (was 83, +5 nieuw).
      - **A1** `Chunk::is_empty()` (O(1) voor uniform) → `mesh_chunk_world_meters` slaat
        `greedy_mesh` (~196k neighbour-probes + 6 allocs) over voor lege chunks.
        Tests: uniform_air/solid + one-solid-voxel (voxel-core).
      - **A2** `generate_chunk` early-out: (1) O(1) globale bound `MAX_SURFACE_M` (123 m) →
        chunks boven het surface-plafond meteen leeg terug, zonder height-buffer/fBm;
        (2) exacte per-kolom envelope-check voor onder-bedrock/boven-surface chunks.
        Tests: max_surface_bound_covers_real_terrain (veiligheid: bound > echte max),
        air_chunk_gen_is_cheap_and_empty (voxel-worldgen).
      - **Meting (release, view-volume r48, cy 0..=14, 108.195 chunks, 91% air):**
        baseline 933 chunks/s (1,07 ms/chunk) → **3862 chunks/s (0,26 ms/chunk) = 4,1×**,
        IDENTIEKE output (3.693.532 tris in beide → geen visuele regressie).
      - **Verificatie-tooling hersteld** (was stale sinds fBm-lift, mat lege scène):
        `client_smoke` en `gpu_bench` streamden hardcoded cy=0 (onder de opgetilde surface
        = AIR) → 0 frames. Nu berekenen ze de echte surface-cy + streamen Y-lagen via
        `mesh_chunk_world_meters`. `client_smoke`: **120/120 frames** (spawn top=216 vox
        ~27 m), geen panic, geen wit. `gpu_bench` 1 km² r48: **p50=9,33 ms / avg 93,8 FPS /
        243k zichtbare tris/frame** (echte scène i.p.v. de oude lege-scène "3636 fps").
      - renderer.rs (shader-spike) NIET aangeraakt/gecommit door deze sessie. Opmerking:
        de spike blijkt inmiddels als commit 7b23b89 vastgelegd (buiten deze sessie om).

27n. ✅ FASE A vervolg #2 (2026-07-15, 2e autonome CRON-herstart na opnieuw stilgevallen
      hoofd-sessie): worldgen **per-kolom height-buffer cache**, strict TDD, workspace
      **91/91 groen** (was 89, +2 nieuw). GEEN destructieve git; renderer.rs shader-spike
      + WIP (walk/fly/step-up in voxel-player + gpu_window) ONaangeraakt gelaten.
      - **Bevinding:** `surface_height_m` is een zuivere functie van wereld-X/Z (onafhankelijk
        van chunk.y). `generate_chunk` bouwde tóch de volledige (n+2)²≈1156-cel height-buffer
        opnieuw voor ELKE Y-slab, terwijl de client per kolom `cy in 0..=max_cy` (~7-8 slabs)
        streamt → tot ~7× redundante fBm-herberekening per kolom. De ~6 diepe all-air chunks
        per kolom betaalden de volle buffer-kost alleen om via de A2-envelope weg te vallen.
      - **Fix:** thread-local LRU (`COLUMN_HBUF_CACHE`, cap 64) keyed op (cx,cz,seed); buffer
        één keer bouwen, delen over alle Y-slabs via `Rc<Vec<f32>>`. Per-thread → rayon mesh-pool
        heeft geen lock nodig; determinisme behouden (buffer = zuivere resultaten). Byte-identieke
        output (envelope + fill ongewijzigd).
      - **Tests (Rood→Groen):** `column_reuse_is_faster_than_distinct_columns` (zelfde kolom
        markant sneller dan N distincte kolommen) + `column_cache_preserves_determinism_and_seed_isolation`
        (determinisme na cache-churn; geen seed-collisie). Bestaande spike_s04 (deterministisch/
        boundary-continu/material-layers) blijven groen = correctheidsgarantie.
      - renderer.rs (shader-spike) NIET aangeraakt; alleen `crates/voxel-worldgen/src/lib.rs`
        gewijzigd + gecommit (geïsoleerd, zoals 27m's veilige patroon).

27o. ✅ **TERRAIN GEN 2.0 — 3D DENSITY FIELD (Stap 4, 2026-07-15):** platte 2D-hoogtekaart-shell
      vervangen door hybride density-field (optie A, door gebruiker gekozen). De walkable
      heightfield blijft de grond; een 3D fBm-warp (`fbm3`, nieuw) duwt overhangs/richels
      BOVEN de surface (alleen omhoog, zodat de grass-cap + walkability intact blijven), en een
      aparte 3D cave-noise boort grotten in een band van ~12 m onder de surface. Nieuw:
      `OVERHANG_AMP_VOX=6`, `CAVE_BAND_DEPTH=96`, `CAVE_THRESH=0.5`, `fbm3`+`hash3` (8-hoek
      trilinear), `MAX_SOLID_M` (surface+overhang bound voor early-out). De ondergrond is nu een
      SOLID body tot y=0 (geen 1-voxel shell meer → zijwanden + caves zichtbaar). Envelope/
      `column_solid_cy_range` vereenvoudigd naar [0, max_h+overhang]. `classify` geeft STONE voor
      voxels boven de heightfield (overhangs renderen echt). Strict TDD: nieuwe test
      `terrain_has_caves_and_overhangs` (Rood→Groen) + 5 bestaande tests aangepast aan de
      solid-body (overhang/cave/multi-layer/stone-body/underground-truncate). **19/19 worldgen
      tests groen** (14 lib + 5 spike), workspace 36/36 binaries groen, 28/28 GPU-lib (zonlicht)
      onaangetast. Ad-hoc verificatie (8×8 kolommen, seed 7): 2/64 kolommen overhang (910 vox),
      16/64 kolommen cave (276k air-vox) → **PASS**. Release `gpu_window_main` herbouwd; spawn
      top=216 vox (~27 m). Run voor visuele check:
      `cargo run --release --example gpu_window_main -p voxel-client`.

27p. ✅ **TERRAIN 2.0 — OVERHANG-AMPLITUDE VERGROOT (2026-07-15, autonome keuze optie A):**
      gebruiker gaf volmacht ("doe wat jij goed vind, autonoom"). `OVERHANG_AMP_VOX` 6→28
      (~3,5 m richels/kliffen), `OVERHANG_AMP_CEIL` 6→28, en `OVERHANG_OCTAVES` van 1 breed
      octaaf naar 2 (128 vox @0,7 + 48 vox @0,3) voor gevarieerde overhangs in plaats van één
      vlakke slab. Walkability + grass-cap blijven intact (alleen-omhoog-warp, surface-term
      domineert bij y≈surface). `max_surface_bound_covers_real_terrain` test-bound 6→28 vox.
      **19/19 worldgen groen**, workspace 36/36, 28/28 GPU-lib onaangetast. Ad-hoc verificatie
      (24×24, seed 7): **409/576 kolommen overhang** (was ~3%), **166/576 cave** → wereld is nu
      rijk aan richels/kliffen + grotten. Release `gpu_window_main` herbouwd.

Researchmemo's zijn input, geen waarheid. Een steekproef vond foutieve actualiteitsclaims en niet-onderbouwde benchmarkgetallen. Geen cijfer of stackadvies wordt overgenomen zonder onafhankelijke broncontrole of lokaal experiment.

## Actieve automatisering
- Dagelijkse no-agent plan-alignmentguard.
- Wekelijkse no-agent OpenRouter-budget- en free-modelguard.
- Wekelijkse read-only governance-review op `openrouter/free`.
- Vier gespecialiseerde profielen, alle gepind op `openrouter/free`.

## Menselijke input
Alleen noodzakelijke vragen worden als geblokkeerde Kanban-kaarten gesteld. Geen menselijke code-review vereist; wel menselijke toestemming voor uitgaven, publicatie, accounts, grote scopewijzigingen en destructieve acties.
