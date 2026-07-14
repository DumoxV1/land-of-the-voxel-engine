# State-of-the-art onderzoek micro-voxel engines — advies voor onze engine
*Datum: juli 2026 · Context: Rust, wgpu 0.17, greedy meshing per chunk, RTX 4080, headless authoritative server, doel ~150 km², 2–8 spelers, filmische micro-voxel look. Regel: correctheid > dichtheid.*

## Prioritering (samenvatting)

| # | Onderwerp | Aanbeveling | Complexiteit |
|---|-----------|-------------|--------------|
| 1 | Winit-client + render loop | winit 0.30 `ApplicationHandler`, wgpu upgraden naar recente versie | S/M |
| 2 | Chunk-streaming + background meshing | rayon-pool + crossbeam-kanalen, prioriteit op afstand, staging-buffer uploads | M |
| 3 | Meshing | Binary greedy meshing (Tantan-stijl) als drop-in versnelling; GEEN meshless/raymarching nu | S |
| 4 | Belichting | Voxel-AO (vertex-based) + 1 cascaded shadow map nu; voxel-GI uitstellen | M |
| 5 | LOD | Chunked octree/clipmap-LOD met per-niveau downsampling; pas nodig ná 1 km²-benchmark | L |
| 6 | Worldgen | Noise-layering (continentalness/erosie/moisture) + biome-lookup; rivieren via Veloren-aanpak later | M |
| 7 | Netcode | Snapshot-interpolatie voor entiteiten + betrouwbare edit-log voor voxels (jullie hebben al de goede basis) | M |

---

## 1. Meshing: greedy vs binary greedy vs GPU-driven meshless

**Aanbeveling: blijf bij mesh-gebaseerd renderen; vervang de klassieke greedy mesher door binary greedy meshing. Complexiteit: S.**

- Binary greedy meshing (bitmask-gebaseerd, 64-bit kolommen + bitwise ops) is een algoritmische drop-in die dezelfde output geeft als klassieke greedy meshing maar ~10–30× sneller is. Tantan meet ~0,2 ms per 32³-chunk; de `binary-greedy-meshing` crate claimt ~30× sneller dan `block-mesh-rs`. Dit verlaagt direct de latency van remeshing na edits — belangrijk voor correctheid van interactie.
- GPU-driven meshless rendering (raymarched bricks / DDA / SVO à la John Lin, Teardown) geeft de mooiste micro-voxel look en schaalt beter bij extreme dichtheid, maar: John Lin zelf waarschuwt in "The Perfect Voxel Engine" dat SVO's alleen goed zijn in opslag+rendering en slecht in collision, GI, pathfinding en dynamische objecten — en dat het genre synoniem is met vaporware. Voor een team met "correctheid > dichtheid" en een geplande interactieve client is dit nu de verkeerde afslag. Herzie dit pas als greedy meshing aantoonbaar de bottleneck is op de 1 km²-benchmark.
- Let op bij binary greedy meshing: AO/per-voxel data breekt quad-merging; Tantans demo lost dit op door AO-waarden in de bitmasks mee te hashen.

Bronnen:
- https://github.com/TanTanDev/binary_greedy_mesher_demo (Rust/Bevy demo + uitleg)
- https://crates.io/crates/binary-greedy-meshing (productie-crate, gebruikt in Riverbed)
- https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/ (canonieke referentie greedy meshing)
- https://voxely.net/blog/the-perfect-voxel-engine/ (John Lin: waarom pure-render-first engines stranden)

## 2. LOD voor 150 km²

**Aanbeveling: chunked octree-LOD ("chunked clipmaps"): ringen van steeds grovere chunks rond de speler, elk niveau 2× downsampled en apart gemesht. Complexiteit: L — maar pas starten ná de 1 km²-benchmark.**

- 150 km² op micro-voxelresolutie is zonder LOD onhaalbaar (10 cm voxels ⇒ ~10⁵ × 10⁵ kolommen). De bewezen aanpak voor voxelterrein is clipmap-achtige ringen (zoals Proc World/Voxel Farm van Miguel Cepeda) of een octree waarvan je per diepte een mesh bakt (Avoyd streamt zo zelfs over netwerk).
- Kritisch punt: LOD-naden (cracks) tussen niveaus zijn de echte moeilijkheid, niet de datastructuur. Kies vroeg een naadstrategie (skirts zijn het simpelst en visueel prima voor filmische look).
- Volgorde: eerst 1 km² zonder LOD benchmarken; jullie meetdata bepaalt hoeveel LOD-niveaus nodig zijn. Bouw de chunk-key alvast als (x, y, z, lod-level) zodat streaming-code niet herschreven hoeft te worden.

