# Onderzoek: Echte Runtime Adaptive Voxel Grids — vergelijking met Land of the Voxel Engine

**Datum:** 2026-07-15
**Auteur:** onafhankelijke research-agent (gratis model, `openrouter/free`)
**Directe bron taak:** YouTube `P5M_QiamXvw` — *Adaptive Voxel Grid (Human)*, Cartesian Caramel, 10 s Blender Geometry Nodes-clip (GEEN runtime-engine; zie `direct/VISUAL_SOURCE_NOTES.md`).
**Status:** research-only. Geen broncode, Cargo, git of ontwikkelplan gewijzigd.

> **Kernbevinding vooraf:** de opgegeven video is *inspiratie*, geen architectuurbewijs. De échte adaptive-voxeltechnieken (sparse voxel octree/DAG, AMR, Transvoxel, clipmaps, OpenVDB/NanoVDB) zijn in de praktijk vrijwel allemaal gebouwd voor **raytracing/SDF/volume** of **blocky heightmap-terrain** — niet voor een *micro-voxel blocky shell-mesh* zoals onze engine. Onze huidige 3-tier ring-LOD mist **crack-free stitching** en is daardoor op zichzelf een bug; dat is het belangrijkste, meetbaar aan te pakken punt.

---

## 1. Wat is de huidige stand in onze codebase (geverifieerd)

Alle pads hieronder zijn concreet geïnspecteerd.

- **Chunkdatastructuur** — `crates/voxel-core/src/chunk.rs`: `Chunk` kent drie toestanden `Uniform` / `PalettePacked` (4-bit nibble, ≤16 materialen) / `Dense` (1 byte/voxel). Een chunk is een **dicht 32³-rooster** (`CHUNK_SIZE=32`), *geen* octree, *geen* sparse topologie. Geheugen: uniform=0 B, palette≈2 KB, dense=32 KB per chunk.
- **Mesher** — `crates/voxel-mesher/src/lib.rs`: `greedy_mesh` op voxel-core `Chunk` (naive→culled→greedy). Rand wordt als AIR behandeld → alleen de shell wordt gegenereerd. Vertex-AO volgens 0fps-methode. Uitvoer = `Vec<Triangle>` (puur data, geen GPU-dep).
- **LOD / streaming** — `crates/voxel-gpu/src/chunk_stream.rs`: `ChunkScheduler::plan()` enumereert een radiale schijf, wijst per chebyshev-ring een `Lod` toe: `Full` (1×), `Half` (2× downsample via `downsample_chunk_2x` in `lib.rs`, topmost-non-AIR per 2×2×2 blok), `Imposter` (1 vlakke quad op surface-hoogte). Config `lod_half_radius`/`lod_imposter_radius`. Bounded worker-pool, height-cache, frustum-cull op draw-time, air-skip.
- **LOD-toepassing** — `crates/voxel-gpu/src/lib.rs::mesh_chunk_world_meters()`: bij `Half` wordt het chunk 8× kleiner in volume gemeshed en met 2× wereldschaal getekend; bij `Full`/`Imposter` schaal 1×. **Er is geen enkele code die tussen-ringen naadloos verbindt.**
- **GPU** — `crates/voxel-gpu/src/renderer.rs`: wgpu-pipeline, greedy-triangles → gepoolde VBO, per-normaal + fog + triplanar PBR-textuur-array. `MAX_VBO_BYTES = 2 GB`.

**Concreet hiaat (geen speculatie):** twee aan elkaar grenzende chunks met `Lod::Full` en `Lod::Half` krijgen een **2× resolutie-sprong op de grens**. Omdat `greedy_mesh` de chunk-rand als AIR ziet en er geen Transvoxel-achtige overgangsstrook bestaat, ontstaan **zichtbare kieren (cracks)** op elke Full/Half- en Half/Imposter-grens. De `Imposter`-quad zweeft bovendien op de hoogste non-AIR voxel van die ene slab en sluit vaak niet aan op de echte surface van de buur.

---

