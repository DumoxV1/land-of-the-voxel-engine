# ADR-0006: Voxel Ray Marching als additieve filmische laag

**Status:** Voorgesteld (2026-07-15)
**Deciders:** Hermes (lead), gebruiker
**Context:** Gebruiker wil ray-tracing/volumetric-technieken om op filmisch niveau van Crimson Desert / Lay of the Land te komen.

## Besluit

Wij voegen **geen** hardware ray tracing (DXR/BVH) toe als primaire renderer, en wij vervangen onze chunk-opslag **niet** door een sparse voxel octree/DAG. In plaats daarvan bouwen we een **additieve filmische lighting-laag** bovenop de bestaande greedy-mesh → wgpu-rasterizer:

- Voxel **ray marching / DDA-traversal** (Amanatides-Woo) in een **wgpu-compute shader** over onze eigen voxelwereld (acceleratiestructuur = onze chunks).
- Deze laag levert achtereenvolgens: RTAO + zachte schaduwen → voxel GI (enkele bounce / DDGI-probes) → volumetrische wolken/mist → filmische post-stack (skybox, distance fog, bloom, ACES tonemap).
- Backend-portable via wgpu/WebGPU; draait op compute, géén RT-cores vereist (wel lonend op RTX 4080 Super).

## Rationale

1. Wij zijn een voxel-engine: onze wereld is de acceleratiestructuur. DDA over chunks is native en goedkoper dan BVH-opbouw voor dynamische terrain.
2. HW ray tracing is niet first-class in wgpu; een DXR-primair renderer vervangt onze werkende mesh-pipeline onnodig en breekt backend-portabiliteit.
3. De geleverde research bevestigt: ztkh (Rust+Vulkan voxel ray traversal, Apache-2.0) haalt 121 FPS @1000² op Intel Arc A770 puur via 3D-texture-locality; tDTB (DoonEngine, MIT) toont per-voxel shadows+GI via 64-bit-ID-hashmap-dedupe + Fibonacci-stratificatie + temporal accumulation. Beide zijn referentie-implementaties voor de late fase.
4. Crimson Desert (UE5/Nanite/Lumen) en Lay of the Land zijn géén voxel-engines; "op hun niveau komen" = de *look* halen, niet hun techniek kopiëren. Wat Lay of the Land exact is, is nog niet geverifieerd en wordt niet als architectuur-bron aangenomen zonder onderzoek.
5. SVO/DAG/AMR/OpenVDB/NanoVDB (uit adaptive-grid dossier) lossen onze bottleneck (streaming/scheduling) niet en vereisen een andere renderpijplijn → verworpen voor nu.

## Consequenties

**Voordelen:**
- Filmische lighting zonder rewrite van werkende code.
- Backend-portable, geen RT-hardware-afhankelijkheid.
- Elke stap is een isolated tracer-bullet met meetbare winst.

**Kosten/risico's:**
- Compute-shader complexiteit; DDA over streaming chunks vereist een stable voxel-view (chunks in flight correct afgehandeld).
- RTAO/RTGI kost GPU-time; moet gebudgetteerd naast raster-draw.
- Latere fasen (octree, GPU-driven indirect) blijven uitgesteld tot >100k chunks.

## Implementatie-fases (elk met tracer-bullet)

- **F1 — RTAO + zachte schaduwen:** DDA in compute over voxelwereld; vergelijk met huidige vertex-AO. Meting: AO-kwaliteit + frame-time bij r48/RTX4080.
- **F2 — Voxel GI:** enkele bounce / DDGI-probes voor indirect light in grotten/overhangen.
- **F3 — Volumetric:** low-res voxel-cloud-volume raymarch + jitter (zie vqWz-video), gekoppeld aan `time_of_day`.
- **F4 — Filmic post:** skybox gradient, distance fog, bloom, ACES tonemap.

## Licentie-notitie

- `DeadlockCode/voxel_ray_traversal` (Apache-2.0) en `frozein/DoonEngine` (MIT) zijn compatibel met onze MIT/Apache-2.0 stack als referentie. Attributie vereist bij eventuele code-overname; onze dual-licentie blijft intact.
- Geen Transvoxel-code overnemen (proprietary, Terathon).

## Referenties

- `docs/research/voxel-engine-survey-2026/direct/adaptive-voxel-grids.md` (§2.1–2.7)
- `docs/research/voxel-engine-survey-2026/direct/set-b.md` (§1 ztkh, §2 tDTB, §4 vqWz)
- `docs/research/voxel-engine-survey-2026/RETAIN_UPDATE_REPLACE_MATRIX.md` (§2)
- Amanatides & Woo, "A Fast Voxel Traversal Algorithm for Ray Tracing" (1987)
