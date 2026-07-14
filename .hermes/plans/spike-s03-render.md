# Spike S-03 — `voxel-render` (software-raster, mesher output zichtbaar)

**Datum:** 2026-07-15
**Fase:** engine-startgate (S-01 + S-01-hardening groen; S-02 mesher groen)
**Methode:** Strict TDD — failing tests eerst (rood), dan minimale implementatie (groen).
**Afhankelijkheid:** S-01 (`voxel-core`), S-02 (`voxel-mesher`).
**Geen betaalde modellen:** alles lokaal; gratis `:free` alleen voor triage/review.

## Context & motivatie
S-02 levert een pure-data mesh (`Vec<Triangle>`), maar die is niet zichtbaar. S-03 sluit de
keten `Chunk -> mesh -> beeld` met een minimale **software-rasterizer** (puur Rust, géén GPU),
zodat de gebruiker het resultaat kan bekijken. Dit bewijst de pipeline end-to-end vóór de
zware renderer-keuze (wgpu/Vulkan vs Godot vs Bevy), die uitgesteld is naar Fase 2 (ADR-0002).

## Scope (kleinste bewezen render-kern)
- Nieuwe crate `crates/voxel-render`, toegevoegd aan workspace `members`.
- Consumeert een `voxel_core::Chunk`, mesht intern via `voxel_mesher::greedy_mesh`.
- `Camera` (perspectief): yaw/pitch/distance/fov configureerbaar.
- `render_scene(chunk, camera, w, h) -> RgbaImage`: projecteert, rasterizeert met z-buffer,
  shadeert per normaal + materiaalkleur, schrijft PNG via pure-Rust `image`-crate.
- `examples/demo.rs`: rendert een demo-chunk en schrijft een PNG-artifact (voor de gebruiker).

## Acceptance criteria (concreet, meetbaar)
- `cargo test -p voxel-render` 100% groen.
- **Lege chunk** -> blanco afbeelding (alle pixels = achtergrondkleur).
- **Één voxel** -> niet-blank (zichtbare pixels aanwezig).
- **Volle chunk** -> het geprojecteerde centrumpixel is gevuld (geometrie projecteert).
- `Camera` met configureerbare yaw/pitch/distance/fov.
- **Geen GPU/renderer-dependency in core-crates** (ADR-0002): `voxel-render` mag de pure-Rust
  `image`-crate gebruiken; géén godot/bevy/wgpu import in `voxel-core`/`voxel-mesher`.
- Demo-PNG artifact wordt geproduceerd en is door de gebruiker te openen/bekijken.
- `greedy_mesh`-eigenschappen (greedy <= 1,5x culled; waterdicht) ongewijzigd van toepassing (S-02).

## TDD-volgorde
1. Schrijf spike-plan (dit document).
2. Scaffold crate + `lib.rs` stub (modules `camera`, `render` nog niet gevuld).
3. Schrijf FAILING tests (rood): leeg / één voxel / volle chunk.
4. Run `cargo test -p voxel-render` -> ROOD (API ontbreekt / compileert niet).
5. Implementeer `camera` + `render` minimaal -> groen.
6. Run `cargo test --workspace` + genereer demo-PNG -> GROEN + zichtbaar artifact.

## Niet in S-03 (expliciete niet-doelen)
- wgpu/Vulkan/GPU-path, shaders, texturen, lighting-model (Fase 2).
- Client-shell integratie, input, windowing (aparte spike).
- LOD / Transvoxel / smooth SDF (later).
- Netwerk/persistentie van render-output.
