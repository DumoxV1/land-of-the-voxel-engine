# Eerste codebase-vergelijking — Voxel Engine Survey (lokale bronnen)

Status: **voorlopig**. De drie researchsubagents zijn beëindigd door een OpenRouter-creditlimiet (`HTTP 402`) en hebben geen dossiers geschreven. Dit document is gebaseerd op:
- 7 lokaal gedownloade transcripties;
- 3 visueel geanalyseerde video’s + metadata;
- onze actuele codebase (crates/voxel-core, voxel-world, voxel-worldgen, voxel-mesher, voxel-gpu, voxel-player);
- bestaande voxel-engineliteratuur (0fps, Laine & Karras, Transvoxel, OpenVDB/NanoVDB).

**Statuslegenda:** `[B]` bewezen uit directe bron/transcript/metadata, `[O]` onzeker/specultief, `[K]` algemene vakkennis, `[V]` visueel waargenomen.

---

## 1. Concreet vergelijkbare technieken

### 1.1 Micro-voxelgrid en celgrootte
- Onze engine: `VOXEL_SIZE_M = 0.125` (12.5 cm), `CHUNK_SIZE = 32`. Wereld is homogeen fijn.
- `fS3V` [B]: claimt 8×8×8 micro-voxelgrid, maar de “micro” in die video is waarschijnlijk grover dan onze 12.5 cm; exacte voxelgrootte nog live verifiëren.
- `P5M` [V]: toont adaptieve celgrootte op een model, maar is een Blender Geometry Nodes-demo, **geen** runtime-enginebewijs.

**Advies:** behoud homogeen 12.5 cm voor nu. Echte adaptive grids (zie §3) zijn een aparte, grotere investering.

### 1.2 Greedy meshing
- Onze `voxel-mesher` doet greedy meshing + vertex-AO [K/B uit eerdere commits].
- `fS3V` [B]: claimt greedy meshing als kernfeature.

**Advies:** behoud; onze implementatie is al state-of-the-art voor dit schaalniveau.

### 1.3 LOD / HLOD
- Onze `ChunkScheduler` heeft drie tiers: `Full`, `Half` (2×2×2 downsample), `Imposter` (1 flat quad) [B uit code].
- `fS3V` [B]: claimt drie HLOD-tiers en 24-chunk renderafstand op geïntegreerde GPU.
- `vqWz` [V]: toont volumetrische wolken, geen terrein-LOD.

**Advies:** onze 3-tier LOD is vergelijkbaar met de geclaimde HLOD-aanpak. Mogelijke verbetering: skirts/geomorph om naadloze overgangen te garanderen (0fps-artikel). Dit is al eerder geïdentificeerd als B1-verbetering.

### 1.4 Streaming en scheduling
- Onze `ChunkScheduler`: close-first prioriteit, bounded worker-pool, frustum-first, height-cache, air-skip [B uit code].
- `fS3V` [B]: 24-chunk render distance, geen details over scheduling.

**Advies:** onze aanpak is al state-of-the-art voor dit schaalniveau (~1350 chunks). GPU indirect/occlusion (D1/D2) pas zinvol boven ~100k chunks.

### 1.5 Physics
- Onze `voxel-player`: voxel collision via `world.material_at`, AABB-achtige stapresolutie, step-up [B uit code].
- `fS3V` [B]: claimt AABB-collision + hybride cellular automata voor water/lava/zand/gravel.
- `PJEm` [B]: toont Jolt narrow-phase “hijack” voor voxel physics; waarschuwt dat voxel contact generation O(N³) is en vaak niet opweegt tegen capsule primitives. Adviseert hybride aanpak.

**Advies:**
- Behoud voxel collision voor de avatar.
- Voeg **geen** volledige voxel rigid-body physics toe zonder bewijs; volg `PJEm`: hybride primitieve colliders + voxel terrain is vaak 6× goedkoper.
- Cellulaire automata voor fluids/granular (zand/water) is een waardevolle, beperkte uitbreiding; klein experiment aanbevolen.

### 1.6 Lighting
- Onze engine: vertex-AO + dag/nacht directional light [B uit code].
- `fS3V` [B]: claimt BFS-lighting (waarschijnlijk flood-fill zonlichtpropagatie, zoals Minecraft).

