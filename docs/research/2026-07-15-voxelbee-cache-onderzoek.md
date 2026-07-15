# VoxelBee cache-onderzoek — toepasbaarheid op Land of the Voxel Engine

**Datum:** 2026-07-15
**Bron:** YouTube-kanaal VoxelBee (C++/Vulkan voxel-engine "Voxel Universe"), devlogs #1, #4, #5, #7.
**Doel:** bepalen of VoxelBee's chunk/dataset-cache-architectuur bruikbaar is voor onze engine (Rust + wgpu, micro-voxel 12,5 cm, chunk=4 m, 1 km² = 62.500 chunks, ~150 km² filmische openwereld, RTX 4080 Super / 32 GB RAM).

## 1. Wat VoxelBee doet (cache-architectuur)
VoxelBee rendert via **voxel-octree-raycasting** (DDA-traversal, SDF+octree-hybride voor sparse data, LOD via octree). De cache (Devlog #4 → #5) werkt als volgt:
- **GPU-resident 1D voxel-array**: alle voxels staan als index in één platte buffer op de GPU (voorheen in "pages", maar dat maakte de algoritmes te complex → terug naar één grote array).
- **Move-to-front evictie**: bij render wordt een voxel naar vóór in de array geschoven; niet-recente voxels zakken naar achteren en worden aan de achterkant overschreven (evictie). Geen volledige sort — VoxelBee verwierp sorteren van 10–20M elementen/frame als te traag.
- **Compute-shader-gestuurd**: de GPU beheert zijn eigen cache; voxels die vaak samen bekeken worden, clusteren in geheugen (locality-winst).
- **Virtuele adresruimte**: tot 64 GB aan voxels adresseerbaar als "één grote boom"; fysiek resident slechts een klein deel (getest tot 1 MB GPU-cache, normaal enkele GB).
- **Morton-codes** voor 3D→1D encoding met goede cache-locality.
- **CPU-cache** (gepland, Devlog #4/#5) voor grotere voxelopslag, met unload naar disk/netwerk.

## 2. Universeel bruikbaar vs specifiek voor zijn engine
**Universeel (direct bruikbaar voor ons):**
- Recency/MRU-gebaseerde residency i.p.v. alles resident houden.
- Virtuele chunk-ruimte + fysiek gebudgetteerde subset (onze 150 km² = virtueel, VRAM/RAM = fysiek).
- Geen full-sort evictie; O(1) "touch" bij gebruik.
- Morton/Z-order keys voor geheugenlocality van buffers.
- Tweelaags streaming-hiërarchie (GPU-cache + CPU-cache + disk).

**Specifiek voor zijn engine (niet 1:1 overneembaar):**
- Raycaster-datalayout (ruwe voxeldata i.p.v. meshes) — onze engine is mesh-based.
- Compute-shader self-managed cache binnen een raytracer; moeilijk te porten naar een wgpu-mesh-pipeline.
- SDF+octree-hybride renderer en de Vulkan/C++ stack.
- Move-to-front heeft lagere hit-rate dan echte LRU — bij ons niet nodig (zie aanbeveling).

## 3. Concrete aanbeveling voor Land of the Voxel Engine
Onze wereld: 150 km² = **9,4M chunks** (4 m), elk 32³ = 32.768 voxels. Ruwe voxeldata is ~300 GB — veel meer dan 32 GB RAM, dus **streaming is verplicht**. VoxelBee's les: beheer residency expliciet per cache-laag.

**Voeg een 3-laags cache toe, met de GPU-buffer-cache als prioriteit:**
- **L1 — GPU-buffer-cache (VRAM, ~6 GB van 16 GB):** LRU-per-viewpoint van geüploade chunk-meshes (vertex+index). Key = chunk-coord (Morton). Touch bij zichtbaarheid; evict minst-recente + verste bij camerabeweging. *Dit is het directe analogon van VoxelBee's recency-cache, maar dan voor mesh-buffers in de wgpu-pipeline.*
- **L2 — Mesh-cache (RAM, ~10 GB):** LRU van CPU-gegenereerde meshes, zodat her-betreden gebied niet opnieuw gemesht wordt.
- **L3 — Worldgen-cache (disk):** gecomprimeerde chunk-voxeldata + meshes, onbeperkt, load-on-demand.

**Waarom echte LRU/ARC i.p.v. move-to-front:** ons item-aantal (chunks, actief tienduizenden) is klein genoeg voor O(log n) LRU/ARC → betere hit-rate dan VoxelBee's O(1) move-to-front, zónder de sorteer-kost die hij afwees. **Mesh-deduplicatie:** hash identieke chunk-meshes (biome/herhaling) en deel één GPU-buffer — VoxelBee deed dit niet expliciet, maar zijn "samen-clusterende voxels"-idee ondersteunt het.

## 4. Bronnen
1. VoxelBee — *Adding A CACHE To My Custom VOXEL Game Engine | Devlog #5* (GPU-array, move-to-front, 1 MB-test): https://www.youtube.com/watch?v=i7vq-HY10hI
2. VoxelBee — *DEBUGGING MY VULKAN GAME ENGINE | Devlog #4* (pages→array, sort verworpen): https://www.youtube.com/watch?v=TPg_LwWM0Bo
3. VoxelBee — *VOXEL Rendering And Traversal Algorithms | Devlog #7* (DDA, SDF+octree, Morton): https://www.youtube.com/watch?v=NzVOPyWvBcw
4. Crassin — *GigaVoxels* (bricked clipmap + LRU/AFC GPU-cache, octree-streaming): https://maverick.inria.fr/Publications/2011/Cra11/
5. Laine & Karras — *Efficient Sparse Voxel Octrees* (NVIDIA, 2010): https://research.nvidia.com/publication/2010-02_efficient-sparse-voxel-octrees-analysis-extensions-and-rendering