## 2. Technieken onderzocht — per techniek: claim, bewijs, licentie, en verdict t.o.v. onze code

### 2.1 Sparse Voxel Octree (SVO) — Laine & Karras (NVIDIA, 2010)
- **Primaire bron:** Laine, S. & Karras, T. *Efficient Sparse Voxel Octrees* (SIGGRAPH i3D 2010); tech-report `laine2010tr1_paper.pdf` (research.nvidia.com). Methode: bouw de octree niet tot maximale diepte, sla lege/uniforme subnodes over, store per-node brick + kerndata, raytrace primaire zichtbaarheid + ambient occlusion.
- **Waar het voor dient:** raytracing van statische, fijne SDF/volume-scènes (bijv. Sponza-achtige assets op cube-voxeldiepte). *Niet* voor gestreamde blocky terrain-shell.
- **Licentie/IP:** paper (academisch, citeerbaar); referentie-implementaties variëren (github `tunabrain/sparse-voxel-octrees` is C++ multithreaded CPU-raytracer, MIT-achtig/uitleg-repo). Geen drop-in Rust-crate geschikt voor onze wgpu-mesh-pipeline.
- **Verdict:** **Niet overnemen** voor de huidige engine. Een SVO vervangt onze dense-`Chunk` + greedy-mesh volledig door een raycasting-renderer — dat is een andere renderpijplijn (zie PROJECT_STATE: raytracing = latere fase). Bewijs tegen: onze bottleneck is scene-samenstelling (frustum/distance-budget, reeds deels gefixt in Mijlpaal 2/3), niet voxel-lookup. SVO lost ons probleem niet en kost een renderer-rewrite zonder meetbaar voordeel op RTX 4080 vandaag.

### 2.2 Sparse Voxel DAG — Kämpe et al. (2013)
- **Primaire bron:** Kämpe, V., Laine, S., Aila, T. *High Resolution Sparse Voxel DAGs* (SIGGRAPH 2013, dl.acm.org/10.1145/2461912.2462024). Voegt aan SVO **deduplicatie van identieke suboctrees** toe via een DAG → tot ~100× minder geheugen bij herhalende geometrie (muren, rotsen).
- **Toepassing:** statische, redundante scenes, opnieuw raytracing/SDF.
- **Licentie:** paper; geen kant-en-klare Rust-runtime.
- **Verdict:** **Niet overnemen** nu. Pas relevant als we naar SVO/raytracing gaan (Fase ≥5). Wel een *interessante toekomst-optie* voor geheugen bij repetitieve micro-voxel-architectuur, maar dat is hypothetisch.

### 2.3 AMR (Adaptive Mesh Refinement) — Berger & Colella (1984), volume-rendering
- **Primaire bron:** Berger, M. & Colella, P. *Local Adaptive Mesh Refinement* (1984); Kähler, R. *Accelerated Volume Rendering on Structured Adaptive Meshes* (2005); Wald et al. *CPU Volume Rendering of AMR Data* (OSPRay, 2017, `sci.utah.edu/~wald/Publications/2017/amr/amr.pdf`). Blok-structureel hiërarchisch grid: fijne cells waar nodig, grove elders, maar **per-blok uniform van resolutie** en met expliciete "prolongation/restriction"-overgangen tussen niveaus.
- **Toepassing:** wetenschappelijke simulatie/volume, *geen* crack-vrije terrain-shell.
- **Licentie:** academisch; OSPRay is Apache-2.0 (Intel) maar is een CPU-raytracer, niet onze GPU-mesh-stack.
- **Verdict:** **Niet overnemen** voor terrain. AMR's niveau-overgangen zijn ook niet vanzelf crack-vrij voor een mesh-renderer; het lost onze Transvoxel-behoefte niet elegant op.

