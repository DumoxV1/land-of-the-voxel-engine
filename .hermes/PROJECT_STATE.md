# Project State

**Canoniek plan:** `.hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md`  
**Status:** researchreview en plansynthese actief  
**Actieve fase:** Fase 0 — productdefinitie, meetlat en governance  
**Laatste update:** 2026-07-14

## North star
Een filmische, zeer rijke en dynamische openwereld-RPG op een eigen micro-voxelfundament — ambitieus als “de GTA VI / Crimson Desert onder micro-voxel-engines”, maar ontwikkeld via meetbare technische gates.

## Huidige beslissingen
- Geen volledige MMO in de eerste twaalf weken; eerst een vertical slice.
- Procedurele basiswereld + sparse persistente wijzigingen.
- Eigen voxel/world/network/persistence-kern; commodity-platformfuncties mogen uit open source komen.
- Blocky versus smooth en clientshell worden beslist via gelijke benchmarks, niet voorkeur.
- Gratis OpenRouter-modellen zijn standaard voor research en eerste reviews.

## Werkprotocol
Na elke derde voltooide uitvoeringsstap wordt de vorige stap opnieuw gecontroleerd en wordt plan-alignment expliciet vastgelegd in `docs/governance/alignment-log.md`.

**Status:** researchreview en plansynthese VOLTOOID  \
**Actieve fase:** Fase 0 → engine-startgate GEOPEND (S-01..S-07 onder strict TDD: voxel-core, hardening, mesher, software-raster, worldgen, world-store, edit-tool, persist)  \
**Laatste update:** 2026-07-15

