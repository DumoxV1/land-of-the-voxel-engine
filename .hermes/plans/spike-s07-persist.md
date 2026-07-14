# Spike S-07 — `voxel-persist` (save/load seed + edits)

**Datum:** 2026-07-15
**Fase:** Fase 3 → Fase 5 opmaart (canoniek plan §3.6 persistence: "append-only editlog per regio/chunk", "save/load van seed + edits"; §4 Fase 3 gate: "save/reload behoudt alle edits").
**Methode:** Strict TDD.
**Autonomie:** binnen bestaande ADR's. ADR-0004 (client-shell) loopt als subagent.

## Doel
Een `voxel-persist` crate die een wereld + z'n edits opslaat naar een bestand en teruglaadt,
zodat na herstart alle edits behouden blijven (canoniek plan success-criterium: "Wereldwijzigingen
persistent na serverrestart" + "save/reload behoudt alle edits").

- `save_world(world, log, path)`: schrijf seed + edit-log naar een binair bestand.
- `load_world(path) -> (World, EditLog)`: lees seed + edits, reconstrueer de wereld door de
  edits te replayen op een verse `World` met die seed.
- Round-trip is deterministisch: na save→load is de wereld identiek aan de oorspronkelijke
  (voor de bewerkte voxels én de gegenereerde basis).

## Acceptatiecriteria
1. `save_world` + `load_world` round-trippen: de geladen wereld is identiek aan de oorspronkelijke
   (gedit眼界 voxels én gegenereerde basis) voor de bekeken posities.
2. Na load zijn de edits replaybaar (de geladen `EditLog` bevat alle oorspronkelijke edits, met
   oplopende revisies).
3. Corrupte/onvolledige input faalt controleerbaar (geen panic) — minstens: verkeerde magie-byte
   of te korte data geeft een `Err`, geen crash.
4. Renderer-agnostisch: alleen voxel-core + voxel-world + voxel-edit.

## Aanpak (KISS, geen externe serialisatie-dep)
Eigen compact binair formaat:
- magic: [b'V', b'W', b'L', b'1'] (4 bytes)
- seed: u32 LE
- edit_count: u32 LE
- per edit: world(x,y,z i64 LE), old u8, new u8, actor u32 LE, tick u64 LE, revision u64 LE
Geen chunk-data opslaan (basis is reproduceerbaar uit seed); alleen de edits. Dat is precies de
"procedurele basis + append-only editlog" aanpak uit het canonieke plan.

## Tests (failing first)
- `save_then_load_reproduces_world`
- `loaded_log_contains_all_edits`
- `corrupt_input_returns_error`

## Verificatie
`cargo test -p voxel-persist` rood → groen; later S-09 (server) gebruikt dit voor recovery.