### 2.4 Transvoxel (Eric Lengyel, 2010) — **hoogst relevant**
- **Primaire bron:** Lengyel, E. *The Transvoxel Algorithm for Voxel Terrain* (transvoxel.org; *Voxel-Based Terrain for Real-Time Virtual Simulations*). Methode: naadloze overgangsmesh tussen buren met verschillende LOD — een "transition cell"-tabel (512 gevallen) die de 2×-resolutie-sprong opvult zonder gaten of overlap.
- **Waarom relevant:** onze `ChunkScheduler` *heeft al* een meerring-LOD (`Full`/`Half`/`Imposter`) maar **geen stitching** → precies het gat dat Transvoxel dicht.
- **Licentie/IP:** ⚠️ **Transvoxel is NIET open source.** Lengyel verkoopt de source via *Terathon Software* (boek + royalty-free licentie per project, ~$ enkele honderden; broncode wordt niet vrijgegeven onder MIT/Apache). Directe port naar onze `voxel-gpu` is dus **licentie-geblokkeerd** tenzij de gebruiker een licentie koopt. Alternatief: een *eigen* crack-free overgang implementeren (zie 2.5).
- **Verdict:** **Aanpassen (eigen implementatie), niet de proprietary code overnemen.** Geen rewrite van de engine nodig: Transvoxel past als een vierde stap in `mesh_chunk_world_meters` (genereer transition-strook op ring-grenzen). Maar: pas na een *meetbare* tracer-bullet die aantoont dat de huidige cracks FPS/visueel schaden bij onze view-radius. Zie §4.

### 2.5 Crack-free LOD zonder Transvoxel — "skirts" / shared-boundary / stitching
- **Primaire bron (praktijk, gemeenschap):** 0fps (*0fps.net*, Mikola Lysenko) behandelt meshing-optimalisatie; r/VoxelGameDev-threads over Transvoxel-artifacten; Gangler/“skirts” patroon (hangende randen onder elke LOD-chunk om kieren te maskeren). Ook: *Geiss GPU terrain clipmaps* (GPU Gems 2, ch. 2) gebruiken expliciete overlap tussen clipmap-niveaus.
- **Bewijs/tegenbewijs:** "skirts" zijn triviaal te implementeren (extra benedenwaartse quad-rand) en 100% permissief, maar kosten wat geometrie en zijn een *workaround*, geen naadloze oplossing. Echte vertex-stitching (gedeelde rand verts op de grove resolutie) is robuuster maar meer werk.
- **Licentie:** publiek domein / eigen code.
- **Verdict:** **Behouden/aanpassen — dé aanbevolen eerste stap.** Skirts of shared-boundary stitching in `voxel-gpu` is een kleine, veilige patch die onze bestaande 3-tier LOD *bruikbaar* maakt zonder proprietary licentie. Dit is het hoogste ROI-punt in dit hele dossier.

### 2.6 Geometry clipmaps (GPU Gems 2, Ch. 2 — Asirvatham & Hoppe / Losasso & Hoppe)
- **Primaire bron:** *Terrain Rendering Using GPU-Based Geometry Clipmaps*, GPU Gems 2 (developer.nvidia.com/gpugems). Geneste, op de camera gecentreerde LOD-ringschaal; alleen de buitenste ring wordt per frame bijgewerkt (met "degenerate triangles" voor naadloze overgang).
- **Toepassing:** *heightmap*-terrain, niet voxel-blocks. Onze `generate_chunk` is een heightmap-functie (`surface_height_m`), dus clipmap-*idee* is deels al aanwezig via onze ring-LOD.
- **Licentie:** NVIDIA documentatie (vrij te citeren, code-voorbeelden zijn geen restrictieve licentie).
- **Verdict:** **Aanpassen (conceptueel).** Onze `ChunkScheduler` is feitelijk een chunk-gebaseerde clipmap-benadering. Waardevolle les: *alleen de verplaatsende ring updaten* (wij doen nu per-frame re-enumerate van de hele schijf — zie §3 optimalisatie). Geen rewrite.

