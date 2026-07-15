# Next-Build Technical Prioritization — Capabilities to ADD

Ranked top-to-bottom; pick from the top. `[PREREQ]` = must-have foundation for later work.
Verified baseline: 83/83 green, live 12.5 cm first-person client (rayon meshing, frustum
cull, LRU cache, 4K triplanar PBR, filmic shader, view-radius 48), headless authoritative
server, VWL1 save/load. Hard constraints: 12.5 cm fixed (ADR-0005), server-authoritative +
deterministic + versioned data (ADR-0003), renderer-agnostic core, strict TDD.

1. **Live voxel edit / place-remove in client** — Interaction — Effort M
   Route edits from the live GPU client into the existing `voxel-edit`/`voxel-persist` path and
   remesh the affected chunk. Makes the world feel editable and exercises the authoritative
   edit-log end-to-end. `[PREREQ]` for multiplayer edit-sync and in-engine authoring tools.

2. **Protocol crate (versioned, from day 1)** — Multiplayer — Effort M
   New `voxel-protocol` crate: framed messages, version handshake, fuzz-tested parsing (ADR-0003).
   Unblocks all networking and is designed first so later crates never retrofit.
   `[PREREQ]` for server-client sync and 2–8 players.

3. **`requested_gen` growth guard + benchmark gate** — Perf — Effort S
   Cap the unbounded `requested_gen`/`pending` maps (OPTIMIZATION_BACKLOG P2); wire the 1 km²
   FPS + replay/soak gate as a CI gate. Cheap stability, protects long sessions.
   `[PREREQ]` for unattended multiplayer soak.

4. **Vertex voxel-AO (0fps method)** — Rendering — Effort S
   Compute 4-neighbour AO at mesh time, bake into vertex data. Research rates this ~80% of the
   "filmic depth" at ~0 runtime cost. `[PREREQ]` for the filmic look; pairs with the existing
   filmic shader.

5. **Day/night cycle** — World-systems — Effort M
   World-clock-driven sun vector + sky/ambient curve. Pure data, no new infra.
   `[PREREQ]` for shadows and weather to be meaningful.

6. **Soft shadows (cascaded shadow map)** — Rendering — Effort M
   One sun CSM (3–4 cascades) within the 4080 budget. Needs the day/night sun vector (#5).
   `[PREREQ]` for filmic outdoor scenes.

7. **Server-client sync** — Multiplayer — Effort M
   Authoritative fixed-tick (20–30 Hz): client input → server → state; snapshot interpolation +
   ack-based edits. Depends on #2. `[PREREQ]` for 2–8p co-op and persistence replication.

8. **Live save in client** — Persistence — Effort S/M
   Serialize seed+edit-log from the running client to VWL1 on demand/interval; resume on launch.
   Depends on #1. `[PREREQ]` for persistent multiplayer worlds.

9. **UI/HUD + debug overlay** — RPG — Effort S/M
   egui overlay: crosshair, chunk/triangle/mem/rev-ID debug, edit-mode indicator.
   `[PREREQ]` for inventory UX and playability feedback.

10. **Audio (spatial sound)** — Audio — Effort S/M
    miniaudio/OpenAL: ambient beds + footstep/place sfx with world positioning. Commodity
    component, no engine risk — big atmosphere win per the north-star. (No hard prereqs.)

11. **Inventory / items / crafting** — RPG — Effort M/L
    Entity inventory, item/material defs, simple crafting graph, server-authoritative. Depends on
    #9. `[PREREQ]` for the RPG gameplay loop.

12. **LOD / clipmap rings** — Rendering/Perf — Effort L
    Chunked clipmap (skirts for cracks), 2× downsample per level; `(x,y,z,lod)` key already
    reserved. Data-driven from the 1 km² benchmark; unlocks true 150–200 m filmic scale.
    `[PREREQ]` for Phase-5 world scale.

**Defer (still valuable, lower urgency):** Water sim (L), Dynamic entities/NPCs + behavior trees
(L, needs #7 + ECS), Weather (M, needs #5), binary greedy meshing drop-in (S, pure speed win),
Live editor palette/HUD tooling (M, needs #1+#9).

**Suggested first picks:** #1 + #4 + #3 (interaction + filmic depth + stability) can ship
immediately and de-risk everything above; #2 then opens the multiplayer track (#7, #8).