## Volgende gates
1. ✅ Onafhankelijke gratis reviewer corrigeerde bewijs, licenties, verzonnen/ongeverifieerde metrics en scope (review-initial-bundle.md, B-01…B-08).
2. ✅ Gratis architect synthetiseerde uitsluitend geverifieerde resultaten naar ADR-spikes (adr/0001–0003) en planupdates.
3. ✅ Exact implementatieplan voor de eerste `voxel-core` tracer bullet (S-01) geschreven (zie `.hermes/plans/spike-s01-voxel-core.md`).
4. ✅ S-01 onder strict TDD: failing tests eerst (rood), dan implementatie (groen). Repo scaffold + `voxel-core` crate.
5. ✅ S-01-hardening: drie chunk-states (`Uniform`/`PalettePacked`/`Dense`) + 4-bit bitpacking + per-chunk palette (≤16), byte-stabiel versie-2 formaat. 7 nieuwe failing tests → groen.
6. ✅ S-01-hardening: drie chunk-states (`Uniform`/`PalettePacked`/`Dense`) + 4-bit bitpacking + per-chunk palette (≤16), byte-stabiel versie-2 formaat. 7 nieuwe failing tests → groen.
7. ✅ S-03 software-raster spike: `voxel-render` crate, `Camera` (perspectief) + `render_scene` (z-buffer, per-normaal shading) → PNG. 3 failing tests → groen; demo-PNG gegenereerd en visueel geverifieerd (voxel-scène herkenbaar). Geen GPU/renderer-dep in core-crates (ADR-0002).
8. ✅ S-04 deterministische worldgen spike: `voxel-worldgen` crate, `generate_chunk(coord, seed)` (seeded value-noise heightmap, grass/dirt/stone lagen). 5 failing tests → groen (determinisme, seed-verschil, chunk-grenscontinuïteit, niet-leeg, laagstructuur). demo_worldgen.png visueel geverifieerd als rollende, scheurvrije terrain. Renderer-agnostisch (alleen voxel-core).
9. ✅ S-05 multi-chunk world-store spike: `voxel-world` crate, `World` (HashMap cache + seed-generatie + edits + dirty-set). 4 failing tests → groen. `render_world` + `demo_world.png` visueel geverifieerd.
10. ✅ S-06 edit/place-remove tool + edit-events: `voxel-edit` crate, `Edit` (world/old/new/actor/tick/revision) + `EditLog` (append-only, monotoon) + `EditTool::place/remove`. 4 failing tests → groen (old-capture, monotone revisies, tool update, replay-reproductie).
11. ✅ S-07 persistence (save/load seed+edits): `voxel-persist` crate, eigen binair formaat (magic+seed+edits), `save_world`/`load_world`/`PersistError`. 3 failing tests → groen (round-trip reproductie, log-behoud, corrupt→Err). `demo_persist.png` bewijst save→load→toren-herstel. Renderer-agnostisch.
12. ✅ S-08 spelercontroller + voxel-collision: `voxel-player` crate, `Player` (pos/AABB/yaw) + `PlayerController` (axis-separated collision, sub-stepping tegen tunnelen, `resolve_floor_y` voor vloer-rust). 4 failing tests → groen (vooruit-beweging, muur-blokkade, gravity→rust op vloer/terrain, langs-muur-glijden). `demo_player.png` bewijst first-person grond-niveau view. Renderer-agnostisch.
13. ✅ ADR-0004 (client-shell): subagent-dossier (deleg_4c7b3b6d) → **Rust + Bevy/wgpu** gekozen (pure-Rust core native, geen FFI, gedeelde server-workspace; Godot GDExtension afgewezen voor eerste slice). Status Proposed, Fase-2 benchmark-gate (B-06/B-07 + FPS) blijft verplicht voor lock-in. Gedelegeerd door gebruiker (volmacht 2026-07-15).
14. ✅ S-09 headless dedicated server: `voxel-server` crate, `Server` (World + EditLog + spelers), `tick` (stept PlayerController headless, géén GPU), `apply_edit` (server-authoritative, logt in EditLog), determinisme (zelfde seed+edits → identieke wereld). 4 failing tests → groen. `examples/headless_server.rs` draait 600 ticks headless, 3 spelers spawnen/vallen/lopen, beacon-edit zichtbaar in gedeelde wereld. **RUNNABLE ARTIFACT**: `cargo run --example headless_server -p voxel-server`.
15. 🎯 **Vertical slice bereikt (S-01..S-09, strict TDD):** voxel-core→mesher→render→worldgen→world→edit→persist→player→server. Headless server runt zónder GPU; wereld is persistent (S-07) en server-authoritative (S-09). Client-shell = Rust+Bevy/wgpu (ADR-0004, Fase-2 benchmark-gate nog te lopen). Volgende (Fase 4): netwerk/protocol (multiplayer 2–8p), daarna echte Bevy/wgpu-client.
16. ✅ S-10 GPU-renderer (wgpu/Vulkan, RTX 4080): `voxel-gpu` crate. `probe` bewijst wgpu init + offscreen readback op de host-GPU (probe.png: gradient-driehoek). `renderer` neemt `greedy_mesh`-triangles → vertex-buffer op de GPU, WGSL-shader met per-normaal directionele belichting + warme fog/atmosfeer + warme materiaal-tinten (Lay of the Land-vibe). `examples/gpu_world.rs` genereert 2×2 terrain (16.270 tris) en rendert naar gpu_world.png — visueel geverifieerd als geshade voxel-heuvels met diepte. **DIT IS DE EERSTE GPU-RENDER VAN DE ENGINE** (voldoet aan de gebruikerseis: engine draait op de GPU, niet de software-raster). wgpu gepind op 0.17.2 (0.18.0 yanked; 0.19/0.20 hebben API-drift). Camera-matrix via `glam` (geen handgeschreven matrix-fouten). Nog geen winit-venster (offscreen PNG), geen echte Bevy-integratie — dat is Fase 4.

## Auditwaarschuwing
Researchmemo’s zijn input, geen waarheid. Een steekproef vond foutieve actualiteitsclaims en niet-onderbouwde benchmarkgetallen. Geen cijfer of stackadvies wordt overgenomen zonder onafhankelijke broncontrole of lokaal experiment.

## Actieve automatisering
- Dagelijkse no-agent plan-alignmentguard.
- Wekelijkse no-agent OpenRouter-budget- en free-modelguard.
- Wekelijkse read-only governance-review op `openrouter/free`.
- Vier gespecialiseerde profielen, alle gepind op `openrouter/free`.

## Menselijke input
Alleen noodzakelijke vragen worden als geblokkeerde Kanban-kaarten gesteld. Geen menselijke code-review vereist; wel menselijke toestemming voor uitgaven, publicatie, accounts, grote scopewijzigingen en destructieve acties.
