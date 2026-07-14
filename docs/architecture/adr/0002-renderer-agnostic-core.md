# ADR-0002: Renderer-Agnostic Voxel Core

- **Status:** Accepted (via free-model architect synthesis, afgedwongen door review B-07)
- **Datum:** 2026-07-14
- **Deciders:** voxelarchitect (synthese), voxelreviewer (review)

## Context
Het plan (§2.2) eist dat de systemen die de innovatie bepalen — voxelopslag, terrain, meshing, LOD,
streaming, networking, persistence — **eigen** blijven, maar dat commodity-lagen (windowing, input,
audio, UI) hergebruikt mogen worden. Twee client-spikes (Godot+GDExtension, Bevy/wgpu) worden als
gelijke kandidaten beoordeeld vóór een keuze (Plan §4 Phase 2, review B-06). Als de core aan een
renderer hangt, is het spikewerk weggegooid bij een eventuele wissel.

## Beslissing
De `voxel-core` crate (en downstream `voxel-mesher`, `worldgen`, `world-store`, `protocol`,
`game-sim`) is **renderer-agnostisch**:
- Compileert en passeert de volledige test-suite (unit + property) **zonder** Godot-, Bevy- of
  C++-renderer-dependency.
- Geen `godot`, `bevy`, `wgpu` of platform-specifieke imports in de core-crates.
- Renderer-laag consumeert core-data via een smalle interface (chunks, edits, snapshots); clients
  bouwen meshes lokaal, de server verstuurt geen render-meshes (Plan §3.3).
- Determinisme-zorg: integer wereldcoördinaten, versieerbare serialisatie, monotone revisie-ID's.

## Alternatieven
- **Godot-first** (verworpen, schendt Plan §2.2/§4): engine-stack-memo beval Godot als "primary"
  omwille van snelheid; review B-06 verplichtte equal-priority spikes. Keuze volgt pas na Phase 2
  benchmarks, vastgelegd in een aparte ADR.
- **Bevy-first / C++-first**: idem — pas na gemeten vergelijking.

## Bewijs / benchmarks
- Review B-07: core-portabiliteitscriterium is een harde acceptatie-eis voor de stack-keuze.
- Benchmark B-06 (Determinism replay): zelfde seed + inputs → bit-identieke wereldstatus over
  stacks.
- Benchmark B-07 (Headless multiplayer soak): headless authoritative server + 2–8 clients, 30 min.
- `voxel-core` crate MUST compile/test zonder renderer-dep (CI-gate, S-10).

## Gevolgen
- Repo-layout: `crates/voxel-core` etc. onder een Rust-workspace; clients onder `clients/` consumeren
  de core via FFI/GDExtension/Bevy-plugin.
- CI (S-10) blokkeert elke PR die een renderer-import in een core-crate introduceert.
- Meer infrastructuur-frontwerk, maar geen weggegooid spikewerk bij stack-wissel.

## Herzieningstrigger
- Een benchmark toont dat een specifieke renderer-diepe integratie onmisbaar is voor de prestaties
  en de meerwaarde opweegt tegen het verlies van agnosticiteit (vereist nieuwe ADR + meting).
