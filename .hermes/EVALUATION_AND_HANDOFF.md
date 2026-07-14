# Externe evaluatieprompt + next-session handoff

Dit bestand bevat (A) een kant-en-klare prompt die je aan een externe AI (of een andere sessie)
kunt geven om het hele project te laten beoordelen en verbeteren op basis van papers + openbare
github/opensource, en (B) wat de huidige Hermes-sessie nodig heeft om verder te gaan zonder
context te verliezen (token-besparing).

================================================================================
(A) PROMPT VOOR EXTERNE EVALUATIE / VERBETERING
================================================================================

Kopieer alles tussen de ---- MARKERS ---- en plak het in je externe tool (bijv. een andere
AI-chat, of een research-agent). Pas de eerste regel aan als je een specifieke focus hebt.

---- MARKERS ----
Je bent een senior Rust engine-architect en game-tech reviewer. Beoordeel en verbeter het
open-source project "Land of the Voxel Engine" (een micro-voxel openwereld-RPG-engine in
puur Rust, gericht op filmische kwaliteit à la "de GTA VI / Crimson Desert onder
micro-voxel-engines"). De repository staat op:
https://github.com/DumoxV1/land-of-the-voxel-engine

Doe het volgende, en onderbouw ELKE bewering met een bron (paper, officiële docs, of een
concreet openbare github-repo met permalink + relevante bestandslijn):

1. ARCHITECTUURREVIEW
   - Lees de ADR's in docs/architecture/adr/ (0001-0004), .hermes/PROJECT_STATE.md,
     docs/ROADMAP.md en de crate-structuur (crates/*).
   - Beoordeel of de keuze renderer-agnostische core (ADR-0002), server-authority +
     determinisme (ADR-0003) en client-shell = Rust + Bevy/wgpu (ADR-0004) technisch
     houdbaar is voor een 150 km² wereld met 2-8 spelers en filmische kwaliteit.
   - Benoem concrete zwakke punten en risico's (schaal, determinisme-over-netwerk,
     meshing-kost, draw-calls, chunk-streaming).

2. MESHING & RENDERING (focus op voxel-gpu + voxel-mesher)
   - De huidige renderer gebruikt greedy meshing + een WGSL-shader met per-normaal
     directionele belichting, warme fog en materiaal-tinten (offscreen, wgpu 0.17.2,
     Vulkan/RTX 4080). Zoek naar betere/modernere aanpakken in papers en opensource:
     * Greedy meshing vs. "visible surface determination", "Merf" (memory-efficient
       voxel rendering), "Dual Contouring" / "Marching Cubes" voor smooth voxels.
     * Voxel GI / ambient occlusion: "Airlight" / SSAO-varianten, voxel cone tracing.
     * WGSL/WebGPU best practices en wgpu 0.20+ migratie (huidige code zit op 0.17.2).
   - Geef per aanbeveling: wat het verbetert, de trade-off, en een referentie-implementatie.

3. WORLDGEN & PERFORMANTIE
   - Beoordeel seeded value-noise heightmap (voxel-worldgen) vs. moderne noise
     (simplex/open-simplex, "GEA" erosion, "fast hydrology"). Zoek papers over
     procedurale terrain die beter schaalt naar 150 km².
   - Beoordeel chunk-streaming en de 4-bit per-voxel + per-chunk palette (≤16) keuze:
     wanneer wordt 16 materialen te weinig, en wat zijn alternatives (RLE, sparse
     octree, "SVDAG")?

4. NETWERK & MULTIPLAYER (Fase 4)
   - De headless server (voxel-server) is authoritative met een append-only edit-log.
     Beoordeel dit tegen "state sync" patterns (snapshot interpolation, delta
     compression, rollback/netcode à la Gabriel Gambetta, Valve-articles).
   - Zoek opensource-voorbeelden (bijv. bevy_renet, naia, turbulence, "valence") en
     geef een aanbevolen protocol-vorm voor 2-8 spelers.

5. PRAKTISCHE ACTIELIJST
   - Eindig met een geprioriteerde, concrete TODO-lijst (P0/P1/P2) met per item:
     bestandslocatie, wat te veranderen, verwachte winst, en de bron/papers die het
     rechtvaardigen. Ongeveer 60-40 verhouding: "nu doen" vs. "onderzoeken".
   - NOEM explicit welke verbeteringen RISICOVOL of PREMATURE zijn voor een vertical
     slice, zodat we niet over-engineeren.

Geef je antwoord in het Nederlands (of tweetalig NL/EN als dat technisch handiger is).
Wees concreet en citaat-bestendig: geen verzonnen metrics, elke claim = bron.
---- MARKERS ----

================================================================================
(B) NEXT-SESSION HANDOFF (voor de volgende Hermes-sessie — token-besparend)
================================================================================

Als de sessie wordt gereset/vernieuwd, geef dan door (of lees zelf uit de repo):

1. CANONIEKE BRONNEN (altijd eerst lezen):
   - .hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md
   - .hermes/PROJECT_STATE.md  (nu t/m S-11)
   - docs/governance/alignment-log.md  (t/m S-11 entry)
   - docs/ROADMAP.md  (fasen 2-6 + Fase 2b tech-schuld + workflow-lessen)
   - docs/research/2026-07-15-sota-advies-vervolg.md  (bron-onderbouwd SOTA-advies)
   - docs/architecture/adr/0001..0004

2. HUIDIGE STAAT:
   - Branch master -> origin/main. Laatste commits: S-11 audit-hardening + roadmap-update
     (na 2fe4548 S-10 GPU). `git log --oneline -6` voor exacte hashes.
   - Werkdirectory: C:\Users\keere\Desktop\Land of the Voxel Engine
   - 10 crates: voxel-core, voxel-mesher, voxel-render (software-raster), voxel-worldgen,
     voxel-world, voxel-edit, voxel-persist, voxel-player, voxel-server, voxel-gpu (wgpu).
   - Tests: cargo test --workspace = 57/57 groen (incl. 9 s11_audit-tests).
   - GPU bewezen: voxel-gpu rendert terrain op RTX 4080 (gpu_world.png, probe.png),
     nu mét backface-culling (mesher levert CCW-winding + correcte vlakposities).
   - S-11 fixte: mesher face-planes (+vlakken op d+1) & winding, chunk-serialize v3
     (i64 coords + nibble-validatie), player terminal velocity + footprint floor-resolve,
     server gesorteerde tick-volgorde, gpu grass/dirt-tint + fog-vanaf-eye +
     Backends::PRIMARY, persist atomair schrijven (tmp+rename).

3. BELANGRIJKE TECHNISCHE FEITEN (voorkom herontdekking):
   - wgpu gepind op 0.17.2 (0.18.0 yanked; 0.19/0.20 API-drift). Bij upgrade: lees de
     registry-source of docs.rs eerst. Verschillen: RenderPassColorAttachment.ops,
     DeviceDescriptor zonder required_*, TextureUsages/BufferUsages (meervoud),
     draw(Range,Range), map_async(mode, callback) niet-async, uniform vec3 = 16-byte align.
   - Camera-matrix via `glam` (Mat4::look_at_rh / perspective_rh) — NIET handmatig schrijven.
   - WGSL: module-level `const array` mag niet met dynamic index; gebruik if-ladder.
   - git push vanaf lokale `master`: `git push origin master:main` (niet `git push origin main`).
   - .gitignore bevat Cargo.lock (bestaande conventie; niet wijzigen zonder overleg).
   - openrouter-latest.json is runtime — NOOIT committen.

4. VOLGENDE AUTONOME STAP (Fase 2, volgorde uit ROADMAP "Directe volgende stap"):
   a. wgpu/winit-upgrade: wgpu 0.17.2 -> recent (>=22) + winit 0.30 ApplicationHandler-
      patroon. Lees docs.rs van de gekozen versie eerst; verwacht API-drift (zie feiten
      hierboven, die gelden voor 0.17!).
   b. Interactieve GPU-client: winit-venster + render-loop + WASD/muis-camera in voxel-gpu.
      Doe tegelijk Fase-2b #1: World::get/material_at zonder chunk-clone (perf-lek in
      collision; anders vertekent de FPS-benchmark).
   c. Chunk-streaming (rayon-pool + kanalen, afstand-prioriteit, upload-budget per frame);
      chunk-key alvast (x,y,z,lod).
   d. Fase-2 benchmark-gate (B-06/B-07/FPS op 1 km²) vóór ADR-0004 lock-in.

5. GEBRUIKERSCONTEXT:
   - Gebruiker is niet-technisch, geeft volledige autonomie, communiceert in het Nederlands,
     wil minimale tussenstops. Volmacht: doorbouwen tot werkend product; push naar eigen
     repo toegestaan. Eis: engine moet op GPU draaien (voldaan sinds S-10).
   - Art-direction ref: Lay of the Land (~90%) + John Lin (~10%).
   - Hardware: RTX 4080 Super, 32 GB RAM, Intel Core Ultra 7 265K. Windows 10.
   - Budget: geen betaalde calls; gratis OpenRouter-modellen; project-key < $36.
