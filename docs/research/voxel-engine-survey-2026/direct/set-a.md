# Set A — Transcript- & techniekonderzoek (videos + kanalen)

**Datum onderzoek:** 2026-07-15
**Onderzoeker:** gratis OpenRouter-model (onafhankelijke research-agent)
**Scope:** Lane B — videoset A: `video-LxVL`, `video-2dxX`, `video-QFQk`, `video-CJ94`, `video-fS3V` (visueel) + kanalen `dphfox`, `DouglasDwyer`, `xima1`, `IGoByLotsOfNames`.
**Statusregel:** `[B]` bewezen uit transcript/metadata, `[V]` visueel waargenomen, `[O]` onzeker/specultief, `[K]` vakkennis/secondary, `[L]` live-webverificatie.

> Compact technisch dossier. Geen 30k-woordvereiste (expliciet vrijgesteld in opdracht). Vergelijking met actuele Rust/wgpu-codebase via `crates/*` (PROJECT_STATE 2026-07-15, S-01…S-13b + mijlpalen 1–4 + Fase-2 cache-spike + 27i/27l/27m/27n).

---

## 0. Samenvatting codebase-stand (voor vergelijking)

| Subsysteem | Onze status (geverifieerd uit code) |
|---|---|
| Voxelresolutie | `VOXEL_SIZE_M = 0.125` (12,5 cm), `CHUNK_SIZE = 32` → chunk = 4 m (S-13, ADR-0005) |
| Datastructuur | `voxel-core`: 3 chunk-states `Uniform`/`PalettePacked`/`Dense`, 4-bit bitpacking, per-chunk palette (≤16), byte-stabiel VWL1 (S-01hardening) |
| Meshing | `voxel-mesher`: naive→culled→greedy; waterdicht; geen renderer-dep (S-02) |
| Worldgen | `voxel-worldgen`: seeded fBm heightmap + 3-tier biomes (Region→Biome→LocalParams), per-kolom height-buffer cache (S-04, 27d, 27l, 27n) |
| Streaming/scheduling | `voxel-gpu::chunk_stream`: radiaal disc, close-first prioriteit, 3-tier LOD (`Full`/`Half` 2×2×2/`Imposter` 1 quad), `requests_per_frame`-budget, air-skip, height-cache, Y-lagen 0..=max_cy (27l, 27m) |
| Worker-architectuur | `voxel-gpu`: dedicated `rayon::ThreadPool` (1 core vrij voor render) → `generate_chunk`+`greedy_mesh` off-thread; `crossbeam_channel` stuurt kant-en-klare `Vec<Triangle>`+`generation` terug; per-frame `UPLOAD_BUDGET` (4→64), gen-counter stale-discard (Mijlpaal 3 / 27, 27m) |
| GPU | `voxel-gpu`: wgpu 0.30/Vulkan, WGSL per-normaal + fog + triplanar PBR texture-array (4K, Mijlpaal 4), frustum-culling per chunk + buffer-pooling (Mijlpaal 2), `LruMeshCache` (27e) |
| Physics | `voxel-player`: avatar 1,90 m, axis-separated voxel collision, step-up, sub-stepping (27j) |
| Lighting | vertex-AO + dag/nacht directional light (code); BFS/zonglift nog niet geïmplementeerd |
| Client | `voxel-client` crate bestaat nu (extractie voltooid); `gpu_window` first-person live client op 12,5 cm |

---

## 1. video-LxVL — "Incredible voxel mesh optimisations!" (Stockhome/Daydream)

- **Auteur/kanaal:** onbekend bij naam in transcript; project heet "Stockhome", voorgestelde naam "Daydream" (0:09–0:35). YouTube-metadata kanaal = *Technical Fluff* (geverifieerd via search-snippet: "Technical Fluff article on greedy meshing", link fluff.blog). `[L][O]`
- **Duur:** 19:30. **Publicatie:** niet in transcript; via YouTube ~2023–2024 (onbekend exact, `[O]`).
- **Hardware:** niet genoemd. **Licentie:** geen repo genoemd (geen primaire broncode gevonden, `[O]`).