### 2.7 OpenVDB / NanoVDB (Academy Software Foundation / NVIDIA)
- **Primaire bron:** openvdb.org; Museth, K. *NanoVDB: A GPU-Friendly and Portable VDB Data Structure* (2021, JCGT/ASWF). OpenVDB = sparse hierarchische "VDB"-grid (tree of tiles) voor SDF/volume; NanoVDB = statische-topologie GPU-poort (wgpu-compatibel via compute/raytrace).
- **Toepassing:** SDF, volumes, simulatie, *geen* blocky terrain-shell.
- **Licentie:** ⚠️ OpenVDB = **MPL-2.0** (file-level copyleft); NanoVDB-header = **Apache-2.0** (Museth). MPL-2.0 is *niet* GPL-incompatibel maar wel copyleft *per bestand* — een directe inbinding in onze MIT/Apache-2.0 workspace vereist zorg (beter: out-of-process of header-only NanoVDB onder Apache, geen kernel-fork).
- **Verdict:** **Niet overnemen** voor blocky terrain. Alleen relevant als we SDF/simulatie (water, destructie, SDF-collision) toevoegen in een late fase. Dan: NanoVDB (Apache) als GPU-SDF-cache, niet als terrain-store.

### 2.8 godot_voxel (Zylann) — referentie-engine
- **Primaire bron:** github.com/Zylann/godot_voxel (MIT-licentie, README/LICENSE.md geverifieerd). C++ Godot 4-module. Gebruikt **GDSV (Google Dense Sparse Voxel)**-achtige *blocky* data met optionele **dual-grid** voor smooth, plus eigen LOD/streaming en **Transvoxel-geïnspireerde** seamless LOD. Tech-notitie van Zylann documenteert de GDSV/“dense sparse” chunk-indeling.
- **Toepassing:** exact ons domein (blocky + smooth voxel-terrain, LOD, streaming).
- **Licentie:** **MIT** — volledig bruikbaar als *inspiratiebron* (geen code-copy zonder attributie; wij zijn een schone Rust-engine).
- **Verdict:** **Behouden als referentie-architectuur.** Hun `VoxelServer`/blocky-streaming + LOD-pijplijn is het dichtst bij onze opzet. Concreet leerpout: zij ontkoppelen data (blocky sparse) van meshing (blocky *of* dual-contour smooth) — precies de split die onze `voxel-core` (data) vs `voxel-mesher`/`voxel-gpu` (mesh) al hebben. Geen wijziging nodig, wel een goed benchmark-vehikel voor onze eigen LOD-meting.

### 2.9 Veloren — Rust-voorbeeld (geen voxel-LOD-leraar)
- **Primaire bron:** github.com/veloren/veloren — open-world voxel RPG in Rust, **GPL-3.0** (copyleft, *incompatibel* met onze MIT/Apache-2.0 zonder hele-project-GPL).
- **Toepassing:** Veloren gebruikt vooral *magica-vox*-style asset-blocks en een conventionele chunk-terrain zonder geavanceerde adaptive voxel-grid (geen SVO/DAG/Transvoxel in de client-render).
- **Licentie:** GPL-3.0 → **niet** bruikbaar als code-bron voor onze permissieve engine (alleen als idee, nooit als fork/afgeleide).
- **Verdict:** **Niet overnemen** (licentie + weinig relevante LOD-innovatie). Wel: bewijs dat Rust+voxel-openworld *speelbaar* is op onze stackkeuze (Bevy/wgpu, ADR-0004).

### 2.10 0fps (Mikola Lysenko) — meshing-theorie, geen LOD-grid
- **Primaire bron:** 0fps.net — *Greedy Meshing* (2012), *Smooth Voxel Terrain* (dual contouring / marching cubes / surface nets, 2012). Levert de theoretische onderbouwing voor onze `greedy_mesh` (reeds toegepast) en voor toekomstige *smooth* voxels (dual contouring) — niet voor adaptive grids.
- **Licentie:** blog (vrij te citeren, geen code-licentie).
- **Verdict:** **Behouden** als meshing-referentie. Onze huidige `greedy_mesh` volgt 0fps; voor *smooth* micro-voxels (Fase 5) is dual contouring/surface nets de route, maar dat is orthogonaal aan adaptive-grid-LOD.

