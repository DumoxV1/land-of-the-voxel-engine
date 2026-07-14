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
**Actieve fase:** Fase 0 → engine-startgate GEOPEND (S-01 voxel-core + S-01-hardening + S-02 mesher + S-03 software-raster onder strict TDD)  \
**Laatste update:** 2026-07-15

## Volgende gates
1. ✅ Onafhankelijke gratis reviewer corrigeerde bewijs, licenties, verzonnen/ongeverifieerde metrics en scope (review-initial-bundle.md, B-01…B-08).
2. ✅ Gratis architect synthetiseerde uitsluitend geverifieerde resultaten naar ADR-spikes (adr/0001–0003) en planupdates.
3. ✅ Exact implementatieplan voor de eerste `voxel-core` tracer bullet (S-01) geschreven (zie `.hermes/plans/spike-s01-voxel-core.md`).
4. ✅ S-01 onder strict TDD: failing tests eerst (rood), dan implementatie (groen). Repo scaffold + `voxel-core` crate.
5. ✅ S-01-hardening: drie chunk-states (`Uniform`/`PalettePacked`/`Dense`) + 4-bit bitpacking + per-chunk palette (≤16), byte-stabiel versie-2 formaat. 7 nieuwe failing tests → groen.
6. ✅ S-01-hardening: drie chunk-states (`Uniform`/`PalettePacked`/`Dense`) + 4-bit bitpacking + per-chunk palette (≤16), byte-stabiel versie-2 formaat. 7 nieuwe failing tests → groen.
7. ✅ S-03 software-raster spike: `voxel-render` crate, `Camera` (perspectief) + `render_scene` (z-buffer, per-normaal shading) → PNG. 3 failing tests → groen; demo-PNG gegenereerd en visueel geverifieerd (voxel-scène herkenbaar). Geen GPU/renderer-dep in core-crates (ADR-0002).
8. ⏳ Volgende: S-03 uitbreiden (meer demo-scenario's / schaal) of S-04 (client-shell/input — wgpu naargelang Fase-2 benchmark) — keuze autonoom volgende sessie.

## Auditwaarschuwing
Researchmemo’s zijn input, geen waarheid. Een steekproef vond foutieve actualiteitsclaims en niet-onderbouwde benchmarkgetallen. Geen cijfer of stackadvies wordt overgenomen zonder onafhankelijke broncontrole of lokaal experiment.

## Actieve automatisering
- Dagelijkse no-agent plan-alignmentguard.
- Wekelijkse no-agent OpenRouter-budget- en free-modelguard.
- Wekelijkse read-only governance-review op `openrouter/free`.
- Vier gespecialiseerde profielen, alle gepind op `openrouter/free`.

## Menselijke input
Alleen noodzakelijke vragen worden als geblokkeerde Kanban-kaarten gesteld. Geen menselijke code-review vereist; wel menselijke toestemming voor uitgaven, publicatie, accounts, grote scopewijzigingen en destructieve acties.
