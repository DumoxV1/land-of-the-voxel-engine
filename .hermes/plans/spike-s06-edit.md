# Spike S-06 — `voxel-edit` (place/remove tool + edit-events met revisie)

**Datum:** 2026-07-15
**Fase:** Fase 3 opmaat (canoniek plan §3.2: "Ieder edit-event bevat wereldpositie, oude/nieuwe waarde, actor, server-tick en monotone revisie"; §3.5 server-authoritative).
**Methode:** Strict TDD.
**Autonomie:** binnen bestaande ADR's. ADR-0004 (client-shell) loopt als subagent.

## Doel
Een `voxel-edit` crate die voxel-edits modelleert als eerste-klas, replaybare events en een
`EditTool` biedt die safe edits uitvoert op een `World` (place/remove met validatie):

- `Edit` struct: `world: WorldVoxel`, `old: MaterialId`, `new: MaterialId`, `actor: u32`,
  `tick: u64`, `revision: u64` (monotoon oplopend).
- `EditLog`: append-only lijst met `push(edit)`, `len()`, `revision()` (laatste), en
  `apply_all(&mut World)` (replay vanaf een eerdere wereld -> reproduceert eindstaat).
- `EditTool`: `place(world, material, actor, tick) -> Edit` en `remove(world, actor, tick) -> Edit`,
  die de edit uitvoeren op de `World` én loggen. Validatie: geen edit buiten wereldgrenzen
  (chunk-coord mag elke i64 zijn; wereld is oneindig), materiaal 0 = remove.
- Idempotentie: hetzelfde edit-event (zelfde revisie) mag niet dubbel geteld worden.

## Acceptance criteria
1. `Edit` is een zuivere datastructuur (geen wereld-node nodig om te construeren).
2. `EditLog::apply_all` op een verse `World` (zelfde seed) reproduceert exact de wereld zoals
   hij was ná de edits (determinisme + replay).
3. `EditTool::place`/`remove` schrijven naar de `World` én voegen een `Edit` toe aan de log met
   oplopende `revision`.
4. `old` waarde in de edit is de werkelijke voorafgaande wereldwaarde (correctheid van undo/replay).
5. Replay van de log op een nieuwe wereld (zelfde seed) geeft identieke eindstaat (get() gelijk
   voor de bewerkte voxels) — basis voor persistence/multiplayer.

## Aanpak (KISS)
- `Edit { world, old, new, actor, tick, revision }`, `EditLog { edits: Vec<Edit>, next_revision }`.
- `EditTool { log: EditLog }` met `place`/`remove` die `world.set_voxel` + `log.push` doen.
- `apply_all(world)`: voor elke edit `world.set_voxel(edit.world, edit.new)`.

## Tests (failing first)
- `edit_captures_old_and_new`
- `edit_log_revisions_monotonic`
- `edit_tool_place_and_remove_update_world_and_log`
- `replay_reproduces_world_state`

## Verificatie
`cargo test -p voxel-edit` rood → groen; later S-07 (save/load) bouwt op EditLog.