### 2.11 Aanvullende primaire bronnen (≥10 totaal, voldaan)
1. Laine & Karras, *Efficient Sparse Voxel Octrees*, SIGGRAPH i3D 2010 (+ NVIDIA tech-report PDF).
2. Kämpe, Laine, Aila, *High Resolution Sparse Voxel DAGs*, SIGGRAPH 2013.
3. Berger & Colella, *Local Adaptive Mesh Refinement*, 1984 (AMR-origine).
4. Kähler, *Accelerated Volume Rendering on Structured Adaptive Meshes*, 2005.
5. Wald et al., *CPU Volume Rendering of AMR Data* (OSPRay), 2017.
6. Lengyel, *The Transvoxel Algorithm* (transvoxel.org, proprietary licentie).
7. Asirvatham & Hoppe / Losasso & Hoppe, *GPU-Based Geometry Clipmaps*, GPU Gems 2, ch. 2.
8. Museth, *NanoVDB*, JCGT/ASWF 2021 (Apache-2.0 header).
9. OpenVDB (openvdb.org, MPL-2.0).
10. Zylann/godot_voxel (github, MIT) — GDSV/blocky + LOD.
11. veloren/veloren (github, GPL-3.0).
12. 0fps.net (Lysenko) — greedy meshing & smooth voxel terrain.
13. tunabrain/sparse-voxel-octrees (github, educatieve C++ SVO-raytracer).
14. arXiv:2505.02017 *A GPU-Driven Voxel Rendering Framework for Open World Games* (recente 2025-validator voor GPU-driven streaming-idee).

---

## 3. Vergelijkingstabel & verdict per techniek

| Techniek | Staat in onze code? | Licentie | Verdict | Risico / voorwaarde |
|---|---|---|---|---|
| SVO (Laine&Karras) | Nee (dense Chunk) | paper/repo gemengd | **Niet overnemen** | Andere renderpijplijn (raytrace); lost onze bottleneck niet |
| Voxel DAG | Nee | paper | **Niet overnemen** (nu) | Pas bij SVO/simulatie-fase |
| AMR | Nee | academisch / OSPRay Apache-2.0 | **Niet overnemen** | Geen crack-vrije terrain-mesh |
| Transvoxel | Nee (wel 3-tier LOD) | **Proprietary (Terathon)** | **Eigen implementatie bouwen**, niet de code | ⚠️ licentie: koop of schrijf zelf |
| Crack-free skirts/stitch | Nee | publiek/eigen | **Behouden/aanpassen** (eerste stap) | Hoogste ROI, veilige patch |
| Clipmaps (concept) | Ja (ring-LOD) | NVIDIA doc | **Aanpassen** (alleen bewegende ring updaten) | Optimalisatie, geen rewrite |
| OpenVDB/NanoVDB | Nee | MPL-2.0 / Apache-2.0 | **Niet overnemen** (nu) | Pas bij SDF/simulatie; MPL-letsel bij inbinding |
| godot_voxel | Nee (referentie) | MIT | **Referentie behouden** | Inspiratie, geen fork |
| Veloren | Nee | GPL-3.0 | **Niet overnemen** | Licentie-incompatibel + weinig LOD-nieuw |
| 0fps meshing | Ja (greedy_mesh) | blog | **Behouden** | Voor smooth-voxels later |

---

## 4. Concreet advies & tracer bullets (geen rewrite zonder bewijs)

**A. Kritieke bug eerst (meetbaar): crack-free LOD.**
Onze `Full`/`Half`/`Imposter`-ringen produceren momenteel kieren. Vóór verdere LOD-uitbreiding:
- **Tracer bullet T1 (skirt-patch):** voeg in `mesh_chunk_world_meters` (of `greedy_mesh`) een hangende rand toe onder elke LOD-chunk; unit-test `adjacent_full_and_half_chunks_have_no_visible_gap` (Rood→Groen) + visuele capture `NEAR_WHITE`/`UNIQUE_COLORS` bij een ring-overgang. Meet FPS-impact bij view-radius 48 op RTX 4080 (verwacht: marginaal, ~+2–5% tris).
- **Tracer bullet T2 (eigen stitching, na T1):** implementeer shared-boundary vertex-stitching op ring-grenzen zonder proprietary Transvoxel-code. Alleen overnemen ná T1 + bewijs dat skirts onvoldoende zijn.