### Kernclaims + tijdcodes
- 0:44–1:18 **Overperforming-bug**: chunk-gen zo geoptimaliseerd dat duizenden chunks per frame gegenereerd worden → per-frame GPU-upload wordt de bottleneck (frame-rate daalt). Twee fixes: (1) code optimaliseren zodat méér chunks/frame verzonden worden, (2) zoveel mogelijk chunks *rejecten* (niet renderen).
- 1:20–4:36 **Greedy meshing** uitgelegd (flat surface → één merged face). Verwijst naar 0fps-artikel (2:52).
- 4:36–9:17 **Binary meshing** (kernclaim): chunk als binaire face-stack per richting (6 richtingen); "binary face stack" per chunk (6:42); combineer opeenvolgende flush faces in een richting tot één grote face → greedy meshing versnelt. "just with that alone [speedup]" (7:16).
- 9:55–10:53 **Upload-budget / reject**: alleen meshes/texture-data voor chunks die we écht tekenen; bouw een "rejection process". Lege chunks (geen zichtbare blocks) → geen mesh/draw call.
- 10:53–13:21 **Inter-chunk occlusion (kern van de video)**: gebruik de 6 chunk-gezichtsvlakken; "mutual visibility" — als je een chunk binnenkomt via face A en kunt ontsnappen via face B, zijn A en B "mutually visible"; anders blokkeert iets → chunk onzichtbaar vanaf die kant. Cache deze 6×6 visibiliteitsmatrix per chunk, herbruik zolang chunk niet verandert (12:58–13:13). "step through the world chunk by chunk in the general direction we're facing" (13:13) → chunks erachter worden nooit overwogen.
- 13:37–14:20 dis-occlusion bij veranderende chunks (even wachten met dis-occluden).
- 17:40–18:33 **Approximatieve zichtbaarheid via "heat/path"**: Trey's algoritme — threshold waarde op 0,5 benadert directe zichtbaarheid (goedkoop, "fudge this value"). Niet geïmplementeerd in deze video (18:21 "will have to wait for another day").
- 18:36–18:52 Resultaat: **>30% hogere FPS**, stutters weg, "infinite loading" mogelijk.

### Techniekclusters
1. **Meshing-snelheid**: binary meshing (bitwise face-stack) boven naive greedy.
2. **Upload/back-pressure**: reject lege chunks, budget per frame.
3. **Inter-chunk occlusion-culling**: 6×6 chunk-face visibiliteitsgraph, chunk-by-chunk "ray"/walk in kijkrichting.
4. **Approximatieve globale zichtbaarheid** (pad-heat, niet geïmplementeerd).

### Codebase-gap
- **Binary meshing**: wij doen greedy (culled→greedy) op `voxel-mesher`. Geen bitwise face-stack. Mogelijke winst bij 12,5 cm-resolutie (veel meer faces dan 1 m-voxels).
- **Inter-chunk occlusion**: wij hebben *frustum-culling per chunk* (Mijlpaal 2) + air-skip, maar **geen** chunk-chunk occlusiegraph. Bij 12,5 cm en view-radius 48 (~192 m, 150 km² adresseerbaar) is occlusie achter heuvels/ondergrond de volgende grote reductie.
- **Per-frame upload-budget**: wij hebben `UPLOAD_BUDGET` (4→64) + gen-counter; vergelijkbaar met hun "reject". Goed afgedekt.

### Aanbevolen experimenten
- **E1 (occlusie):** voeg chunk-chunk occlusie toe bovenop frustum: per chunk een 6-bit "welke buren zijn zichtbaar" + walk van camera-column in kijkrichting; meet chunk-count en GPU-tris bij view-radius 48. Tracer: `occlusion_cull_reduces_visible_chunks_by_X_pct`.
- **E2 (binary meshing):** spike `spike_binary_mesh.rs`: bitwise face-stack vs huidige greedy; meet ms/chunk bij 12,5 cm. Alleen overnemen bij >10% winst op release-build (PROJECT_STATE 27m toont greedy al dominant).

