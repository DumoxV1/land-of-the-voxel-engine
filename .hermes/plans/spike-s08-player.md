# Spike S-08 — `voxel-player` (spelercontroller + camera in wereld)

**Datum:** 2026-07-15
**Fase:** Fase 3 (canoniek plan §4: "eenvoudige spelercontroller", §3.4 physics: capsule-vs-voxel).
**Methode:** Strict TDD.
**Autonomie:** binnen bestaande ADR's. ADR-0004 (client-shell) loopt als subagent.

## Doel
Een `voxel-player` crate met een first-person-achtige speler in de voxel-wereld:
- `Player`: wereldpositie (Vec3/f32), een AABB-hitbox (half-extents), en een yaw (kijkrichting).
- `PlayerController`: beweegt de speler op input (vooruit/achteruit/links/rechts + jump/gravity)
  met **collision tegen de voxel-wereld** (vakjes die niet-lucht zijn blokkeren).
- Deterministisch: een vaste `step(&mut World, input, dt)` met integer-safe collision (probeer
  X, dan Y, dan Z-as apart — zodat de speler langs muren glijdt in plaats van kleeft).

## Acceptance criteria
1. `Player` heeft positie + hitbox + yaw; geen wereld-node nodig om te maken.
2. `step` met "vooruit" input verplaatst de speler in de kijkrichting (yaw), mits geen solide
   voxel in de weg zit.
3. Collision: als een solide voxel in de bewegingsrichting staat, stopt de speler (geen
   doorgang); beweging loodrecht op de muur glijdt wel door.
4. Gravity: zonder vloer valt de speler; op een solide voxel (grond) blijft hij staan (geen
   tunneling door de vloer).
5. Renderer-agnostisch: alleen voxel-core + voxel-world (+ voxel-render alleen in voorbeeld).

## Aanpak (KISS)
- `Player { pos: [f32;3], half: [f32;3], yaw: f32, on_ground: bool }`.
- `Input { forward, back, left, right, jump: bool }`.
- `PlayerController::step(world, player, input, dt)`:
  - bereken gewenste delta uit yaw + input;
  - pas X toe, test collision (elke voxel die de AABB overlapt is solide → revert X);
  - idem Y (incl. gravity, jump), Z;
  - zet `on_ground` als er een solide voxel net onder de AABB is.
- Collision-test: voor de AABB's min/max in wereldruimte, controleer alle voxelkolommen in
  bereik op `is_solid` (materiaal != 0).

## Tests (failing first)
- `step_forward_moves_player`
- `collision_blocks_movement`
- `gravity_makes_player_fall_and_rest_on_ground`
- `per_axis_slide_along_wall`

## Verificatie
`cargo test -p voxel-player` rood → groen; later S-09 (server) simuleert dezelfde step.