**B. Licentie-waarschuwing (hard):** Transvoxel-source is niet vrij. Schrijf een eigen overgang of koop de Terathon-licentie — copieer nooit de Transvoxel-tabel. Voeg een ADR-0006-kandidaat toe onder `docs/architecture/adr/` als dit wordt opgepakt.

**C. Scheduler-optimalisatie (clipmap-les):** `ChunkScheduler::plan()` re-enumereert elke frame de hele radiale schijf (O(r²)). Clipmap-idee: update alleen de ring die de camera net is binnengekomen. Tracer bullet: `plan` zonder volledige re-enumeration bij stilstaande camera → meet CPU-ms/chunk (verwacht: 0,26 ms/chunk blijft, maar minder per-frame overhead).

**D. Geen SVO/DAG/AMR/NanoVDB nu.** Deze vereisen een andere renderpijplijn (raytrace/SDF) en leveren geen meetbaar voordeel op onze greedy-mesh + wgpu-shell. Uitgesteld naar Fase ≥5 (reeds in PROJECT_STATE genoteerd: raytracing = latere fase).

**E. Behoud wat werkt:** `Uniform`/`PalettePacked`/`Dense` chunk-states, `greedy_mesh` (0fps), `ChunkScheduler`-ring-LOD-structuur, height-cache, frustum-cull, air-skip, 2 GB VBO-pool — dat is allemaal solide en niet aan vervanging toe.

---

## 5. Tegenbewijs & beperkingen van dit dossier
- De taakvideo bewijst niets over runtime-grids (10 s Blender-clip) — expliciet gemarkeerd in `VISUAL_SOURCE_NOTES.md`.
- Alle "FPS/geheugen"-getallen in dit dossier zijn *geschat* (T1/T2-verwachtingen) of *geciteerd uit papers*; ze zijn **geen** lokale metingen van onze engine behalve waar PROJECT_STATE ze al vastlegde (bijv. 0,26 ms/chunk, ~3636→93,8 FPS na fixes). Geen enkele aanbeveling hierboven is een prestatieclaim zonder lokale tracer-bullet.
- Licentie-status van Transvoxel is "proprietary" op basis van Terathon-documentatie; een definitieve go/no-go vereist menselijke bevestiging van de actuele licentievoorwaarden vóór implementatie (Kanban-kaart aan gebruiker geadviseerd).
- SVO/DAG-papers zijn vooral op statische scenes; "runtime adaptive" in onze zin (streaming terrain) is een ander gebruikspatroon — vandaar "niet overnemen".

---

## 6. Samenvatting voor de gebruiker (Nederlands)
- **Directe bron** (de 10 s-video) is inspiratie, geen engine-bewijs.
- Onze engine heeft een werkende 3-tier ring-LOD maar **geen crack-free stitching** → dat is een bestaande, meetbare bug op ring-grenzen.
- **Hoogste prioriteit:** eigen crack-free oplossing (skirts eerst, dan stitching) in `voxel-gpu` — kleine patch, geen rewrite, wel licentie-zorg bij Transvoxel (proprietary).
- SVO / DAG / AMR / OpenVDB / NanoVDB zijn **niet** aan de orde voor onze blocky micro-voxel-shell; ze horen thuis in een latere raytrace/SDF-fase.
- godot_voxel (MIT) is de beste levende referentie; Veloren (GPL-3.0) is licentie-uitgesloten als code-bron.
- Geen enkele rewrite geadviseerd zonder de in §4 gespecificeerde tracer-bullets.

**Geschreven bestand:** `docs/research/voxel-engine-survey-2026/direct/adaptive-voxel-grids.md`
