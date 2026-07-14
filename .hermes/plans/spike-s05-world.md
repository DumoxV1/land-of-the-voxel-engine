# Spike S-05 — `voxel-world` (multi-chunk wereldstore)

**Datum:** 2026-07-15
**Fase:** Fase 1 → Fase 3 opmaat (canoniek plan: "asynchrone chunkgeneration/meshing/uploadpipeline", "save/load van seed + edits", multi-chunk wereld).
**Methode:** Strict TDD.
**Autonomie:** binnen bestaande ADR's; client-shell (ADR-0004) onafhankelijk, loopt als subagent (deleg_4c7b3b6d).

## Doel
Een `voxel-world` crate die meerdere chunks bijhoudt in één `World`:
- `get_or_generate(coord, seed)`: chunk uit cache, anders gegenereerd via `voxel-worldgen` en gecached.
- `set_voxel(world_voxel, material)`: schrijft naar de juiste chunk (dirty-marking).
- `chunk_at(coord)`: geleverde chunk (gegenereerd indien nodig).
- Edits (player-plaatsen/verwijderen) overleven generatie: een chunk die al in de store zit (met edits) wordt NIET overschreven door generatie.

## Acceptance criteria
1. `World` is renderer-onafhankelijk (alleen voxel-core + voxel-worldgen).
2. `get_or_generate` is idempotent: tweemaal zelfde coord+seed → zelfde chunk (gecachet, deterministisch).
3. `set_voxel` op een wereldpositie schrijft in de juiste chunk; `chunk_at` levert de gewijzigde chunk; na `get_or_generate` van die coord blijft de edit behouden (geen regeneratie-overwrite).
4. Multi-chunk continuïteit: twee aangrenzende chunks uit dezelfde World hebben dezelfde grensoverschrijdende hoogte-eis als S-04 (geen scheuren).
5. `dirty_chunks()` / `take_dirty()` API voor latere async meshing/upload (Fase 3): een `set_voxel` markeert de chunk dirty; na ophalen is hij niet meer dirty.

## Aanpak (KISS)
- `World { chunks: HashMap<ChunkCoord, Chunk>, seed: u32 }`.
- `get_or_generate`: `entry().or_insert_with(|| generate_chunk(coord, seed))`.
- `set_voxel`: map world→chunk via `ChunkCoord::from_world` + `LocalVoxel::from_world`, `chunk.set`, mark dirty.
- Dirty-set: `HashSet<ChunkCoord>`.

## Tests (failing first)
- `get_or_generate_caches_and_is_deterministic`
- `set_voxel_writes_to_correct_chunk_and_persists`
- `adjacent_chunks_join_without_cracks` (grensoverschrijdend)
- `set_voxel_marks_chunk_dirty`

## Verificatie
`cargo test -p voxel-world` rood → groen; daarna multi-chunk demo (render 2x2 chunks naast elkaar of één chunk met edits).
