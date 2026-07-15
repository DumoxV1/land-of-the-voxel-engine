# Autonoom werkplan — Land of the Voxel Engine (nacht van 2026-07-15)

**Doel (gebruiker, voor hij ging slapen):** autonoom 6–7 uur doorwerken aan:
1. Terrein opschalen naar **150 km²** (addressable world via seed; geladen set = view-radius + LOD).
2. **Biome-systeem 3 niveaus** optillen (climate/region → biome → local variation).
3. **Chunk-loader performance** fors omhoog (gen+mesh+stream+cache+GPU upload).
4. **Alle repo-files** reviewen op verbeteringen/upgrades/efficiëntie.
5. **Stappenplan** maken + verbeteringen **uitvoeren** (strict TDD per AGENTS.md).
6. **Cron jobs + sub-agents** inzetten voor zelf-monitoring (herstart bij stilstand).

**Werkwijze:** Nederlands rapporteren, strict TDD (Rood→Groen), geen commits zonder reden,
geen push/destructieve git. Budget: OpenRouter €40/3mnd, ~$27,53 verbruikt, paid-stop $36
→ nog ~$8,47 paid ruimte; gebruik waar mogelijk gratis :free paden (web_search, lokale
tools). Sub-agents erven parent-model — gebruikt voor zwaar parallel leeswerk om eigen
context te sparen (conform rate-limit regel).

## Huidige staat (start nacht)
- 12,5 cm voxels, 4 m chunks (32³). 1 km² = 62.500 chunks. ChunkCoord = i64 (oneindig addressable).
- Vertical-scale spike: surface ~26–28 m (210–220 vox), amplitude 40 m, octaves 2048/512/128.
- BEDROCK_DEPTH=8 (ondergrond tot 1 m onder surface, rest AIR).
- Rayon mesh-pool (non-blocking), LruMeshCache (200k / 12 GB), frustum-culling, VBO 2 GB cap.
- Live client: spawn op surface, 1.90 m avatar, WASD + look. Wit-scherm gefixt (frame-1 guard).
- **Baseline doorvoer:** gen ≈ 2.5 ms/chunk, mesh ≈ 10 ms/chunk (13 ms gecombineerd).
  Lege chunks boven surface verspillen gen-tijd → optimalisatie-kans.
- Shader-spike in renderer.rs: **ONGECommit** (wacht op user A/B/C/D) — niet aanraken deze nacht.

## Lopende research (sub-agents, gestart 2026-07-15 ~06:40)
- deleg_bccfe017 subagent 1: chunk-loader performance plan (gen/mesh/stream/cache/GPU).
- deleg_bccfe017 subagent 2: 150 km² + 3-tier biomes + LOD/streaming ontwerp.
- deleg_bccfe017 subagent 3: per-file code-review (52 files) geprioriteerd.
→ Resultaten komen als message terug; verwerk ze in onderstaand plan.

## Stappenplan (levend — pas aan na elke subagent + elke 3e stap per AGENTS.md)

### FASE A — Performance (chunk-loader)  [HOOGSTE PRIORITEIT]
- A1. Spike: skip lege chunks vóór greedy_mesh (mesh van 100%-AIR chunk = no-op, maar kost tijd).
     Meet: lege-chunk detectie in generate_chunk return → mesh direct [].
- A2. Gen-hot-path: `fbm01` wordt per voxel-Y-sample aangeroepen? slope al gehoist (P1, 14×).
     Check: surface_height_m per (x,z) 1× i.p.v. per (x,y,z). Hash2 goedkoop? Overweeg
     LUT of simd-ish batching per kolom.
- A3. Mesh-pool sizing: 1 core vrij voor render. Test: meer threads = betere gen-doorvoer
     bij grote view-radius. Measure chunks/s op de pool.
- A4. Streaming: view-radius 24 → hoger (voor 150 km² *gevoel*). Combineer met LOD-lite:
     verre chunks op lagere resolutie (elke 2e/4e voxel) of grotere chunks verderop.
- A5. VBO-upload: write_buffer buffering, geen per-frame alloc. Reuse staging.
- A6. Bench: gpu_bench 1 km² + nieuwe 150 km²-view-radius bench. Meet FPS p50/p95.

### FASE B — 150 km² wereldschaal
- B1. Coordinate: ChunkCoord al i64 → 150 km² addressable zonder wijziging. Bevestig
     determinisme over grote afstanden (fbm01 periodes << 150 km, dus geen herhaling).
- B2. Biome 3-tier: `surface_height_m` + `biome_at` herschrijven naar
     climate_noise (continent-period, ~tientallen km) → region (km) → local (chunk).
- B3. View-distance + LOD: gezamenlijk met A4. 150 km² *loaded* niet mogelijk;
     loaded = view-radius (96–200 m) + LOD voor verre horizon.
- B4. Tests: biome-variatie over 150 km (sample op 0/50km/100km/150km), determinism.

### FASE C — Code-review verbeteringen (uit subagent 3)
- C1..Cn: per bevinding uit subagent-3, hoogste-impact eerst, strict TDD.

### FASE D — Verificatie + commit
- D1. Workspace tests groen, live capture (geen wit, variatie), bench FPS verbeterd.
- D2. Commit de geverifieerde verbeteringen (NIET de shader-spike). Push naar eigen repo
     (user gaf volmacht: branch/commit/push naar eigen GitHub zonder vragen).
- D3. PROJECT_STATE.md + alignment-log updaten.

## Self-monitoring
- Heartbeat: schrijf timestamp + laatste stap naar `.hermes/heartbeat.txt` na elke stap.
- Cron: elke 50 min draait een sessie die heartbeat.txt checkt; als >45 min oud/afwezig,
  herstart bij FASE-stap 1 en werkt 45 min door volgens dit plan + AGENTS.md.
- Bij subagent-resultaten: plan bijwerken, daarna verder met FASE A/B/C.

## Risico's / guards
- Niet aan shader-spike (renderer.rs) committen — wacht op user.
- Geen push naar andermans repo / geen destructive git.
- Budget: gratis :free paden eerst; paid alleen binnen $36-limiet.
- Als een optimalisatie de live client breekt (wit/crash): direct terugdraaien + testen.