Bronnen:
- http://procworld.blogspot.com/2011/10/clipmaps.html (clipmaps op octree-voxeldata)
- https://www.enkisoftware.com/devlogpost-20140112-1-Octree-streaming (Avoyd: octree-LOD + streaming)
- https://www.reddit.com/r/VoxelGameDev/comments/kol6lt/resources_on_chunked_clipmap_lod/ (praktische chunked-clipmap discussie)

## 3. Chunk-streaming & background meshing in Rust

**Aanbeveling: dedicated rayon-pool voor worldgen+meshing, crossbeam-kanalen naar de render-thread, uploads via `Queue::write_buffer` (of staging belt) met budget per frame. Complexiteit: M.**

- Architectuur die in de Rust-voxelscene bewezen is (o.a. Veloren, Riverbed): main/render-thread stuurt "chunk gewenst"-verzoeken (gesorteerd op afstand tot camera, met hysterese om thrashen te voorkomen) → worker-pool genereert/mesht → kant-en-klare vertexdata via kanaal terug → render-thread uploadt met een frame-budget (bv. max N MB of M chunks per frame) om spikes te vermijden.
- Gebruik een aparte `rayon::ThreadPool` (niet de globale) zodat meshing nooit de render-thread of servertick verhongert. Kanalen: `crossbeam-channel` of `flume`; `try_recv` in de frame-loop.
- wgpu-uploads: voor chunk-meshes is `Queue::write_buffer` prima (wgpu batcht intern via staging); bij hoge churn een eigen buffer-pool/vertex-pooling (Nick McDonald's "vertex pooling"-artikel) om alloc/free van GPU-buffers te vermijden. Vermijd per-chunk `create_buffer` elke remesh.
- Annuleer verouderde taken: als een chunk al uit range is voordat de worker klaar is, gooi het resultaat weg (generation-counter per chunk).

Bronnen:
- https://nickmcd.me/2021/04/04/high-performance-voxel-engine/ (vertex pooling, upload-strategie)
- https://rtarun9.github.io/blogs/async_copy/ (multi-threaded chunk loading + async copy)
- https://github.com/veloren/veloren (referentie-implementatie Rust voxel-MMO: streaming, meshing, netcode)

## 4. Belichting: voxel-AO, CSM vs voxel-GI

**Aanbeveling: nu (a) vertex-based voxel-AO (0fps-methode) en (b) één cascaded shadow map voor de zon. Voxel-GI (flood-fill of DDGI-achtig) als fase 3. Complexiteit: M (AO=S, CSM=M, GI=L).**

- Voxel-AO à la 0fps is goedkoop, wordt tijdens meshing berekend (4 buurcases per vertex, let op de bekende quad-flip voor anisotropie-artefacten) en geeft micro-voxels direct 80% van de "filmische" diepte.
- CSM voor de zon is de bewezen route voor open werelden; met 3–4 cascades over enkele km werkt dit op een 4080 ruim binnen budget. Teardown's volledige voxel-raytracing (Gustafsson, "Raytracing Voxels in Teardown and Beyond") is prachtig maar veronderstelt een brickmap/raytracing-datamodel dat jullie (terecht, zie §1) niet hebben.
- Voor Minecraft-achtige gameplay-belichting (licht door edits) is flood-fill voxel lighting (0fps "Voxel lighting") de correcte, deterministische keuze — past bij authoritative server omdat het puur op voxeldata draait.

Bronnen:
- https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/ (voxel-AO, incl. quad-flip pitfall)
- https://0fps.net/2018/02/21/voxel-lighting/ (flood-fill lighting, optimalisaties)
- https://www.youtube.com/watch?v=IM1Dr98f3xU (Teardown: "Raytracing Voxels in Teardown and Beyond" — referentie voor latere GI-ambities)

## 5. winit + wgpu render-loop (2024+)

**Aanbeveling: upgrade wgpu 0.17 → recente versie (≥22/23) en winit 0.30 met `ApplicationHandler`; doe dit vóór de client-fase, niet erna. Complexiteit: S/M.**

- wgpu 0.17 (2023) is verouderd; latere versies fixen surface/resize-gedrag, hebben `Surface<'window>`-lifetimes die met winit 0.30's `ApplicationHandler` samengaan, en betere presentatie/frame pacing. Migreren met een offscreen renderer + kleine codebase is nu goedkoop; later duur.
- winit 0.30-patroon: struct met `Option<Window>`/`Option<State>`, window + surface aanmaken in `resumed()`, renderen op `RedrawRequested`, en `window.request_redraw()` aan het eind van de redraw voor continue rendering (i.p.v. `ControlFlow::Poll`-busy-loop).
- Resize: config bijwerken in het resize-event, nooit een surface van 0×0 configureren (minimize!), en op `SurfaceError::Lost/Outdated` reconfigureren en frame overslaan. Frame pacing: vertrouw op `present_mode` (Fifo = vsync als default; Mailbox voor lage latency op de 4080) en meet CPU/GPU-tijden apart in de benchmark.

Bronnen:
- https://github.com/rust-windowing/winit/discussions/3667 (winit 0.30 + wgpu referentiepatroon)
- https://sotrh.github.io/learn-wgpu/beginner/tutorial1-window/ (Learn WGPU, bijgewerkt voor winit 0.30)
- https://github.com/gfx-rs/wgpu/issues/3868 (resize-lag discussie + mitigaties)

## 6. Worldgen: biomes, rivieren, structuren

**Aanbeveling: stap over van pure heightmap op gelaagde noise-velden (continentalness, erosie, ruwheid, temperatuur, vochtigheid) met een biome-lookup-tabel; splines voor hoogte. Rivieren en structuren daarna. Complexiteit: M (rivieren/structuren: L).**

- Het moderne Minecraft-model (meerdere lage-frequentie noisevelden → spline-mapping → biome-tabel op temperatuur×vochtigheid) is deterministisch, seed-stabiel en goedkoop per kolom — past perfect bij jullie seeded value-noise en de correctheidsregel. Domain warping toevoegen geeft veel visuele winst voor weinig code.
- Rivieren die écht kloppen (bergafwaarts stromen) vereisen globale kennis; Veloren lost dit op met een aparte wereldwijde pre-simulatiefase (riviernetwerk + erosie op lage resolutie, daarna per-chunk verfijning). Dat is de juiste architectuur voor 150 km²: een offline/lazy "wereldkaart"-pas boven de per-chunk generator.
- Structuren: genereer deterministisch per regio-seed met een "structure starts"-fase vóór chunk-vulling (Minecraft-model), zodat structuren chunkgrenzen kunnen overschrijden zonder ordering-bugs.

Bronnen:
- https://www.youtube.com/watch?v=CSa5O6knuwI (Henrik Kniberg, "Minecraft terrain generation in a nutshell" — dé referentie voor noise-layering/splines/biomes)
- https://news.ycombinator.com/item?id=43517337 + https://github.com/veloren/veloren (Veloren: riviernetwerk + erosie-gebaseerde worldgen in Rust, broncode beschikbaar)
- https://www.redblobgames.com/maps/terrain-from-noise/ (Red Blob Games: noise→terrein/biomes, grondige uitleg)

## 7. Netcode voor voxel-games

**Aanbeveling: hybride: (a) snapshot-interpolatie voor spelers/entiteiten (Gaffer-model, 20–30 Hz snapshots + ~100–150 ms interpolatiebuffer), (b) voxel-edits als betrouwbare, geordende edit-log met server-tick-nummers, (c) chunk-data als initiële bulk-download + daarna alleen edits. Complexiteit: M.**

- Met 2–8 spelers en een authoritative server is dit ruim voldoende; client-side prediction alleen voor eigen beweging, niet voor edits (edit pas tonen na server-ack, of optimistisch met rollback — begin met ack-based: simpeler en correct).
- Edit-log + tick-nummers maakt de wereldstaat reproduceerbaar en sluit aan op jullie binaire persist-formaat: persistentie = chunk-basis + gecompacteerde edit-log. Comprimeer chunk-payloads (LZ4/zstd) — voxeldata comprimeert extreem goed.
- Vermijd het hersynchroniseren van hele chunks bij elke edit; stuur (pos, oud→nieuw) deltas, en alleen naar clients die de chunk geladen hebben (interest management op chunk-radius).

Bronnen:
- https://gafferongames.com/post/snapshot_interpolation/ (canoniek: snapshot-interpolatie)
- https://snapnet.dev/blog/netcode-architectures-part-3-snapshot-interpolation/ (moderne architectuur-analyse)
- https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech (Teardown: multiplayer + voxel-destructie in de praktijk)

---

## Kritische noot over hype

- "Meshless" micro-voxel showcases (John Lin, veel YouTube-devlogs) zijn render-demo's, geen games; Lin's eigen blog benoemt expliciet dat het datamodel voor rendering vaak alle andere systemen (physics, netwerk, AI) saboteert. Lay of the Land en Teardown zijn wél shipping games — beide gebruiken pragmatische hybrides en beperken wereldgrootte of destructiegranulariteit.
- Voor 150 km² is de schaal, niet de voxeldichtheid, jullie hoofdrisico. De grootste onbekenden zijn LOD-naden en streaming-I/O, niet meshing-snelheid. De 1 km²-benchmark eerst uitvoeren met binary greedy meshing + streaming geeft de data om LOD-beslissingen te onderbouwen in plaats van te gokken.