### Verificatie / tegenbewijs
- 0fps greedy-meshing primaire bron bestaat (0fps.net/2012/06/30/meshing-in-a-minecraft-game, `[L]`).
- "Binary meshing"-term is hier de auteursnaam voor bitwise face-fit; verwant aan Laine & Karras / 0fps "Meshing in a Minecraft Game" discussie over chunk-as-cuboid. Geen paper gevonden met exact deze "mutual visibility 6×6" — dat is de auteur zijn eigen heuristiek (`[O]`, niet in primaire literatuur).
- Tegenbewijs: 30% FPS-winst is anekdotisch, geen hardware/methode genoemd; onze eigen 8,8→15,8→~93 FPS-gains kwamen uit frustum+budget, niet occlusie — occlusie-effect bij ons nog onbewezen.

---

## 2. video-2dxX — multithreading / raycast (voxel devlog)

- **Auteur/kanaal:** onbekend bij naam in transcript (geen titel/channelexpliciet in tekst). Onderwerp: multi-threading van meshing, raycast block-picking, action-thread. `[O]`
- **Duur:** 14:36.

### Kernclaims + tijdcodes
- 0:39–1:27 **Threading-caveat**: als één thread schrijft terwijl een ander leest → race. Oplossing: mutex/lock zodat "only one thread is allowed to [write]" (1:23) — andere thread wacht.
- 1:06 **Meshing op aparte thread** (chunk-meshing off-main).
- 5:23–9:12 **Raycast block-picking**: "collide with all four block faces" (5:23); raycast door chunks (7:40 "raycasts through the chunks"); bepaal welke block-face de speler aanwijst (8:03); crosshair (8:10). Highlights block door de grond heen (9:12).
- 13:13–14:20 **Action-thread**: speleracties naar aparte thread; main-thread blijft vrij voor update/render. "multi-threading the physics, the graphics, the meshing everything" (14:18).

### Techniekclusters
1. **Main-thread bescherming** via lock (eenvoudig mutex-model).
2. **Off-thread meshing + raycast block-selection.**
3. **Gedecouple action-thread** (input/acties los van render).

### Codebase-gap
- **Raycast block-picking**: wij hebben *geen* raycast block-selection in de live client (wel `material_at` voor collision). Nodig voor edit-tool UI (place/remove is in `voxel-edit`, maar geen pick-ray in `gpu_window`).
- **Threading-model**: onze `rayon::ThreadPool` + `crossbeam_channel` is *modernere* variant dan hun mutex-wacht-model; onze gen-counter stale-discard (27) voorkomt exact de race die zij met locks oplossen. **Onze aanpak is superieur/equivalent** — geen overname nodig.
- **Action-thread**: onze input loopt in de winit-event-loop (main); geen aparte action-thread. Voor de slice prima.

### Aanbevolen experimenten
- **E3 (raycast pick):** voeg DDA voxel-raycast toe in `voxel-gpu`/`voxel-client` voor block-selection (feed naar `voxel-edit::EditTool`). Tracer: `raycast_pick_returns_correct_block_in_world_meters`.

---

## 3. video-QFQk — "Rayon is NOT for games - use this instead [Voxel Devlog #27]"

- **Auteur/kanaal:** onbekend bij naam; Rust-engine (noemt Rayon expliciet, 0:07). `[L]` search bevestigt titel + "replacing rayon with a custom lock-free thread pool".
- **Duur:** 14:16. **Hardware:** "five worker threads" (5:01) → 6-core logisch. **Licentie:** geen repo in transcript (`[O]`).

### Kernclaims + tijdcodes
- 0:07–1:19 **Rayon-uitleg**: parallel iterators, hoofdthread plant werk op worker-queue, workers pakken taken. "super simple" (1:07).
- 1:34–3:48 **Probleem met Rayon voor games**: als main-thread parallel werk plant en workers druk zijn met worldgen, dan *wacht* de main-thread tot werk begint (3:25–3:40) → frame hitch. OS swapt main-thread weg (3:37).
- 4:15–6:12 **Thread-timeline-diagram**: workers + main; als alles tegelijk wil draaien, OS kan niet alle threads schedulen (6:06) → idle/stalletjes.
- 6:58–8:32 **Lock-free scheduler claim**: eigen scheduler "lock-free" (8:27); "Multiple threads can spawn new tasks" (8:32); geen lock-contention.
- 9:16–11:27 **Worker-stealing / work-unit**: elke work-unit door exact één thread (9:38); worker weet wanneer klaar (10:59); main weet wanneer parallel iterator compleet is (11:27).
- 13:19–13:52 **Conclusie**: door Rayon te vervangen door eigen thread pool + main-thread participatie ("big green thread helping out", 13:40) → main-thread zit niet idle, wereldgen draait op alle cores.

