# Spike S-09 — `voxel-server` (headless dedicated server, server-authoritative)

**Datum:** 2026-07-15
**Fase:** Fase 3 / Fase 4 opmaat (canoniek plan §3.5 server-authoritative, §4 "headless dedicated server (geen GPU)", ADR-0003 GPU-vrije authoritative server).
**Methode:** Strict TDD.
**Autonomie:** binnen ADR's 0002/0003. ADR-0004 (client-shell) = Bevy/wgpu, maar de server is renderer-onafhankelijk (geen client nodig).

## Doel
Een `voxel-server` crate die de wereld + spelers simuleert zónder GPU (headless), als bewijs van
de "authoritative headless server" uit het canonieke plan. De server:
- houdt een `World` + een `EditLog` (van S-07);
- houdt spelers bij als `Player` (S-08) met een `PlayerController`;
- draait een deterministische `tick(dt)` die speler-inputs verwerkt, de controllers stepped,
  en edits (place/remove) toepast + logt;
- is **volledig headless**: geen renderer, geen window, geen GPU — draait in een terminal.

## Acceptance criteria
1. `Server::new(seed)` seedt een wereld (deterministisch) en start met nul spelers.
2. `Server::add_player(id)` voegt een speler toe op een veilige spawn (boven het terrain).
3. `Server::tick(dt)` stept elke speler met diens input via de `PlayerController`; spelers
   vallen en rusten op het terrain (geen crash, geen GPU).
4. `Server::apply_edit(id, world_pos, material)` past een edit toe op de `World` (S-06), logt
   hem in de `EditLog`, en is zichtbaar voor alle spelers (iedereen ziet dezelfde wereld).
5. Determinisme: twee servers met zelfde seed + zelfde input-sequenie produceren identieke
   eindwereld (server-authoritative state is reproduceerbaar).
6. Headless: de crate compileert en draait zónder enige renderer-afhankelijkheid (geen
   `voxel-render` in de dependency-graaf).

## Aanpak (KISS)
- `Server { world: World, log: EditLog, players: HashMap<u32, (Player, PlayerController, Input)> }`.
- `tick(dt)`: voor elke speler `ctrl.step(&mut world, &mut player, input, dt)`.
- `apply_edit`: `world.set_voxel` + `log.push(Edit { world, old, new, actor, tick, revision })`.
- `state_hash()` / `snapshot()`: optioneel, voor determinisme-test (vergelijk een paar voxels).
- Geen netwerk (Fase 4); de spike bewijst de simulatie + edit-propagatie headless.

## Tests (failing first)
- `server_tick_falls_players_to_ground`
- `server_apply_edit_visible_to_all`
- `server_deterministic_same_seed_same_inputs`
- `server_headless_no_renderer_dependency` (compile-time: crate bouwt zonder voxel-render)

## Verificatie
`cargo test -p voxel-server` rood → groen; later Fase 4 voegt netwerk (protocol) toe. Een
`examples/headless_server.rs` draait N ticks headless en print een state-samenvatting (bewijs
dat de server runnable is zónder GPU).
