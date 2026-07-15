# Werkplan — 5 engine-verbeteringen (roadmap volgorde)

**Datum:** 2026-07-15
**Basis:** RETAIN_UPDATE_REPLACE_MATRIX.md (onderzoek 18 bronnen)
**Methode per stap:** research → werkplan → review/correct → execute (strict TDD: Rood→Groen)
**Verificatie per stap:** `cargo test -p voxel-gpu --lib` + `cargo build --release -p voxel-client --example gpu_window_main` + `client_smoke` 120/120

---

## Stap 1 — Crack-free skirts (LOD-bug fix)  [HOOGSTE ROI]
**Probleem:** 3-tier LOD (Full/Half/Imposter) heeft geen stitching → kieren op ring-grenzen.
**Oplossing:** hangende rand (skirt) rond elke LOD-chunk in `mesh_chunk_world_meters`.
**Acceptatie:** unit-test `adjacent_full_and_half_chunks_have_no_visible_gap` (Rood→Groen) + visuele capture bij ring-overgang.
**Status:** ← NU UITGEVOERD

## Stap 2 — Inter-chunk occlusie (LxVL)
**Probleem:** alleen frustum-culling; geen chunk-chunk occlusie.
**Oplossing:** 6×6 chunk-face visibiliteitsgraph + walk van camera-column in kijkrichting.
**Acceptatie:** tracer `occlusion_cull_reduces_visible_chunks_by_X_pct` bij view-radius 48.
**Status:** ← ONDERZOEK NA STAP 1

## Stap 3 — BFS zonglift-lighting (fS3V)
**Probleem:** geen cave/overhang-schaduw; alleen vertex-AO.
**Oplossing:** flood-fill vanaf lucht naar binnen, propagatie-dimming; koppel aan WGSL.
**Acceptatie:** `bfs_light_produces_cave_shadow`.

## Stap 4 — Voxel RTAO/RTGI compute-spike (ADR-0006)
**Probleem:** geen ray-traced lighting; jouw wens: Crimson Desert-niveau.
**Oplossing:** DDA voxel ray marching in wgpu-compute over onze chunks; RTAO + zachte schaduwen.
**Acceptatie:** RTAO-kwaliteit vs vertex-AO + frame-time bij r48/RTX4080.

## Stap 5 — Volumetric clouds/weather (vqWz)
**Probleem:** geen weather/cloud-stack.
**Oplossing:** low-res voxel-cloud-volume raymarch + jitter, gekoppeld aan `time_of_day`.
**Acceptatie:** capture met/zonder wolken, NEAR_WHITE oracle.

---
Elke stap krijgt eigen research → werkplan → review → execute cyclus. Geen rewrite;
allemaal additief bovenop de werkende client.