### Techniekclusters
1. **Rayon-nadeel voor realtime**: main-thread blocking wachten op worker-werk → frame stalls.
2. **Custom lock-free thread pool** met work-stealing, main-thread neemt ook taken op.

### Codebase-gap — **DIRECT RELEVANT**
- Wij gebruiken **precies Rayon** (`rayon::ThreadPool`, 1 core vrij voor render, Mijlpaal 3 / 27). De video waarschuwt dat Rayon's model de main/render-thread kan laten *blocken* wanneer workers vol zitten — precies het risico dat onze `UPLOAD_BUDGET`+gen-counter probeert te mitigeren, maar onze render-thread roept `render_frame` aan en vult `mesh_cache` uit de channel *binnen budget* — hij blokkeert niet op gen/mesh (goed). Echter: bij `generate_chunk`+`greedy_mesh` op de pool geldt Rayon's "main wacht niet" alleen als we de channel non-blocking lezen (wij doen dat, 27).
- **Risico:** onze pool laat 1 render-core vrij, maar als alle workers + render druk zijn met zware fBm-gen (PROJECT_STATE 27i: 3,185→0,226 ms/chunk na fix; nog steeds pieken bij eerste Y-slab), kan de render-thread onderbroken worden door OS scheduling zoals de video beschrijft.
- **Mitigatie al aanwezig:** dedicated pool (niet de globale Rayon), 1 core vrij, channel-based pull, gen-counter stale-discard. Dit is *beter* dan naive Rayon-gebruik, maar de video's kernpunt (eigen lock-free pool met main-participatie) is een legitieme volgende optimalisatie.

### Aanbevolen experimenten
- **E4 (thread-pool eval):** meet render-frame-time-jitter vóór/na verhoging van worker-count (bv. pool met `num_cpus-1` vs `-2`). Als p99 frame-time stijgt door OS-scheduling (zoals QFQk claimt), overweeg custom pool mét main-participatie. Tracer: `render_p99_frame_time_stable_under_chunk_gen_load`.
- **E5:** verifieer dat `render_frame` nooit blockt op de channel (non-blocking `try_recv`); bestaande test `mesh_chunk_offthread_streams_result` dekt het principe.

### Verificatie / tegenbewijs
- Rayon lock-free work-stealing is gedocumenteerd feit (`[K]`); de "main-thread wacht" claim is specifiek voor *nested* parallel iterators op de main-thread, niet voor onze channel-pull. Dus QFQk's probleem treft ons deels maar niet volledig. `[O]` voor de "30% winst"-implicatie (geen cijfer in transcript).

---

## 4. video-CJ94 — "My game is 262000 times faster than Minecraft" (IGoByLotsOfNames)

