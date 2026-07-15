# Definitieve codebase-vergelijking & beslismatrix

**Datum:** 2026-07-15
**Basis:** `direct/adaptive-voxel-grids.md`, `direct/set-a.md`, `direct/set-b.md`, `direct/CODEBASE_COMPARISON_DRAFT.md`, `direct/VISUAL_SOURCE_NOTES.md`, lokale transcripts, en onze actuele `crates/*`.

**Kernconclusie:** onze Rust/wgpu-architectuur is **niet inferieur** aan wat in de 18 bronnen zichtbaar is. Op streaming/LOD/meshing/worker-pool zijn we verder dan de meeste demo's. De waarde zit in *specifieke technieken*, niet in taal- of opslag-rewrites.

---

## 1. Beslismatrix

| Techniek (bron) | Onze status | Verdict | Oordeel |
|---|---|---|---|
| Greedy meshing (LxVL, CJ94, fS3V, 0fps) | `voxel-mesher` | **BEHOUD** | Staat al; 0fps-methode |
| 3-tier LOD (Full/Half/Imposter) | `chunk_stream.rs` | **BEHOUD + REPARER** | Werkt, maar **geen crack-free stitching** → kieren op ring-grenzen (bug) |
| Crack-free skirts/stitching | mist | **AANPASSEN (eerste stap)** | Kleine patch, hoogste ROI, geen licentie-risico |
| Transvoxel | mist | **EIGEN impl, NIET proprietary** | Terathon-licentie! Schrijf zelf of koop licentie |
| Inter-chunk occlusie (LxVL) | frustum-only | **AANPASSEN (hoogste ROI)** | 6×6 chunk-face visibiliteitsgraph; top bij 12,5cm/view-radius 48 |
| Per-frame upload-budget/reject (LxVL) | `UPLOAD_BUDGET`+gen-counter | **BEHOUD** | Al afgedekt |
| Off-thread gen (2dxX, CJ94, fS3V) | rayon pool + channel | **BEHOUD** | Superieur aan mutex-model |
| Rayon vs custom pool (QFQk) | rayon pool | **EVALUEREN** | Pas bij gemeten p99-jitter |
| Raycast block-pick (2dxX) | mist in client | **AANPASSEN** | Nodig voor edit-UI |
| BFS zonglift-lighting (fS3V) | geen | **AANPASSEN** | Goedkoop, covey cave-schaduw; natuurlijke volgende stap |
| God-rays (CJ94, fS3V) | directional+fog | **AANPASSEN (filmisch)** | Occlusion + radial blur, WGSL post-pass |
| Cellulaire automata fluids (fS3V) | geen | **EXPERIMENTEREN (klein)** | Water/lava/zand via `EditLog`-regels; pas bij <0,5 ms/chunk |
| Cubic chunks (CJ94) | Y-slab columnair | **VERWERPen nu** | Complexer; Fase 5 indien verticale LOD nodig |
| GPU-driven/indirect (xima1) | mesh-CG draw | **Fase 5** | D1/D2 eerder uitgesteld; zinvol >100k chunks |
| Octree (DouglasDwyer) | grid | **Fase 5** | Slechts voor sparse/scale-fase |
| Voxel rigid-body (dphfox/Douglas/PJEm) | avatar-collision | **VERWERPen** | PJEm: O(N³), 6×+ duurder dan primitieven. Hybride Jolt-primitieven later |
| Memory-locality Z-order/3D-texture (ztkh) | dense flat rij-orde | **AANPASSEN (laag risico)** | Toepasbaar op `Chunk`-backing-store; meet chunks/s |
| **Voxel ray traversal / path-tracing (ztkh, tDTB)** | triangle-raster | **LATE FASE (additief)** | Zie §2 — filmische laag, geen rewrite |
| Volumetric clouds/weather (vqWz) | geen | **FILMISCHE FASE** | Low-res volume raymarch + jitter; koppel `time_of_day` |
| SVO / DAG / AMR / OpenVDB / NanoVDB | geen | **VERWERPen nu** | Andere renderpijplijn; lossen onze bottleneck niet |
| godot_voxel | referentie | **BEHOUD als referentie** | MIT, beste levende architectuur-vergelijk |
| Veloren | referentie | **VERWERPen (GPL-3.0)** | Licentie-incompatibel als code-bron |

