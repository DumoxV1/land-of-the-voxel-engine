# Set B — Transcript- & techniekonderzoek (video's + kanalen)

** scope:** video-vqWz, video-PJEm, video-ztkh, video-tDTB + kanalen @MishMash95, @EDBev-b5g, @DeadlockCode, @frozein
**Datum:** 2026-07-15 · **Model:** openrouter/free (gratis researcher) · **Status:** analyse voltooid, wacht op onafhankelijke bronreview
**Methode:** lokale transcripts gelezen (transcripts/), video-auteurs geverifieerd via YouTube oEmbed + pagina-extractie, repo's/licenties live gecontroleerd via GitHub API/raw, codebase grondig gelezen in `crates/`.
**Regel:** geen code/Cargo/git gewijzigd. Alle claims zijn hypotheses tot primaire bron ze ondersteunt (zie statusvelden). Statuslegenda: `[B]` bewezen uit transcript/bron, `[V]` visueel/metadata, `[R]` live repo geverifieerd, `[K]` vakkennis, `[O]` onzeker.

---

## 0. Belangrijkste vaststelling

De 4 video's en 4 kanalen zijn **dezelfde 4 auteurs** (oEmbed + kanaal-URL's bevestigen 1:1):

| Video | Titel | Kanaal | Auteur | Repo (live geverifieerd) | Stack / licentie |
|---|---|---|---|---|---|
| vqWz | Raytracing Volumetric Clouds using Voxels (Devlog #5) | @MishMash95 | MishMash | geen publiek repo in video | micro-voxel engine (C++→Rust/Bevy per kanaal) |
| PJEm | One Change made my Voxel Physics engine 67x Faster | @EDBev-b5g | EDBev | geen repo getoond | Jolt-physics voxel-hijack, Teardown-stijl |
| ztkh | This Tiny Algorithm Can Render BILLIONS of Voxels in Real Time | @DeadlockCode | Deadlock | `DeadlockCode/voxel_ray_traversal` [R] | Rust + Vulkan compute, Apache |
| tDTB | Adding RAYTRACED LIGHTING to my Voxel Engine (Devlog 3) | @frozein | frozein | `frozein/DoonEngine` [R] | C + GLSL voxel path-tracer, MIT (© Daniel Elwell 2022) |

**Centrale bevinding:** drie van de vier (ztkh, tDTB, vqWz) zijn **ray-gebaseerde** renderers (raymarch / path-trace / volume-raymarch) — géén triangle-rasterisatie. Onze engine (`voxel-gpu`) is een **greedy-mesh → triangle → wgpu** rasterizer. Dat is de fundamentele architectuurdivergentie. Onze `PROJECT_STATE` markeert raytracing al als "latere fase (andere pipeline)". Conclusie vooraf: set-B is waardevol als **referentiedoel voor de latere filmische/raytracing-fase** en voor **specifieke technieken**, niet als vervanging van de huidige vertical slice.

---

## 1. video-ztkh — GPU voxel ray traversal (Deadlock)

### Kernclaims & tijdcodes `[B]` (transcript) + `[R]` (repo/description)
- 0:00 — doel: "render over 32 billion voxels in real time" zónder triangles/meshing, één ray per pixel op de GPU.
- 1:06–11:13 — opbouw Amanatides & Woo (1987) "A Fast Voxel Traversal Algorithm for Ray Tracing": Ray-AABB slab-methode voor entry, dan per-stap `t_x/t_y/t_z` plane-vergelijking, stap in richting van laagste `t`.
- 11:13 — render: Stanford Bunny (triangle mesh → gevoxeliseerd; voxelisatie + bare-bones Rust/Vulkan-impl in repo).
- 12:13–14:34 — **optimalisatie (kern):** 2048³ grid @1000×1000 → **12 FPS** baseline. (a) Z-order curve als flattening → **102 FPS** (8×); (b) voxels in een **3D-texture** (`3D image`) → **121 FPS** (10×). Algoritme ongewijzigd; winst puur uit geheugen-locality.
- 14:34 — teaser: volgende video = sparse voxel octrees + spaarzaam-traversaal.
- Hardware `[R]`: GPU Intel Arc A770, CPU AMD Ryzen 9 7900X. Gepubliceerd 2025-09-05, 172K views.
- Repo `[R]`: `github.com/DeadlockCode/voxel_ray_traversal` — Public, 232★, 24 forks, **100% Rust** (host + Vulkan compute shader), licence `LICENSE-APACHE` aanwezig.

### Techniekclusters
1. **Voxel ray traversal (DDA):** Amanatides-Woo, `tNext` incrementeel optellen van `|1/dir|`, géén herhaalde deling.
2. **GPU compute per-pixel ray:** primaire ray in compute shader; face-hit, hit-positie en afstand herberekenbaar voor shading/texture/secundaire rays.
3. **Memory-layout voor locality:** Z-order (Morton) curve en 3D-texture (hardware-sample locality). Dit is de eigenlijke "10× trick".
4. **Geen meshing, geen LOD, geen streaming:** de demo rendert één statische gevoxeliseerde bunny in een vast 2048³ grid.

### Codebase-gap (vs onze crates)
- `voxel-mesher` doet greedy meshing → `Triangle{pos,normal,material,ao}`; `voxel-gpu` rasteriseert die via wgpu. Geen ray-traversal-pad.
- Onze chunk-opslag in `voxel-core::chunk` is dense `CHUNK_SIZE³` flat array (rij-orde), geen Z-order/3D-texture. Greedy-meshing doet ~196k neighbour-probes/chunk (zie `27m`) — geheugen-locality telt dus ook voor óns.
- `voxel-worldgen` / `voxel-world` streamen en cachen (LruMeshCache); ztkh toont geen streaming/LOD.

### Aanbevolen experimenten
- **E1 (laag risico, hoog ROI):** evalueer Z-order/Morton of een echte 3D-texture als backing-store voor onze `Chunk`-data; meet greedy-mesh-throughput en worldgen-sample-latency (de locality-win is renderer-onafhankelijk). Tracer: `chunks/s` vóór/na (zie bestaande `27m`-benchmark ~3862 chunks/s).
- **E2 (late fase):** een GPU compute ray-traversal spike als *optioneel* alternatief render-pad naast rasterisatie, enkel zinvol boven ~100k chunks / voor filmische weergave. Niet voor vertical slice.

### Tegenbewijs / begrenzing
- "32 billion voxels real time" = één statische bunny in een 2048³ (8,6e9) grid; de 10× is pas ná 3D-texture. Geen open wereld, géén edits, géén streaming, géén LOD, géén physics. Niet 1:1 vergelijkbaar met een streaming game-engine.
- Arc A770 (Xe-HPG, 16 GB) is ruim; 121 FPS bij 1000² op die GPU zegt weinig over onze RTX 4080-load bij 1 km² streaming.

---

## 2. video-tDTB — raytraced lighting / GI (frozein)

### Kernclaims & tijdcodes `[B]` + `[R]`
- 0:13–2:00 — **hard shadows:** één shadow-ray per voxel richting zon; dedupe via hashmap met 64-bit voxel-ID + atomic boolean; elke voxel éénmaal in queue; finale full-screen pass donkert.
- 2:00–4:30 — **soft shadows:** per-voxel meerdere rays naar random punten op zon; `num_visible`/`num_shadowed` integers (atomic increment); **Fibonacci-sphere stratificatie** i.p.v. puur random → deterministisch, minder ruis.
- 4:30–7:00 — **global illumination (path tracing):** per zichtbaar punt ray in hemisfeer, bounces tot sky; `Vec3` indirect-light geaccumuleerd in hashmap; **temporal accumulation** (twee hashmaps, huidig+vorig frame, gemiddeld) om ruis te dempen; half-res, max 2 bounces; emissive voxels.
- Validatie: Cornell box (klassieke GI-test) — licht-bleeding zichtbaar, correct o.a. bij 2 bounces.
- Performance `[B]`: forest ~**70 FPS @1080p** op laptop **RTX 3050 Ti** (≈ desktop GTX 1060), GI half-res/2 bounces.
- Repo `[R]`: `github.com/frozein/DoonEngine` — Public, 197★, **C 88,9% + GLSL 10,6%**, "a voxel path-tracer" (per-voxel lighting, dynamic CPU→GPU streaming). Licentie **MIT** (© Daniel Elwell 2022).

### Techniekclusters
1. **Per-voxel lighting (niet per-pixel):** zichtbare voxels ontdubbeld via 64-bit ID-hashmap + atomics → geen dubbele shadow/GI-rays.
2. **Stratified sampling:** Fibonacci-sphere voor deterministische soft-shadow-ruisreductie.
3. **Temporal accumulation:** frame-overschrijdende sample-hergebruik voor GI (en scene-edit-dynamiek via sample-clamp).
4. **Hybride CPU→GPU voxel-streaming** in DoonEngine (dynamische edits).

### Codebase-gap
- Onze lighting (`voxel-gpu`/`renderer.rs`): vertex-AO (0fps-methode, bij mesh-tijd gebakken) + per-normal directional + fog (`time_of_day` dag/nacht) + PBR triplanar texture-array. **Geen** shadow-rays, **geen** GI, **geen** temporal accumulation.
- Geen path-trace/compute-pad; `voxel-render` (software z-buffer) en `voxel-gpu` zijn rasterizers.
- Onze `ChunkScheduler`/`chunk_stream.rs` dedupeert ook (per-chunk mesh-cache, geen per-voxel ID-hashmap) — ander paradigma.

### Aanbevolen experimenten
- **E3 (MVP-verbetering, al geïdentificeerd in CODEBASE_COMPARISON_DRAFT):** BFS/zonglift-propagation voor correcte grotten-schaduw (goedkoop, rasteriser-vriendelijk). Tracer: `material_layers_are_sane`-stijl test + capture-duisternis-onder-overhang.
- **E4 (late fase):** als ray-pad ooit komt, is frozein's 64-bit-ID-hashmap-dedupe + Fibonacci-stratificatie + temporal accumulation **direct herbruikbaar** als referentie-implementatie.
- **E5:** vergelijk onze vertex-AO+directional+kwaliteit versus frozein's per-voxel shadows bij gelijke scene — kwantificeer of de ray-kost (70 FPS @ GTX1060, half-res) opweegt tegen onze goedkope look bij RTX 4080.

### Tegenbewijs / begrenzing
- 70 FPS is op bescheiden GPU én met GI half-res/2 bounces + temporal lag (beeld "komt pas na een paar frames"). Bij volle res/meer bounces keldert het.
- Per-voxel lighting schaalt met zichtbare voxels → duur bij onze 12,5 cm-microvoxels (veel meer voxels dan bij grove grids). Onze rasterizer is voor dezelfde look nu véél goedkoper.

---

## 3. video-PJEm — voxel physics met Jolt (EDBev)

### Kernclaims & tijdcodes `[B]` (transcript)
- 0:09–0:40 — vorige video: voxel physics via "hijack" van Jolt's narrow-phase contact generator.
- 0:41–1:09 — **de 12× was letterlijk alleen release-build aanzetten** (Jolt veel sneller in release dan debug). 6→72 gestapelde ragdolls bij 30→…; "hundreds of physics objects at 100+ FPS in large maps".
- 1:21–1:37 — **kernwaarschuwing:** voxel contact-point generation is O(N³) in object-grid-grootte; voordeel moet opwegen tegen kost.
- 1:38–eind — capsule primitives: **400 gestapelde ragdolls** vs 72 bij voxel; "more than 6× performance cost" voor voxel physics; de meeste gameplay heeft die vrijheid niet nodig.

### Techniekclusters
1. **Hybride physics:** professionele multithreaded rigid-body lib (Jolt) + voxel-terrain als collider-bron; narrow-phase "hijack" genereert voxel-contacten.
2. **O(N³)-waarschuwing:** full voxel-voxel contact-gen is inherent traag; capsule/primitive colliders zijn 6×+ goedkoper.
3. **Release-build discipline:** debug vs release is ordegrootte-verschil (relevant voor ónze benchmarks!).

### Codebase-gap
- Onze `voxel-player`: custom AABB-avatar collision via `World::material_at` (axis-separated, sub-stepping, `resolve_floor_y`, step-up). **Geen** Jolt, **geen** rigid-body-sim, **geen** voxel-voxel contacten.
- `voxel-edit` doet `Edit`/`EditLog` (place/remove) — puur wereld-edit, geen physics-response.

### Aanbevolen experimenten
- **E6:** valideer dat onze AABB-avatar collision voldoet voor de vertical slice; meet avatar-move-cost (moet <1 ms/frame).
- **E7 (als rigid-body ooit nodig):** volg PJEm — neem **hybride primitive colliders (Jolt) + voxel-terrain**, NIET full voxel-voxel contacts. Concretiseer als ADR-kandidaat; meet 400-vs-72-ragdoll-claim niet blind over.
- **Aandachtspunt:** onze eigen benchmarks (`gpu_bench`, `client_smoke`) moeten in **release**-modus draaien — PJEm bewijst dat debug-cijfers misleidend zijn.

### Tegenbewijs / begrenzing
- Geen repo/getallen gepubliceerd in de video zelf; de "67×" titel is clickbait voor een 12× release-build-effect. De O(N³)-redenering is wel standaard vakkennis en consistent met Teardown/Jolt-discussies.

---

## 4. video-vqWz — volumetric voxel clouds (MishMash)

### Kernclaims & tijdcodes `[V]` (visueel + on-screen tekst, transcript is muziek)
- Titel/tekst: "Raytracing Volumetric Clouds using Voxels (Devlog #5)", kanaal MishMash, 2026-07-14, 3:20.
- Zichtbaar/tekstueel `[V]`: "Raytracing Voxel Cloud Volumes"; "Volumetric horizon scatter at sunset"; weather-event-simulatie + cyclische weather-states; sneeuw die een hele biome verandert; volledig 3D-wolken in de wereld; rays door volume met jitter, "about 1/4 resolution"; dag/nacht, sneeuwstorm, regen, verlichte nachtwolken.
- Kanaalcontext `@MishMash95`: micro-voxel engine, devlogs #3 caves, #4 procedurale vijvers, #5 clouds.

### Techniekclusters (afgeleid, `[O]`/speculatief tot primaire bron)
1. **Volumetric ray marching** door een voxel-cloud-volume, met jitter op lage resolutie (~1/4).
2. **Light transmittance / horizon scatter** voor zonsondergang-wolken.
3. **Weather-state koppeling** aan biome-verandering (sneeuw).

### Codebase-gap
- Onze engine: géén weather-/cloud-stack. Lighting = vertex-AO+directional+fog. Geen volumetrische pass.

### Aanbevolen experimenten
- **E8 (filmische fase, aparte spike):** na stabiele client, prototype low-res voxel-cloud-volume raymarch + jitter als post/overlay-pass; koppel aan onze `time_of_day`. Tracer: capture met/zonder wolken, NEAR_WHITE/Pixel-oracle zoals bestaande client_smoke.
- **E9:** weather-state→biome-koppeling (sneeuw) is ook relevant voor onze `voxel-worldgen` biomes (reeds 7 biome-tiers) — kleine uitbreiding, geen renderer-变更.

### Tegenbewijs / begrenzing
- 3:20, vrijwel louter muziek; claims rusten op on-screen tekst + beeld, **geen** code, geen benchmark, geen methodologie. Puur devlog-eye-candy. Techniek-identificatie is speculatief totdat maker-bron/repo volgt. Niet geschikt als architectuur-bewijs.

---

## 5. Kanalen — identiteit & verificatie

Alle vier kanalen zijn de auteurs van de vier video's (1:1 via oEmbed `author_url`):
- **@MishMash95 (MishMash):** micro-voxel engine, C++→Rust/Bevy (per kanaal-beschrijving/playlist). Devlogs #3–#5. Geen publiek repo waargenomen.
- **@EDBev-b5g (EDBev):** voxel physics met Jolt-hijack, Teardown-stijl. Geen repo in video.
- **@DeadlockCode (Deadlock):** Rust+Vulkan GPU voxel ray traversal; repo `voxel_ray_traversal` (Apache, 100% Rust) — zie §1.
- **@frozein (frozein):** voxel path-tracer; repo `DoonEngine` (MIT, C+GLSL) — zie §2.

**Synthese:** vier solo-indie-devs die elk een deel van de "filmische voxel-engine" bewandelen die onze noordster ook is (GTA VI / Crimson Desert-onder-microvoxels). Hun stacken zijn heterogeen (Rust/Vulkan, C/GLSL, Jolt, Bevy) maar de **renderers zijn ray-gebaseerd** behalve EDBev (triangle/Teardown). Geen van hen toont een volledige, gescaleerde, speelbare open-wereld-RPG zoals ons doel — ze zijn specialistische demo's/spikes.

---

## 6. Verzamelde codebase-gap matrix (exacte crates/bestanden)

| Subsysteem | Ons (crates/) | Set-B | Verdict | Experiment |
|---|---|---|---|---|
| Rendering | `voxel-gpu` greedy-mesh→wgpu triangle raster | ztkh/tDTB/vqWz = ray/raymarch/path-trace | **RETAIN** rasterizer (MVP); ray = latere fase | E1 (locality), E2 (ray-spike) |
| Chunk-opslag | `voxel-core::chunk` dense flat, rij-orde | ztkh: Z-order/3D-texture locality | **UPDATE** backing-store layout | E1 |
| Meshing | `voxel-mesher` greedy+vertex-AO | ztkh: géén meshing | **RETAIN** (wij doen het al) | — |
| LOD/streaming | `chunk_stream.rs` Lod{Full,Half,Imposter}, priority queue, frustum, air-skip, LruMeshCache | ztkh/tDTB: géén streaming/LOD | **RETAIN** (verder dan zij) | — |
| Lighting | `renderer.rs` vertex-AO+directional+fog+PBR triplanar | tDTB: per-voxel shadows+GI (ray) | **RETAIN** MVP; **REJECT** ray-GI nu | E3 (BFS), E4/E5 (ray-fase) |
| Physics | `voxel-player` AABB avatar collision | PJEm: Jolt voxel-hijack, O(N³)-waarschuwing | **RETAIN** avatar; **REJECT** full voxel-RB | E6, E7 (hybride Jolt later) |
| Weather/clouds | geen | vqWz: volumetric clouds | **RETAIN** (niet nu); filmische fase | E8, E9 |
| Determinisme | `voxel-worldgen` seeded fBm, byte-stabiel serialize | — | **RETAIN** | — |

**Netto-oordeel:** onze architectuur is **niet inferieur** aan set-B; op streaming/LOD/meshing zijn we verder. De unieke, waardevolle overnames zijn *specifieke technieken*, niet een rewrite:
1. **Memory-locality** (Z-order/3D-texture) — toepasbaar op onze chunk-opslag (E1).
2. **BFS/zonglift-lighting** voor grotten (E3) — al geïdentificeerd.
3. **Hybride physics** (Jolt primitives + voxel terrain), géén full voxel-RB (E7).
4. **Volumetric clouds/weather** als filmische juice (E8/E9).
5. **Ray/path-trace + temporal accumulation** als late-fase referentie (E2/E4), NIET voor vertical slice.

---

## 7. Bronnen (live geverifieerd)

1. YouTube oEmbed — video-vqWz: `https://www.youtube.com/oembed?url=https://www.youtube.com/watch?v=-vqWzDaWUKk` → auteur MishMash, kanaal @MishMash95.
2. YouTube oEmbed — video-PJEm → auteur EDBev, @EDBev-b5g; titel "One Change made my Voxel Physics engine 67x Faster".
3. YouTube oEmbed — video-ztkh → auteur Deadlock, @DeadlockCode; titel "This Tiny Algorithm Can Render BILLIONS of Voxels in Real Time".
4. YouTube oEmbed — video-tDTB → auteur frozein, @frozein; titel "Adding RAYTRACED LIGHTING to my Voxel Engine | Devlog 3".
5. Transcript `transcripts/video-ztkh.txt` (138 seg, 15:29) — Amanatides-Woo, 3D-texture 10×, repo in description.
6. Transcript `transcripts/video-tDTB.txt` (222 seg, 10:50) — hard/soft shadows, GI, temporal accumulation, RTX 3050 Ti 70 FPS.
7. Transcript `transcripts/video-PJEm.txt` (70 seg, 2:20) — Jolt-hijack, release 12×, O(N³) waarschuwing.
8. Transcript `transcripts/video-vqWz.txt` (13 seg, 3:20) — muziek; claims uit VISUAL_SOURCE_NOTES.md (contact-sheet + on-screen tekst).
9. `github.com/DeadlockCode/voxel_ray_traversal` [R] — Public, 232★, 100% Rust, `LICENSE-APACHE` (Apache-2.0). Video-description link bevestigd via pagina-extractie (GPU Intel Arc A770, CPU Ryzen 9 7900X, 2025-09-05).
10. `github.com/frozein/DoonEngine` [R] — Public, 197★, C 88,9%+GLSL 10,6%, MIT (© Daniel Elwell 2022). "a voxel path-tracer … per-voxel lighting … dynamic CPU→GPU streaming".
11. Amanatides & Woo, "A Fast Voxel Traversal Algorithm for Ray Tracing" (1987) — geciteerd door ztkh als basis (primaire paper, standaardliteratuur).
12. Primair in-repo (onze codebase, gelezen): `crates/voxel-core/src/{coords,chunk}.rs` (CHUNK_SIZE=32, VOXEL_SIZE_M=0.125, Euclidean div/rem), `crates/voxel-worldgen/src/lib.rs` (MAX_SURFACE_M=123 m, fBm, 3-tier biomes, column hbuf cache), `crates/voxel-mesher/src/lib.rs` (greedy + vertex-AO), `crates/voxel-gpu/src/{renderer,chunk_stream,cache}.rs` (WGSL PBR, Lod{Full,Half,Imposter}, LruMeshCache), `crates/voxel-player/src/lib.rs` (AABB collision), `Cargo.toml` (workspace, MIT OR Apache-2.0).

**Licentie-status:** onze engine = MIT OR Apache-2.0. Deadlock repo = Apache-2.0 (compatibel). DoonEngine = MIT (compatibel). Bij eventuele code-overname uit die repo's is attribuut verplicht en blijft onze dual-licentie intact.

**Openstaande verificatie vóór `accepted`:**
- vqWz: maker-repo/paper voor cloud-raymarch-details (nu alleen visueel).
- ztkh/tDTB: volledige repo-lezing (alleen hoofd + licentie live gezien; algoritme-internals niet line-by-line geverifieerd).
- PJEm: Jolt narrow-phase "hijack" broncode + exacte 400-vs-72-ragdoll-cijfers (niet in video).
- De 8 set-A-kanen (dphfox, DouglasDwyer, xima1, IGoByLotsOfNames) vallen buiten set B en zijn niet in deze run behandeld.
