# Onderzoek: VoxelBee cache-architectuur vs. Land of the Voxel Engine

**Datum:** 2026-07-15
**Kanaal:** [VoxelBee](https://www.youtube.com/@voxelbee) — C++ + Vulkan voxel engine ("Voxel Universe")
**Focus:** Devlog #5 "Adding A CACHE To My Custom VOXEL Game Engine" (feb 2021)

## 1. Wat VoxelBee doet (cache-architectuur in bullets)

- **GPU-eigen cache in één platte array.** Elke voxel is één index in een 1D-array op de GPU. Geen boom-allocatie, geen pagina's (Devlog #4 verwierp paginering wegens complexiteit).
- **Move-to-front / LRU-achtige evictie.** Wanneer een voxel gerenderd wordt, schuift hij naar vóór in de array; niet-gebruikte voxels zakken naar achteren en worden daar overschreven. Geen sorteerpass (te traag: 10–20M elementen/frame).
- **Volledig in compute shaders.** De GPU beheert zijn eigen cache; voxels die samen gebruikt worden, clusteren in geheugen → betere spatiale locality.
- **Extreem schaalbaar.** Getest tot **1 MB** GPU-cache; kan theoretisch **64 GB** aan voxels adresseren. Bedoeld voor "bijna oneindige" wereld.
- **Cache-hiërarchie (later).** Devlog #5 kondigt een CPU-cache aan bovenop de GPU-cache (CPU houdt meer, GPU houdt de actief zichtbare set).

## 2. Engine-architectuur (Devlog #1–#7)

- **Taal/Renderer:** C++ + Vulkan, **voxel-octree raycasting** (geen polygon-meshes), LOD via octree-niveaus.
- **Streaming:** Camera-culling "on the fly"; voxels laden per zichtprioriteit, unload buiten gezichtsveld.
- **Multithreading:** Initieel **geen** (alles op render-thread); multithreading stond op de todo.
- **Coördinaten:** 128-bit precisie (32-bit float lokaal 8 km², geïndexeerd door 32/64-bit ints) voor planeet→stof-zoom.
- **GPU-upload:** Indirect — voxels streamen CPU→GPU, evictie beslist wat op GPU blijft (Devlog #3 "GPU out of pages").

## 3. Universeel vs. specifiek

**Specifiek voor VoxelBee:**
- SVO-raycasting + GPU-compute-cache is fundamenteel anders dan onze **polygon-mesh**-pipeline. Zijn "voxel"-cache zit op leaf-node-niveau in de octree; wij cachen op chunk/mesh-niveau op de CPU.
- Move-to-front in één array werkt bij zijn sparse-voxel-traversatie, niet bij onze VBO's.

**Universiteel bruikbaar:**
- **LRU / view-afhankelijke evictie** met decay-hysteresis (voorkomt thrashing net buiten beeld).
- **Tweeklagen-cache** (CPU-dataset-cache → GPU-buffer-cache) met scherpe GPU-cap.
- **Spatial locality**: voxels/tiles die samen zichtbaar zijn, samen cachen.
- **Prioriteit per gezichtspunt** i.p.v. simpele radius.

## 4. Aanbeveling voor onze engine (150 km², 32 GB, RTX 4080S)

Omdat onze wereldgenereratie **deterministisch** is (fBm + biome), hoeven we voxel-data niet te bewaren — hergenereren is goedkoop. Het dúúre is **meshen**.

**Voeg toe, in volgorde:**
1. **LRU mesh-cache (CPU)** per chunk `(x,z)`, keyed op laatst-zichtbaar-timestamp. Cap op bv. 8–12 GB RAM. Evict minst-recent-zichtbare chunks; houd mesh + (optioneel) ge-bakken vertexdata.
2. **View-afhankelijke GPU-VBO-pool-LRU** (nu blinde 256 MB-cap). Koppel evictie aan zichtbaarheid + hysteresis zodat rondvliegen niet thrasht.
3. **Geen aparte worldgen-cache nodig** — deterministic, dus regenereer op cache-miss. Bespaart RAM.
4. **Prioriteits-streaming**: meshen op nabijheid-tot-camera, niet alleen radius-24.

**Onderbouwing 150 km²:** 150 km² = 9,375M chunks; radius-24 laadt ~1.809 chunks tegelijk. Een mesh van 4 m-chunk micro-voxels ≈ 50–200 KB; 12 GB RAM dekt ~60k–240k meshes ruim boven de actieve set. 256 MB GPU (≈ enkele duizenden chunks) dekt de zichtbare set; LRU houdt die fris. Dit geeft "oneindige" wereld zonder alles in RAM, analoog aan VoxelBee's GPU-cache-idee maar op mesh-niveau.

## 5. Bronverwijzingen

1. VoxelBee, *Adding A CACHE To My Custom VOXEL Game Engine | Devlog #5* — https://www.youtube.com/watch?v=i7vq-HY10hI
2. VoxelBee, *DEBUGGING MY VULKAN GAME ENGINE | Devlog #4* (paginering→platte array) — https://www.youtube.com/watch?v=TPg_LwWM0Bo
3. VoxelBee, *INFINITE ZOOM | Devlog #3* (GPU-out-of-pages, cache-motief) — https://www.youtube.com/watch?v=JmuLQrtvdO8
4. Crassin et al., *GigaVoxels* (LRU GPU-cache, SVO) — https://maverick.inria.fr/Publications/2009/CNLE09/CNLE09.pdf
5. Laine & Karras, *Efficient Sparse Voxel Octrees* — https://research.nvidia.com/sites/default/files/pubs/2010-02_Efficient-Sparse-Voxel/laine2010tr1_paper.pdf