- **Auteur/kanaal:** **IGoByLotsOfNames** (bevestigd `[L]`: playlist "igobylotsofnames voxel game engine development", 472K subs, Unity-engine). Video = CJ94gOzKqsM.
- **Duur:** 12:20. **Engine:** Unity (C#) + Shader Graph (10:20). **Licentie:** geen open repo (`[O]`).

### Kernclaims + tijdcodes
- 0:50–1:28 **Mesh = alleen gezichtsvlakken die niet door solid blocks bedekt zijn**; per-face texture-coördinaat. (Standaard culled meshing.)
- 1:28–2:09 **Frustum culling** op face-niveau + verder opsplitsen mesh in delen (1:33). Vertex-optimalisatie: "default vertex format could be eliminated by customizing" (1:44–1:57). "hundreds of chunks without [issue]" (2:05).
- 2:09–2:29 **Cubic chunks** (≠ Minecraft's 16×16×Y): cubic chunks, spawn rond camera (2:22).
- 2:29–3:49 **Chunk-gen traag** → "infinite loading" (3:37). Gen op main-thread → freeze (5:20–5:30).
- 4:06–4:33 **Multi-res LOD**: verre chunks "one block per 8 cubic m", nog verder "progressively lower resolution", uiteindelijk "single block per chunk" (4:22). Lage-detail chunks gecombineerd tot minder objecten.
- 4:33–5:30 **Greedy meshing** toegevoegd (5:02); alle gen op main-thread → freeze (5:20).
- 5:30–5:51 **Off-thread chunk-gen** (5:35 "banished to a different thread"), main-thread niet meer geblokkeerd.
- 5:51–6:20 **Perlin noise**, meer octaves = "two to the power of the perlin" (6:17–6:20) exponentiële detailtoename.
- 9:17–9:56 **Ambient Occlusion** toegevoegd (9:39).
- 9:50–10:55 **Volumetric/god-ray lighting** via "50-step tutorial shortcut": witte afbeelding geoccludeerd door zichtbare objecten + radial blur (10:01–10:08), in pipeline gevoed. **Shaders via Shader Graph** (10:20). Realistisch water: refractie ondiep / reflectie diep (10:44–10:49).
- 11:03–11:30 shader update voor sky e.d.

### Techniekclusters
1. **Culled + greedy meshing**, face-frustum-culling, aangepaste vertex-format.
2. **Cubic chunks** (volledig 3D, niet columnair).
3. **Multi-resolution LOD** (detail daalt met afstand, tot 1 block/chunk).
4. **Off-thread worldgen** (zelfde les als onze Mijlpaal 3).
5. **Perlin fBm** (octave-stacking).
6. **AO + goedkope god-rays (occlusion+sprite-blur) + Shader-Graph water/sky.**

### Codebase-gap
- **Cubic chunks**: wij gebruiken columnaire Y-slabs (cy 0..=max_cy) met `CHUNK_SIZE=32³`. CJ94's cubic-chunk-model is flexibeler voor verticale LOD maar complexer; onze Y-slab-streaming (27l) is vergelijkbaar in geest. Geen overname tenzij verticale LOD nodig is.
- **Multi-res LOD**: wij hebben 3-tier (`Full`/`Half`/`Imposter`) — vergelijkbaar met hun "8 m³ → 1 block/chunk". Onze `Half`=2×2×2 (8× minder tris), zij "8 cubic m" per block. **Vergelijkbaar niveau.**
- **Off-thread gen**: wij hebben dit (rayon + channel). Goed afgedekt.
- **God-rays**: wij hebben directional + fog, geen god-rays. CJ94's "occlusion sprite + radial blur" is goedkoop en filmisch — past bij onze noordster.
- **AO**: wij hebben vertex-AO (code). Goed.
- **262.000× sneller dan Minecraft**: claim is marketing (`[O]`); geen methodologie. Onze eigen bench (93 FPS @ 1 km² r48, 27m) is de enige harde maatstaf die telt.

### Aanbevolen experimenten
- **E6 (god-rays):** goedkope occlusion-based god-rays (radial blur van depth/occlusion) in WGSL na het terrain-pass; tracer: `godrays_under_1ms_at_1080p`.
- **E7 (verticale LOD):** evalueer cubic-chunk vs Y-slab voor verticale detail-afname bij hoge gebouwen/ondergrond; alleen zinvol boven view-radius 48.

---

## 5. video-fS3V — "I Built a C++ Micro Voxel Engine using AI" (saladmander) — VISUEEL

- **Auteur/kanaal:** **saladmander** (YouTube) = **Saladmander99** op GitHub (`[L]` description + tags). 
- **Duur:** 293 s (4:53). **Publicatie:** 2026-06-15. **Hardware (metadata):** Intel i5-1235U (1,30 GHz), 8 GB RAM, Intel UHD geïntegreerde GPU. **Transcript:** uitgeschakeld (geen tekst). **Repo:** `github.com/Saladmander99` (`[L]` uit description; exacte repo-pad niet geverifieerd — géén clone gedaan).
- **Visueel waargenomen (VISUAL_SOURCE_NOTES):** volledige procedurele voxelgame — terrein/biomes, bomen, third-person character, renderer-panelen, water/lava, grotten, zwemmen, inventory, character-animatie. "10–18 render distance", shadow on, godlight on, >60 FPS.

### Kernclaims (uit metadata/description, `[B]`)
- **Architectuur:** C++/OpenGL, custom **ECS + Data-Oriented Design**.
- **Rendering:** **8×8×8 micro-voxel grid** (let op: "micro" hier = 8³ sub-grid per block, niet per se 12,5 cm — exacte voxelgrootte onbekend, `[O]`), **greedy meshing**, **3-tier HLOD**, 24-chunk renderafstand op geïntegreerde GPU.
- **Physics:** **Hybride Cellular Automata** voor fluïde dichtheid + granulaire verplaatsing (zand, grind, lava, water) + AABB-collision.
- **Lighting:** **BFS Light Propagation** (zonglift-flood-fill à la Minecraft), real-time RGB-blending, volumetrische god-rays.
- **Assets:** allemaal procedureel extern gegenereerd (TextureDesigner/TreeDesigner/CreatureDesigner), geladen als tekst — "Zero PNGs".

### Techniekclusters
1. **Micro-voxel + DOD/ECS** (C++, OpenGL).
2. **Greedy + 3-tier HLOD** (vergelijkbaar met onze greedy + 3-tier LOD).
3. **Cellulaire automata** voor fluids/granular — **uniek ten opzichte van onze codebase**.
4. **BFS zonglift-propagation** — wij hebben dit nog niet.
5. **Procedurele asset-pipeline (tekst, geen PNG)** — verwant aan onze texture-array/PBR maar ander genereermodel.

### Codebase-gap
- **Micro-voxelgrootte:** onze 12,5 cm is fijner dan hun 8³-grid claim (waarschijnlijk grover). Behoud 12,5 cm (advies CODEBASE_COMPARISON_DRAFT §1.1).
- **Cellulaire automata:** **Gap.** Wij hebben geen fluids/granular. ECS/DOD is C++-specifiek; onze `voxel-edit::EditLog` is de dichtstbijzijnde "edit propagation"-primitive.
- **BFS-lighting:** **Gap.** Onze vertex-AO dekt hoek-darkening, maar geen cave/overhang-schaduw. BFS zonglift is natuurlijke volgende stap (CODEBASE_COMPARISON_DRAFT §1.6).
- **HLOD/3-tier:** vergelijkbaar met onze `Full`/`Half`/`Imposter`.

### Aanbevolen experimenten
- **E8 (BFS zonglift):** spike `spike_bfs_light.rs` in `voxel-worldgen` of aparte `voxel-light`: flood-fill vanaf lucht naar binnen, propagatie-dimming; koppel aan WGSL als vertex-lighting/extra AO. Tracer: `bfs_light_produces_cave_shadow`.
- **E9 (cellulaire automata):** klein experiment: water/lava/zand als `EditLog`-regels op een `voxel-fluid`-spike; meet ms/update op view-radius 24. Alleen behouden bij <0,5 ms/chunk.

### Verificatie / tegenbewijs
- Repo `github.com/Saladmander99` genoemd maar **niet geverifieerd op inhoud/licentie** (geen clone, taakverbod op codewijziging geldt niet voor lezen maar ik heb geen repo-inhoud opgehaald). `[O]` voor exacte implementatie/licentie.
- ">60 FPS op i5-1235U + Intel UHD" is indrukwekkend maar: 8³ micro-grid + 24-chunk afstand is veel kleinschaliger dan onze 12,5 cm @ view-radius 48; geen directe vergelijking mogelijk zonder hun repo + bench-methode.
- AI-claim ("AI as architectural co-pilot") is geen technische claim; negeren voor vergelijking.

---

## 6. Kanalen (cross-reference voor Lane B)

| Kanaal | Identiteit (verifieerd) | Relevantie voor onze codebase |
|---|---|---|
| **dphfox** (Daniel P H Fox) | "Building a cosy survival voxel game", 5K subs, 30 video's; Rust voxel game (first-person controls, "Fusion Dev Stream"), voxel physics engine met rigid-body/materials (`[L]`). | Rust-ecosysteem (verwant aan onze Rust/wgpu-stack). **Voxel rigid-body physics** → zie PJEm-waarschuwing (O(N³)); onze `voxel-player` houdt avatar-collision, geen full rigid-body. |
| **DouglasDwyer** | "massively-detailed voxel engine"; GitHub `DouglasDwyer/voxel_engine` (Octo engine public modding API) + `octo-release` (rigidbody physics, connected component detection, collision, response, multiplayer) (`[L]`). | **Octree/structuur** + GPU-versnelde voxel rigid-body (TGS solver, "shatter on impact", "tripled render distance"). Octree is interessant voor onze sparse/scale-fase (Fase 5 LOD/clipmap). Rigid-body blijft PJEm-waarschuwing. |
| **xima1** (xima / _x1m4) | **GPU-driven, shader-based voxel engine** (WebGPU), global illumination / path-traced, transparency, underwater, inventory/items (`[L]` YouTube + X). | **GPU-driven rendering** = onze toekomstige richting (indirect/bindless, Fase 5). Hun path-traced GI is verder dan onze WGSL-PBR; relevant als referentie voor latere lighting-upgrade. Geen repo gevonden (wel X-account). |
| **IGoByLotsOfNames** | Unity voxel-engine-dev (CJ94 hierboven), 472K subs, "262k× faster than Minecraft", raytracing-video's (`[L]`). | Unity/Shader-Graph stack; hun LOD/greedy/occlusion-lessons (CJ94) zijn engine-agnostisch bruikbaar (zie §4). Raytracing-video (bi DJB) relevant voor latere Fase. |

---

## 7. Codebase-gap matrix (set A → onze crates)

| Techniek (bron) | Onze crate/status | Gap | Oordeel |
|---|---|---|---|
| Greedy meshing (LxVL, CJ94, fS3V) | `voxel-mesher` (S-02) | geen | **BEHOUD** |
| Binary/bitwise meshing (LxVL) | `voxel-mesher` greedy | mist | UPDATE (E2 spike, pas bij >10% winst) |
| Inter-chunk occlusie (LxVL) | `chunk_stream` frustum-only | mist | **UPDATE** (E1, hoogste ROI bij 12,5 cm) |
| Per-frame upload-budget/reject (LxVL) | `UPLOAD_BUDGET`+gen-counter | geen | **BEHOUD** |
| Off-thread gen (2dxX, CJ94, fS3V) | rayon pool + channel | geen | **BEHOUD** |
| Custom lock-free pool vs Rayon (QFQk) | rayon pool | deels | **EVAL** (E4, pas bij p99-jitter) |
| Raycast block-pick (2dxX) | geen in client | mist | UPDATE (E3) |
| Cubic chunks (CJ94) | Y-slab columnair | anders | REJECT (voor nu; Fase 5) |
| Multi-res LOD (CJ94, fS3V) | 3-tier LOD | vergelijkbaar | **BEHOUD** |
| AO (CJ94) | vertex-AO | geen | **BEHOUD** |
| God-rays (CJ94, fS3V) | directional+fog | mist | UPDATE (E6, filmisch) |
| BFS zonglift (fS3V) | geen | mist | **UPDATE** (E8, caves) |
| Cellulaire automata fluids (fS3V) | geen | mist | UPDATE (E9, klein) |
| GPU-driven/indirect (xima1) | mesh-CG draw | mist | Fase 5 (REJECT nu) |
| Octree (DouglasDwyer) | grid | mist | Fase 5 (REJECT nu) |
| Voxel rigid-body (dphfox/Douglas) | avatar-collision | mist | REJECT (PJEm O(N³)-waarschuwing) |

---

## 8. Prioriteit experimenten (meetbaar, strict TDD)

1. **E1 — Inter-chunk occlusie** (LxVL): hoogste verwachte ROI bij 12,5 cm + view-radius 48. Tracer: `occlusion_cull_reduces_visible_chunks_by_X_pct` bij autopilot.
2. **E8 — BFS zonglift-lighting** (fS3V): natuurlijk, goedkoop, covey cave-schaduw. Tracer: `bfs_light_produces_cave_shadow`.
3. **E3 — Raycast block-pick** (2dxX): vereist voor edit-tool UI. Tracer: `raycast_pick_returns_correct_block_in_world_meters`.
4. **E6 — God-rays** (CJ94/fS3V): filmische juice, past noordster. Tracer: `godrays_under_1ms`.
5. **E9 — Cellulaire automata** (fS3V): fluids/granular, beperkte scope. Tracer: `fluid_step_under_0_5ms_per_chunk`.
6. **E2 — Binary meshing** (LxVL): pas bij >10% ms/chunk-win op release.
7. **E4 — Thread-pool eval** (QFQk): pas bij gemeten p99-jitter.

> Geen enkele aanbeveling vereist een rewrite naar C++/OpenGL (fS3V), octree (DouglasDwyer) of cubic chunks (CJ94). Onze Rust/wgpu-stack is architecturaal niet inferieur; winst zit in *algoritmen* (occlusie, BFS-light, automata), niet in taal/datastructuur.

---

## 9. Bronnenlijst

**Directe video's (transcripten lokaal):**
- `transcripts/video-LxVL.txt` (LxVLqCiDqd8, 19:30) — Technical Fluff / Stockhome / Daydream
- `transcripts/video-2dxX.txt` (2dxX755WgGk, 14:36) — voxel devlog, multithreading+raycast
- `transcripts/video-QFQk.txt` (QFQkqFSg8Z4, 14:16) — "Rayon is NOT for games"
- `transcripts/video-CJ94.txt` (CJ94gOzKqsM, 12:20) — IGoByLotsOfNames / Unity
- `media/fS3VVlx49ao.info.json` + `VISUAL_SOURCE_NOTES.md` — saladmander / Saladmander99 (C++, transcript uit)

**Kanalen (live geverifieerd):**
- youtube.com/@dphfox — Daniel P H Fox (Rust voxel + rigid-body physics)
- youtube.com/@DouglasDwyer — Octo voxel engine; github.com/DouglasDwyer/voxel_engine + octo-release
- youtube.com/@xima1 — xima / _x1m4 (GPU-driven WebGPU voxel, path-traced GI)
- youtube.com/@IGoByLotsOfNames — Unity voxel engine dev (CJ94)

**Primaire/secundaire technische bronnen:**
- 0fps, "Meshing in a Minecraft Game" — https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/ (greedy meshing, `[L]`)
- Technical Fluff, "binary meshing" artikel (fluff.blog, genoemd in LxVL-desc, `[L]` snippet, volledig artikel niet opgehaald → `[O]`)
- Rayon docs (lock-free work-stealing) — `[K]`
- Laine & Karras, "Efficient Sparse Voxel Octrees" / SVDAG — `[K]` (context voor octree bij DouglasDwyer/Fase 5)
- Transvoxel (crack-free LOD) — `[K]` (niet in set A video's, wel relevante cluster)

**Niet geverifieerd:**
- `github.com/Saladmander99` exacte repo-pad/licentie (wel genoemd in fS3V-description) → clone + licentiecheck aanbevolen vóór "accepted".
- Exacte publicatiedata van LxVL/2dxX/QFQk (niet in transcript).
- "262.000× sneller dan Minecraft" (CJ94) en ">60 FPS i5-1235U" (fS3V) zijn marketing/anecdotisch, geen methodologie.

**Codebase-referenties (voor vergelijking):**
- `crates/voxel-core`, `voxel-mesher`, `voxel-worldgen`, `voxel-gpu/src/chunk_stream.rs`, `voxel-gpu/src/renderer.rs`, `voxel-player`, `voxel-edit` (alle S-01…S-13b + mijlpalen per PROJECT_STATE 2026-07-15).
- `.hermes/PROJECT_STATE.md` (status 2026-07-15).
- `direct/CODEBASE_COMPARISON_DRAFT.md`, `direct/VISUAL_SOURCE_NOTES.md` (bestaande draft-notities).
