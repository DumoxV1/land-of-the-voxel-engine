# Spike S-04 — `voxel-worldgen` (deterministische seeded generatie)

**Datum:** 2026-07-15
**Fase:** Fase 1 — Data- en meshing-spikes (canoniek plan §4, week 2–3 deliverable: "deterministische worldgen met seed").
**Methode:** Strict TDD — failing tests eerst, dan implementatie.
**Autonomie:** binnen bestaande ADR's (geen nieuwe architectuurbeslissing nodig); client-shell-keuze (Godot vs Bevy/wgpu) is bewust NIET onderdeel van deze spike (Fase 2 gate).

## Doel
Een `voxel-worldgen` crate die `voxel_core::Chunk`s genereert uit een seed + `ChunkCoord`, deterministisch (zelfde seed+coord → identieke chunk) en met grensoverschrijdende continuïteit (aangrenzende chunks vormen één doorlopend terrein zonder scheuren — canoniek plan §6: "random edits veroorzaken geen scheuren aan chunkgrenzen").

## Acceptance criteria
1. `generate_chunk(coord, seed) -> Chunk` bestaat en is renderer-onafhankelijk (geen GPU/renderer dep).
2. Determinisme: `generate_chunk(c, s) == generate_chunk(c, s)` (alle velden gelijk) over meerdere calls.
3. Seed-gevoeligheid: verschillende seeds geven (hoogstwaarschijnlijk) verschillende chunks voor zelfde coord.
4. Grensoverschrijdende continuïteit: de heightmap is een pure functie van wereld-X/Z (niet van chunk), dus aangrenzende chunks sluiten naadloos aan (geen height-sprong aan chunkgrens). Geverifieerd via een test die de randkolom van chunk A vergelijkt met de aangrenzende rand van chunk B.
5. Materiaalverdeling: bovenkant = gras (2), daaronder dirt (1), kern = stone (3); lucht (0) erboven. Tennminste één voxel per chunk is niet-lucht (geen lege chunk bij seed 0).
6. S-03 render kan de gegenereerde chunk zichtbaar maken (demo breidt uit naar worldgen).

## Aanpak (minimal, KISS)
- Seeded PRNG: simpele, snelle, deterministische hash (bv. een `u32` xorshift of `splitmix`-achtige mix op seed+world-coord). Geen externe rand-crate nodig; we roepen determinisme en continuïteit af, geen crypto.
- Heightmap: 2D value-noise (interpoleren van een gehashte integer-grid) op wereld-X/Z → hoogte h in [0, CHUNK_SIZE). 
- Vulling per kolom (world-Y): y > h → lucht(0); y == h → gras(2); h-3 < y <= h → dirt(1); y <= h-3 → stone(3).
- `generate_chunk` zet alleen de chunk-local voxels; wereld-Y = chunk_origin_y + local_y. Omdat heightmap puur van wereld-X/Z afhangt, is continuïteit gratis.

## Tests (failing first)
- `deterministic_same_seed_same_chunk`: 2 calls gelijk.
- `different_seed_different_chunk`: verschillende seeds ≠ (hoogstw. waar, niet strikt — maar seed 0 vs 1 verschillen bij onze formule).
- `chunk_boundary_continuous`: chunk (0,0,0) rand-x = 31 vult aan op chunk (1,0,0) rand-x = 0 (zelfde wereld-X grens → zelfde height).
- `non_empty_chunk`: chunk heeft >0 niet-lucht voxels.
- `material_layers`: bovenste niet-lucht voxel = gras(2); eronder dirt/stone; lucht erboven.

## Verificatie
`cargo test -p voxel-worldgen` rood → groen; daarna `cargo test --workspace`. Geen GPU.
