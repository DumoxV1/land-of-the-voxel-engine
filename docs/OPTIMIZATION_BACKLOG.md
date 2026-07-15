# Optimalisatie-backlog — Land of the Voxel Engine

Kanban-opstap voor de zuivere-optimalisatiesessie. Gesorteerd op verwachte winst vs.
risico. Alle code is groen (36/36) en gepusht tot `39be666`; deze lijst is de volgende
stap, niet een correctheidswijziging.

## Context (waarom dit nu)
De vertical-scale spike (terrain ~40m, multi-chunk-Y) vermenigvuldigde de streamed
chunk-set ~13x. Fallout gefixt in `39be666`:
- Y-streaming per kolom gebound (`max_cy` uit `surface_height_m`) — slaat lege slaben over.
- `UPLOAD_BUDGET` 4 -> 64.
- `VBO_BYTES_CAP = 256 MB` gate in streaming-loop (stop aanvragen boven cap).
- Renderer: `verts` hard getruncate op `vbo_capacity` vóór `write_buffer` (crash-proof).

**Huidige harde limiet:** 256 MB VBO = plafond voor zichtbare terrain. Alles erboven
wordt getruncate (geen crash, maar incomplete wereld bij grote view-distance).

## P0 — VBO-cap verhogen (snelste winst, laag risico)
- **Waar:** `crates/voxel-gpu/src/renderer.rs:539` `max_buf = min(max_buffer_size, 256MB)`.
- **Wat:** verhoog naar 1-2 GB (RTX 4080S heeft 16 GB; 256 MB is belachelijk conservatief).
  Check `device.limits().max_buffer_size` — wgpu default is vaak 256 MB, dus je moet
  `required_limits` in device-aanvraag verhogen (b.v. `max_buffer_size: 2*1024*1024*1024`).
- **Win:** ~4-8x meer zichtbare terrain binnen dezelfde view-distance, géén LOD nodig.
- **Verificatie:** live capture (UNIQUE_COLORS stijgt, geen truncate-warn in stderr),
  `grep -c "VBO budget exceeded"` moet 0 zijn na load.

## P1 — Chunk-gen profilen + hot-path (fBm-kosten)
- **Waar:** `crates/voxel-worldgen/src/lib.rs` `generate_chunk` + `fbm01` (5 octaves:
  2048/512/128/32/4 voxels) x 32^3 voxels per chunk.
- **Wat:** profileren (cargo-flamegraph / manual timer in `mesh_pool` job) om te zien of
  fBm of `classify` dominant is. Opties: (a) fBm-resultaat cachen per (x,z) kolom (deel
  kolommen tussen Y-slabs), (b) simpelere noise voor de fijnste octaaf, (c) genereer
  kolom-hoogte één keer i.p.v. per Y-chunk.
- **Win:** minder CPU per chunk -> snellere eerste load + hogere FPS bij beweging.
- **Risico:** gemiddeld — raak de deterministiciteit niet (seed-invariant).

## P2 — `requested_gen` / `pending` groeiguard
- **Waar:** `gpu_window.rs` — `requested_gen: HashMap`, `pending: HashSet`; enkel gecleared
  bij ingest (`pending.remove` in drain, regel ~397). `requested_gen` groeit per unieke
  chunk-aanvraag en wordt nooit gecleared.
- **Wat:** `requested_gen` entry verwijderen in de drain-loop na verwerking (zoals `pending`),
  OF een `cleanup` pass die verouderde entries (gen < huidige) verwijdert. Voorkomt
  onbegrensde HashMap-groei bij langdurige sessies.
- **Win:** geheugen-stabiliteit bij urenlange sessies; geen langzame degradatie.

## P3 — Mesh-cache RAM-budget (reeds gebudget — lage prioriteit)
- **Status:** `LruMeshCache::new(200_000, 12 GB)` met werkende `evict_if_needed`
  (`estimated_ram() > max_ram_bytes` -> LRU-evict). **Geen actie nodig**, tenzij je de
  12 GB cap omlaag wilt voor lagere RAM-voetafdruk.
- **Notitie:** de CPU-mesh-data is gebudget; het enige onbegrensde deel is `requested_gen`
  (zie P2).

## P4 — Echte filmische schaal (150-200m) = Fase 5 LOD/clipmap
- **Niet nu:** vereist bricked clipmap of chunk-LOD (verre chunks -> lagere voxel-res).
  Pas na P0-P2. Zie `docs/ROADMAP.md` (Fase 5) + alignment-log 2026-07-15.

## Acceptance criteria per item
- P0: load toont volledige view-distance zonder `VBO budget exceeded`-warn; 36/36 groen.
- P1: profiler toont <1 ms/chunk; geen regressie in terrain-uitvoer.
- P2: `requested_gen.len()` stabiliseert na N frames (geen monotone groei in log).
- P3: geen actie (reeds gebudget).
- P4: aparte spike, eigen TDD.

## Harde grenzen (AGENTS.md)
- Geen push/publicatie/deploy zonder menselijke goedkeuring.
- TDD (Rood->Groen) voor elke niet-triviaal item.
- Budget: OpenRouter EUR40/3mnd; optimalisatie is lokaal (geen LLM nodig).
