# Sessie-handoff — 2026-07-15 (avond)

> **Doel van dit bestand:** de volgende sessie in < 2 min op snelheid brengen. Dit is een
> *state-of-play* snapshot, geen roadmap. Autoritatieve bronnen blijven:
> - `docs/ROADMAP.md` — fasen + wat bewijsbaar staat
> - `docs/research/voxel-engine-survey-2026/WORKPLAN_5_STEPS.md` — de 5 must-do verbeteringen
> - `docs/architecture/adr/` — beslissingen (met name ADR-0004 client-shell, ADR-0006 ray-marching)
> - `docs/research/2026-07-15-sota-advies-vervolg.md` — SOTA-vervolgadvies

## Runtime-staat (bevestigd werkend deze sessie)
De interactieve GPU-client draait op de RTX 4080 Super. Startcommando:
```
cd "C:\Users\keere\Desktop\Land of the Voxel Engine" && cargo run --release --example gpu_window_main -p voxel-client
```
Besturing: **WASD** lopen · **muis** kijken · **Space** springen · **F** walk/fly · **F2** dag/nacht.

**Debug-HUD** (rechtsboven, `crates/voxel-client/src/hud.rs`): FPS · X/Y/Z · YAW · CHUNKS · TRIS ·
SEED · MODE · TIME. Bitmap-font (5x7, 2x upscale), geen externe text-deps. Door gebruiker
visueel goedgekeurd.

## Wat deze sessie is gefixt (allemaal op master, gepusht)
| Commit | Fix |
|--------|-----|
| `3ae54bd` | **Half-LOD wereld-origin** stond 4x fout → verre chunks zweefden als raster hoog in de lucht. `to_world` vermenigvuldigt met `voxel_scale`, dus origin-factor moet `VOXEL_SIZE_M / voxel_scale` zijn (was omgekeerd). TDD: `lod_half_shares_full_world_origin_at_double_voxel_size` (RED 80!=20 → GREEN). **Dit was de echte bron van de zwevende blokken.** |
| `d249c2c` | Imposter-LOD-ring uitgezet in client (`lod_imposter_radius == view_radius`). Losse platte imposter-quads zagen er van hoogte uit als zwevende vierkanten. Imposter-code blijft intact voor een latere crack-free far-LOD-pass. |
| `1d60a32` | `mesh_chunk_imposter` geeft lege mesh voor all-air chunks (geen quad op chunk-basis). |
| `6901766` | Debug-HUD overlay (zie boven). |
| `7cab7c3` | Bedrock-floor (`is_solid` y<0→true) + `BEDROCK_DEPTH=1` → niks meer zichtbaar onder de map; `is_solid_ao` y<0→false zodat bodem-zijvlakken geen valse AO krijgen. |

**Teststatus:** `voxel-gpu --lib` = 24/24 · `voxel-mesher` = 8/8 · `client_smoke` = 120/120 frames, no panic.

## Streaming/LOD-parameters (client, `voxel-client/src/lib.rs` ~r.231)
- `VIEW_RADIUS = 48` chunks (~192 m disc)
- `view_radius: 48`, `max_y: 12`, `requests_per_frame: 4`
- `lod_half_radius: 8` (chebyshev ≥8 → Half), `lod_imposter_radius: 48` (= view_radius → imposter effectief uit)
- `air_margin: 1`
- `CHUNK_SIZE = 32`, `VOXEL_SIZE_M = 0.125`, wereld verticaal ~40 m (human-scale juice, niet de voxel-grootte).

## Openstaande must-do stappen (WORKPLAN_5_STEPS.md)
1. ✅ Stap 1 — crack-free skirts (gedaan, daarna uitgezet in render-pad wegens artefacten)
2. ✅ Stap 2 — inter-chunk occlusie
3. ⬜ **Stap 3 — BFS zonlicht-lighting** (grot/schaduw). Verse research nodig.
4. ⬜ **Stap 4 — visuele juice**: skybox-gradient + distance-fog + materiaalkleuren. Creative UI → look eerst laten goedkeuren, tests achterhouden tot commit.
5. ⬜ **Stap 5 — deterministic seed** + spawn-consistentie-test.

## Grote richting (nog te plannen)
- **ADR-0006 ray-marching laag**: voxel DDA/ray-march in wgpu-compute als *additieve* filmische
  laag (GEEN octree-rewrite, GEEN HW-DXR). Voor de Crimson Desert / Lay of the Land look.
- **ADR-0004 client-shell**: status Proposed (Bevy/wgpu vs kaal wgpu). De client draait nu op
  kaal wgpu 30 + winit 0.30 — ADR bijwerken naar de feitelijke keuze.

## Bekende quirks / valkuilen
- **wgpu 30 API** wijkt sterk af van tutorials: descriptor-velden zijn deels `Option`/slices-of-Option
  (`bind_group_layouts: &[Some(&bgl)]`, `entry_point: Some(..)`, `multiview_mask: None`,
  `depth_slice: None`, `TexelCopyTextureInfo`/`TexelCopyBufferLayout`, `immediate_size: 0 (u32)`).
  Skill: `wgpu-bitmap-hud-overlay` legt de werkende vormen vast.
- **LNK1104** bij `cargo build`: de client-.exe is gelockt zolang het venster open staat → eerst sluiten.
- **`search_files` faalt op paden met spaties / de `index.crates.io-*` cargo-registry-dir** (punt in
  dirnaam) → gebruik `read_file` op het absolute pad.
- Bash-shell in `terminal` spuit wat `clawdock`/`compdef` warnings uit bij start — onschuldig.
- Skirts staan overal `with_skirts=false` (functie bewaard voor latere correcte stitching).

## Aanbevolen eerste zet volgende sessie
Codebase + roadmap herzien zoals gevraagd: (a) ADR-0004 bijwerken naar de feitelijke wgpu-30-shell,
(b) kiezen tussen **Stap 4 (visuele juice, grootste zichtbare sprong)** en **Stap 3 (lighting)**,
(c) beslissen wanneer de ray-marching laag (ADR-0006) ingepland wordt.
