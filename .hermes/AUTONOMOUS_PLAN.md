# AUTONOME BOUW-SESSIE — Land of the Voxel Engine

**Start:** 2026-07-15 (vervolg op de 150km²-nacht)
**Mandaat:** gebruiker wil GEEN multiplayer nu; wel "alles wat je kan om de engine beter te maken".
**Methode:** strict TDD (Rood→Groen), verificatie via build/test/live-capture na elke stap.
**Lock-in:** shader-spike + 3-tier biomes + perf al gepusht (`99c390a`, `7b23b89`). 83/83 groen.

## Wat VALT BUITEN scope (expliciet)
- Geen netwerk/protocol-laag, geen server-client sync, geen 2-8p co-op (ADR-0003 volgende fase).

## Fase-indeling (top-down, dependency-gesorteerd)

### Fase 1 — Stabiliteit + interactie (de-riskt alles daarboven)
- [ ] P1 `requested_gen`/`pending` groeiguard (OPTIMIZATION_BACKLOG P2) — S
- [ ] F5 Vertex-AO (0fps-methode, bake in mesh) — S, filmische diepte
- [ ] I1 Live voxel edit in client (muis-klik → place/remove via `voxel-edit`, remesh) — M
- [ ] I2 Live save in client (F5 → VWL1 save/load) — S/M

### Fase 2 — Filmische shader-passes (puur GPU, geen core-wijziging)
- [ ] F1 Post-processing stack (ACES tone-map + bloom + vignette + grain) — S
- [ ] F2 Dag/nacht-cyclus + procedurele lucht — M
- [ ] F3 Cascaded shadow map (CSM, 3-4 cascades) — M (na F2)
- [ ] F4 Water-oppervlak (reflecterend, Gerstner) — M

### Fase 3 — Wereld-rijkdom
- [ ] F8 Vegetatie & props (instanced grass/bomen/stenen + wind) — M
- [ ] F6 Volumetrische wolken + atmospheric scattering — M/L
- [ ] F7 Weer (regen/sneeuw + nevel + natigheid) — S/M (na F2)
- [ ] A1 Positie-audio (miniaudio: ambient + voetstappen/place) — M

### Fase 4 — RPG-structuur
- [ ] R2 UI/HUD (egui: crosshair, debug-overlay, edit-mode) — M
- [ ] R1 ECS / game-loop (foundation voor entities/quests) — M/L
- [ ] R3 Inventory / items / crafting — M/L (na R1+R2)

### Fase 5 — Schaal
- [ ] W1 Dynamische entities/NPC's (na R1, ECS) — L
- [ ] W2 LOD/clipmap rings (Fase 5, na 1km²-benchmark) — L

## Verificatie per stap
- Build: `cargo build --release -p voxel-gpu --example gpu_window`
- Tests: `cargo test --workspace`
- Live: capture UNIQUE/NEAR_WHITE/CLEAR/RENDER_ERRORS bij view-radius 48
- Geen regressie: NEAR_WHITE < 1%, CLEAR = 0%, geen panic.

## Infrastructuur
- `.hermes/heartbeat.txt` — laatste activiteit
- cron `voxel-autonomous-heartbeat` — herstart bij >45m stilstand
