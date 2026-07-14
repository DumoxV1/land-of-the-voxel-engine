# Spike S-11 — Audit-hardening (fixes uit onafhankelijke code-audit, 2026-07-15)

## Aanleiding
Onafhankelijke audit (deleg_b5c57770, gratis model, door Hermes geverifieerd in de bron) vond 6 hoge en 13 middel-bevindingen. Dit spike-plan fixt de hoge + goedkope middel-bevindingen onder strict TDD.

## Scope (in deze volgorde)
1. **M-01 mesher-geometrie**: +vlakken op `d+1` i.p.v. `d`; winding consistent met normaal (u×v-teken). Golden vertex-positietest + windingtest (RED eerst). Daarna `cull_mode: Some(Back)` in voxel-gpu.
2. **C-01 serialize-robustheid**: nibble-validatie (< palette_len) in `from_bytes`; chunk-coords als i64 op de wire (formaat versie 3, versie ≠ 3 → nette Err). Malformed-input tests RED eerst.
3. **P-01 player-physics**: terminal velocity clamp (≤ 40 u/s zodat |dy| < 1 voxel per substep van 0.02 s); `resolve_floor_y` over de volledige AABB-footprint (max 4 kolommen). Tunneling-test (val van y=500 op 1-dikke vloer) + blokrand-test RED eerst.
4. **S-01s server-determinisme**: tick-volgorde sorteren (`ids.sort_unstable()`); determinisme-test met 3 spelers.
5. **G-01 materials**: canonieke material-constanten in `voxel-core::palette::materials`; worldgen/gpu/render gebruiken die; grass/dirt-tintswap gefixt; fog vanaf camera-eye i.p.v. origin. CPU-unit-tests voor `material_tint`.
6. **PS-01 persist**: atomair schrijven (tmp + rename).

## Acceptance criteria
- Alle nieuwe tests eerst RED aantoonbaar, daarna GREEN.
- `cargo test --workspace` volledig groen (≥ 48 + nieuwe tests).
- `gpu_world.png` opnieuw gerenderd mét backface-culling en correcte kleuren; visueel geverifieerd.
- Geen API-breuk buiten de workspace (persist VWL1-formaat ongewijzigd; chunk-serialize versie-bump is intern, gedocumenteerd).

## Buiten scope (naar roadmap)
Neighbor-aware meshing, `World::get` zonder clone, fixed-timestep server-API, checksum in persist, binary greedy meshing, fuzz-targets (Fase 2/3-werk).