**Advies:** BFS/zonglift-propagation is een natuurlijke volgende stap voor correcte schaduw in grotten en onder overhangen. Klein experiment aanbevolen na client-extractie.

### 1.7 Volumetric clouds / weather
- `vqWz` [V/B]: raytraced voxel-cloudvolumes, volumetric horizon scatter, weather events, sneeuw-biota-verandering, jitter op ~1/4-resolutie.
- Onze engine: geen weather/clouds-stack.

**Advies:** dit is puur filmische “juice” en past bij onze noordster (GTA VI / Crimson Desert sfeer). Niet nodig voor vertical slice, maar hoogwaardige ROI voor visuele aantrekkingskracht. Apart experiment na basis-client.

---

## 2. Technieken die we NIET moeten overnemen zonder bewijs

- **Volledige voxel rigid-body physics** (`PJEm` waarschuwt expliciet voor O(N³)-kosten).
- **Blindelings overstappen op adaptive grids** puur op basis van de 10-seconden Blender-demo `P5M`.
- **Rewrite naar C++/OpenGL** zoals `fS3V`: onze Rust/wgpu-stack is al correct en portabel; de winst zit in algoritmen, niet in taal.

---

## 3. Adaptive voxel grids — apart onderwerp

De gebruiker vindt adaptive grids “super gaaf”. Technisch onderscheid:

1. **Artistieke adaptive discretisatie** (Blender Geometry Nodes, `P5M`): mooi effect, geen runtime-engine.
2. **Runtime adaptive grids** met echte voordelen:
   - Sparse Voxel Octree (SVO) / DAG: minder geheugen voor sparse werelden, ray traversal versnelling.
   - AMR (Adaptive Mesh Refinement): fijne cellen waar nodig, grove elders.
   - Clipmaps: vaste grootte window rond speler, ideaal voor gigantische werelden.
   - Crack-free LOD: Transvoxel of skirts om naadloze overgangen te garanderen.

**Voor onze engine:**
- Bij homogene 12.5 cm-terrein is een SVO/DAG pas zinvol als werelden echt groot en sparsam worden (>>100k chunks).
- Een **praktische tussenstap**: adaptive resolution rond de speler (fijn binnen X meter, grover daarbuiten), gekoppeld aan onze bestaande LOD-tiers. Dit geeft “adaptive grid”-gevoel zonder volledige datastructuur-rewrite.
- **Experiment aanbevolen:** een adaptive-resolution LOD-laag bovenop `ChunkScheduler`, waarbij `Half`/`Imposter`-radii dynamisch schalen met camera-snelheid en gezichtssnelheid. Meetbare tracer: chunk-geheugen en frame-time bij constante view radius.

---

## 4. Openstaande verificatie

De volgende claims moeten nog aan primaire bronnen worden gekoppeld voordat ze “accepted” worden:
- `fS3V` repo/source + volledige feature-claim + benchmarkmethodologie.
- `PJEm` broncode Jolt-hijack + exacte performance-cijfers.
- `vqWz` repo/paper voor voxel-cloud-raymarching细节.
- `P5M` bevestigen dat het Blender-demo is (metadata wijst daar sterk op).
- De acht kanalen (dphfox, DouglasDwyer, xima1, IGoByLotsOfNames, MishMash95, EDBev, DeadlockCode, frozein) zijn nog niet geanalyseerd wegens creditblokkade.

---

## 5. Conclusie (voorlopig)

Onze huidige architectuur is **niet inferieur** aan wat in deze video’s zichtbaar is. De meeste geclaimde technieken (greedy meshing, 3-tier LOD, scheduling, AABB-collision, BFS-lighting) hebben we al of staan gepland. De unieke, waardevolle uitbreidingen zijn:
1. Cellulaire automata voor fluids/granular (klein experiment).
2. BFS/zonglift-lighting voor grotten (klein experiment).
3. Adaptive-resolution LOD-laag als praktische “adaptive grid” (middelgroot experiment).
4. Volumetrische wolken/weather voor filmische juice (aparte fase).

Een volledige rewrite naar adaptive grids/octrees is **niet** gerechtvaardigd zonder schaalbewijs.