---

## 2. Ray-tracing-route (jouw wens: Crimson Desert / Lay of the Land-niveau)

**Mijn advies: voxel ray marching / DDA-traversal in een wgpu-compute shader als additieve filmische laag — géén HW-DXR-primair, géén SVO-opslag-rewrite.**

Waarom:
- Wij zijn een voxel-engine → we hebben onze eigen wereld als acceleratiestructuur. DDA (Amanatides-Woo) over onze chunks is native en backend-portable (wgpu/WebGPU).
- HW ray tracing (DXR/BVH) is niet first-class in wgpu en vervangt onze werkende raster-mesh onnodig.
- De ray-traversal-demo's (ztkh, tDTB) bewijzen: per-pixel DDA + 3D-texture-locality = 100+ FPS op mid GPU's. Op jouw RTX 4080 Super is dit ruim haalbaar als *secundaire* lighting/volumetric-pass.

**Gefaseerd pad (elk een tracer-bullet):**
1. **RTAO + zachte schaduwen** via DDA over voxelwereld in compute; vergelijk met huidige vertex-AO.
2. **Voxel GI** (enkele bounce / DDGI-proben) voor indirect light in grotten/onder overhangen.
3. **Volumetric integratie** (wolken/mist zoals vqWz).
4. **Filmic post-stack** (skybox gradient, distance fog, bloom, ACES tonemap).

Geen daarvan is een rewrite. Ze zijn *additief* bovenop de werkende client.

**Belangrijke nuance:** Crimson Desert en Lay of the Land zijn géén voxel-engines (UE5/Nanite/Lumen resp. onbekend). "Op hun niveau komen" = de *look* halen (filmische lighting, schaal, dichtheid), niet hun techniek kopiëren. Wat "Lay of the Land" exact is, hebben we nog **niet** geverifieerd — het staat niet in de geleverde dossiers en ik beweer niet dat we hun tech moeten matchen zonder dat te weten.

---

## 3. Aanbevolen volgorde (meetbaar, strict TDD)

1. **Crack-free skirts** (LOD-bug fix) — klein, veilig, hoogste ROI op bestaande LOD.
2. **Inter-chunk occlusie** (LxVL E1) — grootste verwachte chunk-reductie bij 12,5cm/r48.
3. **BFS zonglift-lighting** (fS3V E8) — cave-schaduw, goedkoop.
4. **Raycast block-pick** (2dxX E3) — edit-tool UI.
5. **God-rays** (CJ94/fS3V E6) — filmische juice.
6. **Voxel RTAO/RTGI compute-spike** (jouw ray-tracing-wens) — aparte ADR + tracer-bullet.
7. **Volumetric clouds/weather** (vqWz E8/E9) — latere filmische fase.
8. **Cellulaire automata fluids** (fS3V E9) — klein experiment.
9. **Memory-locality Z-order/3D-texture** (ztkh E1) — backing-store eval.
10. **GPU-driven/indirect + octree** — Fase 5, pas bij >100k chunks.

---

## 4. Licentie-waarschuwingen (hard)

- **Transvoxel**: proprietary (Terathon). Nooit de tabel kopiëren. Eigen stitching schrijven of licentie kopen.
- **Veloren**: GPL-3.0 → nooit als code-bron voor onze permissieve engine.
- **godot_voxel**: MIT → inspiratie toegestaan, geen fork zonder attribuut.
- **DeadlockCode/voxel_ray_traversal**: Apache-2.0 → compatibel met onze MIT/Apache stack.
- **frozein/DoonEngine**: MIT → compatibel.

---

## 5. Conclusie

Geen enkele rewrite geadviseerd. Onze codebase is solide; de roadmap is een reeks kleine, meetbare experimenten die de engine van "werkende vertical slice" naar "filmische voxel-wereld" tillen — exact jouw noordster, zonder hobby-engine-rot en zonder halve maatregelen.
