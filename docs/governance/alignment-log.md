# Plan-alignment log

Hier wordt iedere verplichte terugstap en iedere materiële koerscorrectie vastgelegd.

## 2026-07-15 — WASD-bewegingsbug gefixt (te snel + wit bij vliegen)

- Twee oorzaken gevonden en onder TDD gefixt:
  1. **Super snel:** `update_camera` telde `speed` per-frame op (geen dt) → bij hoge FPS
     ~384 m/s. Nu `voxel_gpu::free_fly_step(eye, yaw, pitch, dt, speed, keys)` met echte
     frame-dt; 8 m/s basis, Shift = 4× sprint. Test `free_fly_speed_is_frame_rate_independent`.
  2. **Wit bij vliegen:** streaming-loop én `nearest_visible_chunk` hadden `if cx<0||cz<0
     continue` → vliegen in negatieve ruimte selecteerde 0 chunks → tris=0 → wit.
     `ChunkCoord` is i64 + Euclidean div, dus negatieve chunks zijn geldig. Guards verwijderd.
     Test `negative_chunk_coords_yield_nonempty_mesh` bewijst negatieve chunks geven terrain.
- Geverifieerd: live capture bij camera 200 m in negatieve ruimte = 7116 unieke kleuren,
  NEAR_WHITE_PCT=0.1% (niet wit). Normale spawn = 6575 kleuren, 0.002% wit.
  Workspace 36 test-binaires groen.

## 2026-07-15 — Mijlpaal 4 P0: 4K-texture-system (texture-array + triplanar + PBR)

- P0 GEDAAN: `GpuScene` krijgt `MaterialPbr`-storage-buffer + `texture_2d_array` albedo +
  anisotropic sampler (clamp 16). WGSL doet triplanar-sample langs wereld X/Y/Z.
- Strikte TDD: failing test `grass_surface_shows_texture_variation_not_flat_tint` bewees
  dat een gras-oppervlak nu >1 groentint toont (textuur) i.p.v. één vlakke tint.
- Bijkomende bug: VBO-pool groeide voorbij `max_buffer_size` (256 MB) bij grote view-radius
  → client-panic. Gepakt op device-limiet met veilige growth-steps.
- Geverifieerd: live client capture 3072 unieke kleuren (was 434), gras/steen met textuur.
  Benchmark 1 km² p50=0.24ms, avg_fps≈3636 — geen FPS-daling t.o.v. pre-texture 3753.
  Workspace 36 test-binaires groen.

## 2026-07-15 — audit: wit scherm was echte bug, tests gaven false positive

- Gebruiker rapporteerde blijvend wit scherm na de "never go white" hotfix.
- Verificatie via **native window-capture** (Python `win32gui.PrintWindow` op de echte
  `gpu_window.exe`, niet de offscreen smoke): **92,9% van het venster was puur wit**,
  géén clear-kleur, géén terrain. Eerdere 63/63 groene tests waren dus een **false positive**
  voor de live client.
- Onafhankelijke audit (2 subagents) bevestigde de rootcause-keten zonder de code te wijzigen:
  **camera-Y in voxels → frustum vindt 0 chunks → lege tris → render faalt → fout ingeslikt
  met `.is_ok()` → nooit gepresenteerd → wit window**. Tweede fout: resize wijzigde de
  surface maar niet de depth-texture.
- Fix strict TDD: twee Rood→Groen tests (`live_spawn_frustum_contains_at_least_one_world_chunk`,
  `streamed_mesh_is_in_chunk_world_meters`) + `resize_recreates_matching_depth_attachment`.
  Centraal coördinatencontract: `mesh_chunk_world_meters()` + `spawn_eye_y_m()`; alle chunkmeshes
  krijgen nu `(chunk_origin + local) * 0,125` meter. Live renderfouten worden gelogd.
- **Bewijs na fix:** PrintWindow-capture toont 0,002% wit, 434 unieke kleuren, lucht + terrain
  zichtbaar. Workspace 65/65 groen. Benchmark herschreven naar echte 12,5 cm-schaal:
  1 km² = 250×250 = 62.500 chunks, avg_fps≈3750 (RTX 4080).
- **Les (toegevoegd aan AGENTS.md-werkprotocol):** offscreen/smoke-tests zijn GEEN bewijs voor
  de live window/surface/present-flow. Voeg een pixel-oracle of native window-capture toe aan de
  verificatie van elke render-path-change. `.is_ok()` rond presentatie mag nooit een fout maskeren.
- Planstatus: aligned. Volgende stap (Mijlpaal 4, 4K-textures) mag starten.

## 2026-07-14 — initialisatie
- Canoniek plan bevestigd.
- North star toegevoegd: filmische openwereldkwaliteit geïnspireerd door de ambitie van GTA VI / Crimson Desert, zonder assets of ontwerpen te kopiëren.
- Gratis OpenRouter-research als standaard bevestigd.
- Terugstapregel na iedere derde stap vastgelegd in `AGENTS.md` en plan.
- Status: aligned; volgende gate is governance + monitoring + Kanban.

## 2026-07-14 — verplichte terugstap na uitvoeringsstap 3
- Eén stap teruggegaan naar stap 2: researchprotocol, budgetbeleid en benchmarkcontract opnieuw gelezen.
- `scripts/plan_alignment_check.py` opnieuw uitgevoerd: OK.
- Beide monitoringscripts syntactisch gecompileerd: OK.
- OpenRouter-guard live uitgevoerd zonder inferencecall: 23 gratis modellen, 17 met tools; keyusage $9,4192 en $30,5808 resterend op de key ten tijde van de controle.
- Afwijking gevonden en gecorrigeerd: cron-environment had de API-key niet geëxporteerd; guard leest nu veilig de Hermes `.env` zonder de sleutel te loggen.
- Planstatus: aligned. Volgende stap mag starten.

## 2026-07-14 — verplichte terugstap na uitvoeringsstap 6
- Eén stap teruggegaan naar stap 5: Kanban-kaarten, dependencies, assignees, workdir en acceptance criteria opnieuw gecontroleerd.
- Research → onafhankelijke review → plansynthese vormt een correcte afhankelijkheidsgraaf; gebruikerskaart is `needs_input` en blokkeert research niet.
- Alle vier workerprofielen zijn expliciet gepind op `openrouter/free`; betaalde fallback is niet geconfigureerd.
- Drie cronjobs gecontroleerd: twee no-agent guards draaiden succesvol; de enige LLM-cron is expliciet `openrouter/free` en read-only.
- Budgetguard opnieuw uitgevoerd: gerapporteerde all-time keyusage $11,5444 en $28,4556 resterend. De €/$10-reviewdrempel is daardoor bereikt; autonome betaalde inference blijft dicht en alle geplande research blijft gratis.
- Plan-alignment opnieuw uitgevoerd: OK. Volgende stap mag starten.

## 2026-07-14 — volledige audit vóór engine-start
- Hermes Doctor en configcheck: geslaagd; alleen niet-benodigde optionele integraties ontbreken.
- Gateway geïnstalleerd als Windows-loginitem en gestart; cronstatus bevestigt dat jobs automatisch zullen draaien.
- Canonieke unittest-suite toegevoegd voor budgetgrenzen, secretveiligheid en fail-closed plan-alignment: 3/3 groen.
- Ad-hoc integratietest met tijdelijk `hermes-verify-*`-script: groen; script opgeruimd.
- Vier researchmemo’s aanwezig, maar onafhankelijke review en architectuursynthese zijn nog niet voltooid; productie-enginecode blijft daarom achter de gate.
- Ongeautoriseerde implementatietaak `t_d9b45797`, door een researchworker te vroeg aangemaakt, gearchiveerd en het workerproces beëindigd.
- Bronsteekproef vond onbetrouwbare claims in researchmemo’s (onder andere Bevy-repo ten onrechte als actief beschreven en niet-onderbouwde benchmarkcijfers). Deze memo’s blijven hypotheses tot reviewcorrectie.
- Budgetguard gecorrigeerd: drempels gebruiken project-key spend, niet dubbelzinnige accountbrede historische usage. Live rapport na correctie: $17,8022 project-key spend en $22,1978 resterend.
- Besluit: infrastructuur werkt; start van productiecode wacht op researchreview → synthese → expliciet spikeplan.

## 2026-07-14 — blocking findings B-01…B-08 toegepast + synthese voltooid
- Review (review-initial-bundle.md) eiste 8 blocking findings (B-01…B-08) toegepast op de memo's vóór architect-synthese.
- B-01: network-persistence Claim 3 "< 1 KB/tick" vervangen door "techniek aangetoond; voxel-benchmark vereist (Experiment 2)" — reeds toegepast in eerdere sessie.
- B-02: Claim 4 Reddit-bron → Mikolalysenko/Teardown primaire bron — reeds toegepast.
- B-03: Claim 7 GNS-licentie gecorrigeerd naar Valve BSD-like + crate MIT — reeds toegepast.
- B-04: voxel-data-rendering benchmarks gemarkeerd als "unverified — requires Criterion reproduction on RTX 4080 Super".
- B-05: Blocky "Production Readiness" gedegradeerd naar Medium (unproven at RPG scale with networking).
- B-06: engine-stack "Godot-first" aanbeveling verwijderd; beide spikes nu equal-priority per plan §2.2; SAM-referentie vervangen door Mikolalysenko/Vulkan-voorbeelden.
- B-07: core-portabiliteitscriterium (renderer-agnostic `voxel-core` crate) toegevoegd + benchmarks B-06 (determinism replay) en B-07 (headless multiplayer soak).
- B-08: north-star metrics geannoteerd met "target — validate in spike S-XX"; Audio-subsysteem-row toegevoegd; meta-risico "free-model instabiliteit" (was risk 7) verplaatst naar dit governance-log.
- Traceability-matrix aangemaakt: `docs/research/traceability-matrix.md` (Plan → Claim → Bron → Spike).
- Drie ADR's gesynthetiseerd van uitsluitend geverifieerde bevindingen: `adr/0001-voxel-representation.md`, `adr/0002-renderer-agnostic-core.md`, `adr/0003-multiplayer-target.md`.
- Status: researchreview + plansynthese VOLTOOID; engine-startgate geopend voor S-01 (voxel-core) onder strict TDD.

## 2026-07-14 — verplichte terugstap na uitvoeringsstappen + S-01 TDD-afronding
- Eén stap terug: alle 8 blocking findings (B-01…B-08) opnieuw geverifieerd tegen de memo's op schijf — allemaal toegepast.
- ADR's 0001–0003 en traceability-matrix aanwezig en gegenereerd van uitsluitend geverifieerde claims.
- Plan-alignment-check: OK (canoniek plan + governancebestanden aanwezig).
- `cargo test -p voxel-core --features proptest`: 7/7 groen (5 integratie + 1 property + 1 lib-unit). Rode fase aantoonbaar vóór groene fase (strict TDD).
- `cargo build -p voxel-core`: OK; geen godot/bevy/wgpu import of dependency (ADR-0002 renderer-agnostisch bevestigd).
- Project guard suite: 3/3 groen.
- Budgetguard: project-key spent $27,5299 (drempel €22 gepasseerd, €30 nog niet). Geen betaalde calls deze sessie; alles via lokale tools + gratis `:free`. Paid blijft dicht.
- Git-repo geïnitialiseerd; lokale commit e6cb818 (geen push — conform aanbevolen grens A van blocked USER INPUT-kaart t_b624d2cb).
- Planstatus: aligned. Volgende stap: S-01 hardening (dense/palette chunk states) of S-02 mesher-spike na goedkeuring.

## 2026-07-14 — S-02 mesher-spike voltooid (strict TDD) + verplichte terugstap na uitvoeringsstap 3
- Kanban USER INPUT-kaart t_b624d2cb gesloten (optie A gekozen); autonome besluitvorming gedelegeerd aan Hermes + eigen onderzoek. Grens A: lokaal commit/push naar eigen repo zonder vragen; publicatie/release/paid-top-up/destructieve git blijft goedkeuringsplichtig.
- Keuze (autonoom): S-02 mesher-spike boven S-01-hardening — logische volgende bewijslaag na coördinaat/chunk-kern.
- Strict TDD: eerst failing tests (`tests/spike_s02.rs`) gecommit in rode fase (API bestond niet → compileerfout), daarna implementatie in `src/lib.rs` (naive → culled → greedy).
- Rode fase aantoonbaar: `cargo test -p voxel-mesher` faalde met "cannot find function `naive_mesh`".
- Groene fase: `cargo test -p voxel-mesher` → 6/6 groen. Hele workspace: voxel-core 6 + voxel-mesher 6 = 12 tests groen.
- Acceptance criteria S-02 vervuld: (a) culling verwijdert interne faces (culled << naive voor solide blok); (b) greedy ≤ 1,5× culled triangle-count; (c) waterdicht — geverifieerd via oppervlakte (6·N² voor volle chunk, 6·N²+6·(N-2)² voor holle shell van dikte 1); (d) geen renderer-dep/godot/bevy/wgpu (ADR-0002 bevestigd).
- Correctie tijdens TDD: eerste test-verwachtingen waren fout (onderstelden ten onrechte 6 vlakken voor holle shell en 0 blootgestelde faces voor volle chunk). Tests herschreven naar echte geometrie; mesher-code was correct.
- Verplichte terugstap: plan-alignment OK, project guard suite 3/3 groen, workspace-tests groen, budgetguard project-key spent $27,5299 (paid blijft dicht). Geen drift t.o.v. canoniek plan.
- Commit + push naar origin main (grens A): S-02 tracer-bullet plan, voxel-mesher crate, alignment-log update.
- Planstatus: aligned. Volgende stap: S-01-hardening (dense/palette chunk states) of S-03 (renderer/camera spike) — keuze autonoom volgende sessie.

## 2026-07-15 — S-01-hardening voltooid (strict TDD) + verplichte terugstap na uitvoeringsstap 3
- Autonome keuze: S-01-hardening boven S-03 — eerst waterdichtheid van voxelopslag borgen (drie chunk-states + 4-bit bitpacking) vóór renderer-spike; sluit aan op USER INPUT-kaart t_b624d2cb (optie A) en de sessie-takenlijst.
- ADR-0001 verplicht drie chunk-states + per-chunk palette (≤16 materialen), bitpacked materiaal-ID's. Voor S-01 bestond alleen `Uniform`/`NonUniform(dense)`.
- Strict TDD:
  - ROOD: `tests/spike_s01_hardening.rs` (7 failing tests) + `spike_s01.rs` aangepast naar `ChunkState::PalettePacked`/`Dense`. `cargo test -p voxel-core` faalde met "no method named `palette`/`packed_data`" en "no variant `PalettePacked`/`Dense`".
  - GROEN: `src/chunk.rs` herschreven naar `Uniform | PalettePacked | Dense` met 4-bit bitpacking (2 voxels/byte via `write_nibble`), per-chunk palette (`PALETTE_LIMIT=16`), automatische promotie naar `Dense` bij >16 materialen. `src/serialize.rs` naar versie-2 byte-stabiel formaat (header 15 B + palette-len + palette + packed / dense).
  - Twee test-bugs zelf gevonden en gecorrigeerd tijdens TDD (verkeerde `HEADER_LEN=12`→15; verkeerde `flat()`-orde in bitpacking-test) — implementatie was correct.
- Verificatie:
  - `cargo test --workspace`: 19 tests groen (voxel-core 13: 1 lib-unit + 5 spike_s01 + 7 spike_s01_hardening; voxel-mesher 6). S-02 ongewijzigd groen (gebruikt alleen `chunk.get()`).
  - `cargo test -p voxel-core --features proptest`: property-roundtrip groen.
  - `cargo clippy -p voxel-core --all-targets`: schoon op nieuwe code; 1 overgebleven style-warning in bestaande S-01 spike (`i64->i64` cast) buiten hardening-scope gelaten (geen drive-by refactor).
  - Renderer-agnostisch (ADR-0002): geen godot/bevy/wgpu in `voxel-core/Cargo.toml`; geen renderer-import in code. Bevestigd.
- Geheugenbudget: `PalettePacked` = N³/2 bytes (16.384 B voor 32³) vs `Dense` = N³ bytes (32.768 B); `Uniform` = 0. Voldoet aan "≤4 B/actieve voxel"-target van ADR-0001 bij gemiddeld <16 materialen.
- Verplichte terugstap: plan-alignment OK (canoniek plan + ADR's 0001–0003 intact); budgetguard project-key spent $27,5299 (onder €30 → paid blijft dicht; ver onder $36 stop). Geen drift t.o.v. canoniek plan. Geen betaalde calls deze sessie; alles lokaal + gratis `:free`.
- Planstatus: aligned. Volgende stap (autonoom): S-03 renderer/camera spike (wgpu headless of software raster) om mesher-output zichtbaar te maken — nu veilig mogelijk omdat voxelopslag waterdicht is.
- Commit + push naar origin main (grens A) volgt na deze terugstap.

## 2026-07-15 — S-03 software-raster spike voltooid (strict TDD) + verplichte terugstap na uitvoeringsstap 3
- Autonome keuze: S-03 software-raster (puur Rust, géén GPU) boven wgpu/Vulkan — laagste risico, geen driver/GPU-afhankelijkheid; bewijst de `Chunk -> mesh -> beeld`-keten end-to-end vóór de zware renderer-keuze (ADR-0002, Fase 2). Sluit aan op gebruikersvraag over "taal/Vulkan".
- Strict TDD:
  - ROOD: `crates/voxel-render` crate aangemaakt (workspace member), `spike-s03-render.md` plan geschreven, `tests/spike_s03.rs` (3 failing tests: leeg/één voxel/volle chunk) — `cargo test -p voxel-render` faalde met "no `Camera`/`render_scene` in root".
  - GROEN: `camera.rs` (perspectief, yaw/pitch/distance/fov) + `render.rs` (look-at view-proj, z-buffer, per-normaal Lambert-shading) + `examples/demo.rs`. Eén echte bug gevonden/corrigeerd: projectie had een tekenfout in de view/proj-conventie (clip-w negatief voor voorliggende geometrie) → herschreven naar één consistente +z-forward (DX-style) conventie.
  - Tweede fix: test-fixture plaatste de "één voxel" op chunk-hoek (0,0,0), ver buiten het camerabrandpunt (chunk-centrum 16,16,16) → vaak buiten beeld. Vastgezet op chunk-centrum (16,16,16); rasterizer was correct.
- Verificatie:
  - `cargo test --workspace`: 25 tests groen (voxel-core 13, voxel-mesher 6, voxel-render 3, doc/unit).
  - `cargo run --example demo -p voxel-render`: `demo.png` (256x256, 10654 niet-achtergrondpixels) gegenereerd.
  - Visuele verificatie via vision: herkenbare 3D voxel-scène (groene grondslab, twee bruine pilaren, grijze beacon), correcte projectie + shading. Bewijst `greedy_mesh`-output is zichtbaar.
  - Renderer-agnostisch (ADR-0002): `voxel-render` gebruikt alleen pure-Rust `image`-crate; géén godot/bevy/wgpu import in `voxel-core`/`voxel-mesher`. Bevestigd.
- Onafhankelijke review: subagent gestart (deleg_6881a331) op S-01-hardening-diff; bevindingen worden verwerkt in een aparte log-entry zodra beschikbaar.
- Verplichte terugstap: plan-alignment OK (canoniek plan + ADR's 0001–0003 + S-03 plan intact). Budgetguard project-key spent $27,5299 (onder €30 → paid blijft dicht; ver onder $36 stop). Geen drift; geen betaalde calls (alles lokaal + gratis `:free`). Taakuitbesteding via subagent bespaart eigen `hy3:free` context/rate-limit.
- Volgende stap (autonoom): S-03 uitbreiden (demo-scenario's/schaal) of S-04 client-shell/input (wgpu na Fase-2 benchmark).
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — onafhankelijke review S-01-hardening verwerkt (subagent deleg_6881a331)
- Subagent (leaf, :free model) reviewde commit 20baac0 met focus op correctheid, niet stijl.
- **Conclusie reviewer: geen code-bugs.** Alle kernmechanismen correct bevonden:
  - (1) nibble-orde consistent tussen `write_nibble`/`get`/`promote_to_dense` (even flat=low, oneven=high).
  - (2) promotie PalettePacked→Dense bewaart ALLE voxelwaarden (decodering vóór overschrijven; untoucht = baseline).
  - (3) v2 byte-stabiel formaat round-tript voor alle drie states (header 15 B, offsets consistent).
  - (4) edge cases: chunk-boundary (flat 32767), materiaalindex 15 vs 16 (17e distincte triggert Dense, geen 5-bit overflow), high-nibble voxel, uniform zero-storage — allen correct.
- **Twee test-gaps** gerapporteerd (geen bugs): (a) geen test voor high-nibble bij chunk-boundary (flat 32767); (b) geen geïsoleerde "promotie bewaart alle waarden" + multi-step post-promotion edit.
- **Verholpen**: twee tests toegevoegd aan `spike_s01_hardening.rs` — `boundary_high_nibble_round_trips` en `promotion_preserves_all_values_then_continues_edits`. Eerste test-versie had zelf een foute aanname (telde baseline(0) niet mee als distinct → 17 ipv 16); gecorrigeerd naar baseline+15 = 16 distinct → PalettePacked, 17e = Dense. Review-gaps nu gedicht.
- Verificatie: `cargo test --workspace` 27/27 groen (voxel-core 15, voxel-mesher 6, voxel-render 3).
- Status: aligned. Geen drift; geen betaalde calls.

## 2026-07-15 — S-04 deterministische worldgen spike voltooid (strict TDD) + verplichte terugstap na uitvoeringsstap 3
- Autonome keuze: S-04 worldgen (canoniek plan §4 Fase-1 deliverable: "deterministische worldgen met seed"). Geen nieuwe architectuurbeslissing nodig; valt binnen bestaande ADR's. Client-shell (Godot vs Bevy/wgpu) expliciet NIET gestart — dat is de Fase-2 benchmark-gate.
- Strict TDD:
  - ROOD: `crates/voxel-worldgen` crate (workspace member), `spike-s04-worldgen.md` plan, `tests/spike_s04.rs` (5 failing tests: determinisme, seed-verschil, chunk-grens, niet-leeg, laagstructuur) — `cargo test -p voxel-worldgen` faalde met "unresolved import generate_chunk".
  - GROEN: `generate_chunk(coord, seed)` — seeded value-noise heightmap op wereld-X/Z (hash + bilineaire interpolatie), grass(2)/dirt(1)/stone(3) lagen. Determinisme + grensoverschrijdende continuïteit volgen gratis omdat hoogte een pure functie van wereld-X/Z is. Renderer-agnostisch (alleen voxel-core).
  - Test-fout tijdens TDD gecorrigeerd: `chunk_boundary_continuous` vergeleek ten onrechte chunk A local-x=31 (wereld-X=31) met chunk B local-x=0 (wereld-X=32) op gelijkheid — dat zijn opeenvolgende wereldkolommen, géén zelfde kolom. Hernomen naar de correcte continuïteitseis: hoogtestap over de chunk-grens mag niet groter zijn dan binnen-chunk-stappen. Generator was correct.
- Verificatie:
  - `cargo test -p voxel-worldgen`: 5/5 groen.
  - `cargo test --workspace`: 32 tests groen (voxel-core 15, voxel-mesher 6, voxel-render 3, voxel-worldgen 5).
  - `examples/demo_worldgen.rs` → `demo_worldgen.png` (320x320, 28668 niet-achtergrondpixels). Visueel geverifieerd: rollend, continu oppervlak, gras/dirt/stone-lagen, geen scheuren.
  - Renderer-agnostisch (ADR-0002): `voxel-worldgen` depends alleen op `voxel-core`; geen godot/bevy/wgpu.
- Verplichte terugstap (na 3e uitvoeringsstap): plan-alignment OK (canoniek plan + ADR's 0001–0003 + S-04 plan intact). S-01-hardening review ingevlogen en verwerkt (9056d81). Budgetguard project-key spent $27,5299 (onder €30 → paid blijft dicht; ver onder $36 stop). Geen drift; geen betaalde calls (alles lokaal + gratis `:free`).
- Volgende stap (autonoom): S-05 multi-chunk wereld/streaming-basis, of S-04 uitbreiden (biomes/macro-micro per plan §2.1). Client-shell-keuze blijft Fase-2 gate.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-05 multi-chunk world-store spike voltooid (strict TDD) + verplichte terugstap na uitvoeringsstap 3
- Autonome keuze: S-05 world-store (opmaat naar Fase 3 "asynchrone chunkgeneration/meshing" + "save/load seed+edits" uit canoniek plan). Binnen bestaande ADR's; geen client-shell-beslissing nodig.
- Strict TDD:
  - ROOD: `crates/voxel-world` crate (workspace member), `spike-s05-world.md` plan, `tests/spike_s05.rs` (4 failing tests) — `cargo test -p voxel-world` faalde met "unresolved import World".
  - GROEN: `World { chunks: HashMap<ChunkCoord,Chunk>, dirty: HashSet, seed }` met `new`, `get_or_generate` (cached, deterministic), `chunk_at`, `set_voxel` (world→chunk mapping via `ChunkCoord::from_world`+`LocalVoxel::from_world`, markeert dirty), `dirty_chunks`/`take_dirty`. Edits overleven generatie (entry().or_insert_with).
  - Twee bugs tijdens TDD gecorrigeerd: (a) borrow-checker: `get_or_generate` gaf `&Chunk` → conflicterende borrows bij meerdere calls; omgezet naar by-value return (clone). (b) demo-toren stond op wereld-Y 5..9, bedolven onder terrain (~Y20+); verplaatst naar surface+1..surface+4 (surface gemeten uit chunk) → zichtbaar.
- Verificatie:
  - `cargo test --workspace`: 36 tests groen (voxel-core 15, voxel-mesher 6, voxel-render 3, voxel-worldgen 5, voxel-world 4).
  - `render_world` toegevoegd aan voxel-render (offset per chunk naar wereldruimte, géén harde voxel-world dep → renderer-agnostisch). `examples/demo_world.rs` → `demo_world.png` (384x384, 82955 niet-achtergrondpixels): 2x2 chunks naadloos + metalen toren-edit zichtbaar (visueel geverifieerd).
  - Ongebruikte import (`LocalVoxel` in render.rs) verwijderd; warnings schoon op nieuwe code.
- Verplichte terugstap (na 3e uitvoeringsstap): plan-alignment OK (canoniek plan + ADR's 0001–0003 + S-05 plan intact). S-01-hardening review verwerkt (9056d81). Budgetguard project-key spent $27,5299 (onder €30 → paid blijft dicht; ver onder $36 stop). Geen drift; geen betaalde calls (alles lokaal + gratis `:free`). ADR-0004 (client-shell) loopt als subagent (deleg_4c7b3b6d); wordt als aparte log-entry + ADR-bestand verwerkt zodra binnen.
- Volgende stap (autonoom): S-06 edit/place-remove tool + revisie, richting werkende vertical slice.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-06 edit-tool + S-07 persistence voltooid (strict TDD)
- S-06 `voxel-edit` (Fase 3 opmaat, canoniek plan §3.2 edit-events): `Edit { world, old, new, actor, tick, revision }`, `EditLog` (append-only, monotoon), `EditTool::place/remove` die op `World` schrijven én loggen. 4 failing tests → groen (old-capture, monotone revisies, tool+log update, replay-reproductie op verse wereld). `old` is de werkelijke voorafgaande waarde → correcte undo/replay.
- S-07 `voxel-persist` (Fase 3/5 opmaat, canoniek plan §3.6 + §4 Fase-3 gate "save/reload behoudt alle edits"): eigen binair formaat (magic `VWL1` + seed u32 + edit_count + per-edit velden), `save_world`/`load_world`/`PersistError`. Alleen seed+edits opgeslagen (basis reproduceerbaar → "procedurele basis + append-only editlog"). 3 failing tests → groen (round-trip reproductie incl. basis-terrain, log-behoud met revisies, corrupt/truncated → `Err` geen panic).
- Verificatie: `cargo test --workspace` 40 tests groen (voxel-core 15, voxel-mesher 6, voxel-render 3, voxel-worldgen 5, voxel-world 4, voxel-edit 4, voxel-persist 3). `examples/demo_persist.rs` → `demo_persist.png` (384x384, 82955 px); visueel geverifieerd: toren staat er nog ná save→load (volledige persistence-keten).
- Bug tijdens TDD: test leende geladen wereld immutabel maar riep `get_or_generate` (mut) — `mut` toegevoegd. `World::seed()` toegevoegd (nodig voor persist). Geen core-logica-fout.
- Renderer-agnostisch (ADR-0002): voxel-edit + voxel-persist dependeren alleen op voxel-core/world/edit; géén godot/bevy/wgpu.
- ADR-0004 (client-shell) loopt nog als subagent (deleg_4c7b3b6d); apart te verwerken.
- Volgende stap (autonoom): S-08 spelercontroller + camera, S-09 headless dedicated server → runnable slice.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-08 spelercontroller + ADR-0004 (client-shell) voltooid
- S-08 `voxel-player` (Fase 3, canoniek plan §4 "eenvoudige spelercontroller" + §3.4 physics): `Player { pos, yaw, on_ground }`, `PlayerController::step` met axis-separated collision, sub-stepping (MAX_SUB_DT=0.02) tegen tunnelen, en `resolve_floor_y` die de speler exact op de hoogste solid voxel onder de voet zet. 4 failing tests → groen (vooruit-beweging langs yaw, muur-blokkade, gravity→rust op vloer én op echt terrain, langs-muur-glijden). `examples/demo_player.rs` → `demo_player.png`: speler zakt van y=40 naar y=13.9 (rust op terrain, on_ground=true); visueel geverifieerd als first-person grond-niveau view.
- TDD-lessen (code was uiteindelijk correct; test-assumpties fout): (a) voxel op wereld-Y=0 vult `[0,1)` → speler rust met center op `1.0+HALF[1]`=1.9, niet 0.9; (b) muur op x=10, half-x=0.3 → dichtstbijzijnde stabiele x=9.7, niet <9.5. (c) `flat_world`-test moest terrain boven de vloer weg-graven (set_voxel genereert seeded terrain). Geen core-logica-bug.
- ADR-0004 (client-shell): subagent-dossier (deleg_4c7b3b6d, gratis model, géén GPU-benchmark) → aanbeveling **B: Rust + Bevy/wgpu**. Gronden: pure-Rust core (ADR-0002) wordt native geconsumeerd (geen FFI/copy), client+headless server delen één Rust-workspace (ADR-0003), wgpu/Vulkan-backend op RTX 4080 is volwassen, volledige eigendom van camera/input/render, één debugger. Godot GDExtension verworpen voor eerste slice (C-ABI-shim + codebase-split). Status: Proposed; Fase-2 benchmark-gate (B-06 determinisme-replay, B-07 headless 2–8 client soak, FPS op 1 km²) blijft verplicht vóór lock-in. Geschreven naar `docs/architecture/adr/0004-client-shell.md`.
- Verificatie: `cargo test --workspace` 44 tests groen (voxel-core 15, voxel-mesher 6, voxel-render 3, voxel-worldgen 5, voxel-world 4, voxel-edit 4, voxel-persist 3, voxel-player 4). Renderer-agnostisch (ADR-0002): voxel-player dependeert alleen op voxel-core/world.
- Volgende stap (autonoom): S-09 headless dedicated server (geen GPU), dan runnable vertical slice.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-09 headless dedicated server voltooid → VERTICAL SLICE BEREIKT
- S-09 `voxel-server` (Fase 3/4, canoniek plan §3.5 server-authoritative + §4 "headless dedicated server (geen GPU)", ADR-0003): `Server { world, log, players }`, `tick(dt)` stept elke `PlayerController` tegen de gedeelde `World` (headless, géén renderer/GPU), `apply_edit(actor, wv, mat)` update wereld + logt in `EditLog`, `material_at` leest de gedeelde authoritative view. 4 failing tests → groen (spelers vallen op terrain, edit zichtbaar voor alle spelers, determinisme zelfde seed+edits→identieke wereld, headless compileert zonder renderer).
- `voxel-server` dependeert alleen op voxel-core/world/edit/player — **geen voxel-render** (bewijs: de headless test compileert niet als een renderer in de graaf zat). `examples/headless_server.rs` draait 600 ticks: 3 spelers spawnen op y=40, vallen naar on_ground op terrain (y≈19.9/22.9), speler 1 loopt vooruit (x 40→44.7), beacon-edit (mat 4) zichtbaar in gedeelde wereld. Output: "server ran 600 ticks headless (no GPU, no renderer)".
- **VERTICAL SLICE BEREIKT (S-01..S-09, allen strict TDD):** core→mesher→render→worldgen→world→edit→persist→player→server. De engine heeft nu een runnable, GPU-vrije authoritative server + persistente, bewerkbare, deterministische wereld + first-person spelercontroller. Client-shell (ADR-0004: Rust+Bevy/wgpu) is gekozen maar de Fase-2 benchmark-gate (B-06/B-07/FPS) loopt later; de headless server is alvast het runbare artifact voor de gebruiker.
- Verificatie: `cargo test --workspace` 48 tests groen (voxel-core 15, voxel-mesher 6, voxel-render 3, voxel-worldgen 5, voxel-world 4, voxel-edit 4, voxel-persist 3, voxel-player 4, voxel-server 4). Run-instructie toegevoegd in `README.md` (headless server runnen, demo-PNGs genereren).
- Volgende (autonoom, Fase 4): netwerk/protocol-laag voor 2–8 spelers multiplayer; daarna de echte Bevy/wgpu-client (ADR-0004). Gebruikersvolmacht: doorbouwen tot werkend product — slice is het eerste runbare checkpoint.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-10 GPU-renderer (wgpu/Vulkan) voltooid → ENGINE DRAAIT OP DE GPU
- S-10 `voxel-gpu` (Fase 2 client-shell opmaat, ADR-0004, gebruikerseis "engine op de GPU"): `probe` bevestigt wgpu 0.17.2 init + offscreen texture-readback naar PNG op de RTX 4080 (Vulkan). `renderer` neemt `greedy_mesh`-triangles → GPU vertex-buffer; WGSL-shader doet per-normaal directionele belichting + warme fog/atmosfeer + warme materiaal-tinten (Lay of the Land-vibe, ~90% ref + John Lin ~10% per gebruiker). `examples/gpu_world.rs` genereert 2×2 terrain (16.270 triangles) en rendert naar `gpu_world.png`.
- Belangrijke TDD/debug-lessen: (a) wgpu 0.18.0 is yanked; 0.19/0.20 hebben API-drift (RenderPassColorAttachment `ops`, DeviceDescriptor zonder `required_`, `TextureUsages`/`BufferUsages` meervoud, `draw` neemt ranges, `map_async(mode, callback)` niet-async, uniform `vec3` 16-byte alignment). Gepind op 0.17.2 en API handmatig tegen de registry-source geverifieerd. (b) Handgeschreven matrix-wiskunde (perspective/look_at/mul) bleef fout (column-major swap → geometry buiten frustum, lege PNG). Vervangen door `glam` (Mat4::look_at_rh / perspective_rh) → direct correct. (c) WGSL module-level `const array` mag niet met dynamic index; vervangen door `mat_tint(id)` if-ladder. (d) Depth-clear `1.0` moest `1.0_f32` zijn (f64 vs f32 inference) en `depth_ops` is `Option<Operations<f32>>`.
- **EERSTE GPU-RENDER VAN DE ENGINE**: voldoet aan de gebruikerseis (GPU, niet software-raster). Nog offscreen-PNG (geen winit-venster), geen echte Bevy-integratie — dat is Fase 4. RTX 4080 / Vulkan bevestigd werkend.
- Verificatie: `cargo test --workspace` groen (incl. voxel-gpu). `gpu_world.png` visueel geverifieerd (vision-tool): "3D voxel terrain, shaded hills/surfaces with depth, warm lighting, earthy green/brown voxels, fog". `probe.png` eveneens geverifieerd.
- Volgende (Fase 4, autonoom): winit-venster + input-loop (interactieve GPU-client), dan netwerk/protocol (2–8p multiplayer) bovenop de headless server, dan echte Bevy/wgpu-integratie + Fase-2 benchmark-gate (B-06/B-07/FPS) vóór ADR-0004 lock-in.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 — S-11 audit-hardening + roadmap-verfijning (terugstapcontrole na 3e stap)
- Aanleiding: gebruikersopdracht "controleer de gehele engine + repo, doe onderzoek naar vervolg, verbeter roadmap en workflows".
- Verificatie vooraf: cargo test --workspace 48/48 groen; headless_server (600 ticks) + gpu_world (16.270 tris) echt gedraaid; gpu_world.png visueel gecontroleerd; Python-guards 3/3 OK.
- Repo-hygiëne: `.hermes/reports/` uit git (runtime-artifacts), clippy --fix toegepast (commit 7547f7e).
- Onafhankelijke audit (gratis model) + SOTA-onderzoek parallel gedelegeerd (deleg_b5c57770). Alle 6 hoge bevindingen door implementer in de bron geverifieerd — géén enkele claim blind overgenomen; alle bleken correct.
- S-11 fixes onder strict TDD (9 nieuwe tests RED→GREEN; workspace 57/57): mesher-planes/winding (+culling aan), serialize v3 (i64, nibble-validatie), player anti-tunneling + footprint-floor, server tick-orde-determinisme, atomaire saves, gpu-tints/fog/backends. Commit 1a8e093.
- Plan-alignment: canoniek plan ongewijzigd van koers (vertical slice → interactieve client → gates). ROADMAP.md geconcretiseerd met bron-onderbouwd advies (docs/research/2026-07-15-sota-advies-vervolg.md): wgpu/winit-upgrade vóór interactieve client, binary greedy meshing, chunk-key (x,y,z,lod), AO+CSM in Fase 3, Gaffer-netcode in Fase 4, LOD-beslissing data-gedreven na 1 km²-benchmark. Fase 2b (technische schuld) toegevoegd.
- Workflow-les (vastgelegd in ROADMAP.md): count-based tests misten de 2 ernstigste bugs; voortaan golden-tests op exacte waarden + malformed-input tests per spike; onafhankelijke audit na elke fase.

## 2026-07-15 (sessie 2) — S-12 deel 1 (Fase-2b #1 clone-fix)
- Bronnen gelezen: EVALUATION_AND_HANDOFF.md (B), PROJECT_STATE.md, ROADMAP.md, SOTA-advies. Status bevestigd: S-01..S-11 af, 57/57 groen.
- Uitvoeringsstappen 1-3: canonieke bronnen gelezen; deps geïnventariseerd (voxel-gpu op wgpu 0.17, géén winit); wgpu/winit-upgrade-onderzoek gedelegeerd aan gratis model (loopt); S-12 deel 1 uitgevoerd + gecommit.
- S-12 deel 1 = Fase-2b #1 uit roadmap: `World::material_at(&mut self, WorldVoxel) -> MaterialId` toegevoegd (geen chunk-clone); `voxel-player::solid_at` gebruikt nu die reader i.p.v. `get_or_generate().get()` (audit #12: 32 KB-clone per voxel-sample in collision). Strict TDD: failing test eerst (RED: `material_at` bestond niet), daarna GREEN. 58/58 tests groen.
- Terugstap-check (na 3e stap): géén drift vs canoniek plan; roadmap/noordster ongewijzigd; geen architectuurwijziging. Commit d8e34cc op origin/main.
- Volgende: wacht wgpu/winit-onderzoek af, dan Fase 2a upgrade-plan schrijven (wgpu>=22 + winit 0.30 ApplicationHandler) onder TDD.

## 2026-07-15 (sessie 3) — S-12 deel 2 (Fase 2a wgpu/winit-upgrade)
- Bronnen: crates.io live API (wgpu laatst stabiel = 30.0.0, winit = 0.30.13; 0.31 is beta),
  lokaal gekloonde wgpu-30 broncode voor exacte signatures. Migratieplan subagent was op
  0.22-0.24 gebaseerd -> 0.17->30 is 13 major-versies, dus dieper dan het plan dekte.
- Wijzigingen (alleen voxel-gpu):
  - Cargo.toml: wgpu "0.17"->"30", winit "0.30" toegevoegd.
  - renderer.rs: InstanceDescriptor(5 velden), request_adapter->Result, request_device(1 arg),
    PollType::wait_indefinitely, TexelCopy* structs, StoreOp::Store, VertexState Option-wrap +
    compilation_options, bind_group_layouts: &[Some(&bgl)], immediate_size, DepthStencilState
    Option<>, multiview_mask + cache, @interpolate(flat) WGSL.
  - NIEUW example gpu_window: ApplicationHandler-event-loop, surface-render, WASD+muis-look
    free-fly camera; deelt GpuScene-pipeline met offscreen-pad.
  - KRITIEKE FIX: GpuScene::new_for_surface nam eigen device (surface/device mismatch ->
    "DeviceId bestaat niet" panic). Nu device/queue van dezelfde adapter als surface.
  - DPI-clip: surface binnen 2048px (RTX 4080 + ~3x scale -> 3048px -> Surface::configure panic).
- Verificatie: cargo build --workspace --examples 0 errors; cargo test 58/58 groen (ongewijzigd);
  gpu_world offscreen PNG intact (16.270 tris, visueel OK); gpu_window startup-log bevestigd
  ("GPU scene initialized (Bgra8UnormSrgb)"), 12s runtime zonder crash.
- Commit + push naar origin main (grens A) volgt.
- Volgende: Fase-2 benchmark-gate (B-06/B-07/FPS op 1 km²) vóór ADR-0004 lock-in, dan
  chunk-streaming (S-12 deel 3).

## 2026-07-15 (sessie 4) — S-12c deel 1: Fase-2 FPS-benchmark-gate op 1 km²
- Bronnen gelezen: git log/status (schoon, S-12b laatste commit), PROJECT_STATE.md (was
  achtergebleven bij "Fase 0 / S-11" — plan-drift gecorrigeerd naar S-12b), ROADMAP.md,
  alignment-log, voxel-gpu renderer.rs/gpu_window.rs/gpu_world.rs (A-PI voor benchmark).
- Verificatie vooraf: cargo test --workspace **58/58 groen**, cargo build --workspace --examples
  schoon (gpu_window.exe gebouwd). PROJECT_STATE.md bijgewerkt + gepushed (master->origin/main,
  branch heet lokaal `master` met upstream origin/main).
- S-12c deel 1 = Fase-2 benchmark-gate: `GpuScene::render_triangles` (offscreen, géén
  readback/PNG) toegevoegd als meetbare frame-A-PI; nieuw `examples/gpu_bench.rs` met
  view-distance chunk-streamer + frametime-meting (p50/p95/p99) naar `bench_1km2.json`.
  Strict: géén wijziging aan bewezen S-10/S-12b renderer-gedrag.
- Uitslag op RTX 4080 (32×32 chunks = 1,05 km², radius 8, 300 frames, 1024×768):
  **avg 8,8 FPS**, p50 129 ms / p95 141 ms / p99 151 ms, ~1,25 M zichtbare tris/frame.
  **Gate NIET gehaald.** Oorzaak = scene-samenstelling (geen frustum-culling, geeen
  distance-budget, per-frame VBO), niet de renderer-pipeline.
- Analyse + besluit vastgelegd in `docs/benchmarks/2026-07-15-fase2-fps-1km2.md`.
  ADR-0004 lock-in uitgesteld totdat de scene efficienter is (frustum-culling + budget).
- ROADMAP + PROJECT_STATE bijgewerkt. Geen drift vs canoniek plan; géén architectuurwijziging.
- Volgende (S-12c deel 2): frustum-culling + triangle-distance-budget + buffer-pooling in de
  streamer, dan hermeten; pas bij gezonde FPS → ADR-0004 naar Accepted. Reproduceerbaarheid
  van de bench nog te bevestigen met een 2e run (<10% afwijking).
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 (sessie 5) — S-13: micro-voxel resolutie 12,5 cm (ADR-0005)
- Gebruikersbesluit: voxels **tussen 9,5–13,5 cm, streven 12,5 cm** (Lay of the Land / Voxtopolis
  / John Lin / Tantan-richting). Voorheen impliciet **1 voxel = 1 m** (chunk=32 m).
- ADR-0005 geschreven + Accepted (autonoom): 1 voxel = 0,125 m (12,5 cm, 1/8 m macht-van-twee,
  nette schaal-wiskunde), chunk blijft 32³ voxels → **chunk = 4 m** (was 32 m). Wereld-coördinaten
  blijven integer-in-voxels (geen coords-wijziging, Euclidean-divisie intact). 1 km² = **62.500 chunks**
  (was 1.024). Keuze: klein chunk (4 m) i.p.v. 32 m/256³ (RAM-explosie ~2,4–4,6 GB/chunk-set).
- Strict TDD: `spike_s13.rs` (2 failing tests: VOXEL_SIZE_M, one_km2_is_62500_chunks) →
  `voxel-core/src/coords.rs` kreeg `VOXEL_SIZE_M: f32 = 0,125` + `chunk_m_size()`. Rood bevestigd
  (unresolved imports), daarna Groen (2/2). Opmerking: patch verwijderde tijdelijk `use std::ops::Div;`
  (nodig voor euclidean_div) — teruggezet, géén regressie.
- Camera's naar 12,5 cm-schaal: gpu_window/gpu_world/bench eye ~6,25 m boven ~4 m terrain
  (was 55 m), view-radius default 60 (was 8). Bench anchor nu binnen wereld-grens (was buiten →
  0 frames); radius 3 run = 534 FPS, 52k tris, géén crash.
- `gpu_world.png` geregenereerd op 12,5 cm (16.270 tris, GPU-rendered). Vision-check: terrain
  rendert correct (groen/dirt/stone, shading, depth) maar op 2×2-chunk-zoom oogt het nog blocky
  — het "micro"-effect komt pas bij first-person (S-12c deel 2) of grotere scenes (1 km² vereist
  frustum-culling). Geen bug; zoom/scale-afhankelijk.
- Verificatie: cargo test --workspace **60/60 groen** (58 + 2 nieuw S-13), géén regressie in
  coords/worldgen/physics. Impressie-set (23 PNG/JPG) verzameld in docs/research/impressie-microvoxel/.
- ROADMAP + PROJECT_STATE + alignment-log bijgewerkt. Geen drift vs canoniek plan.
- Volgende: S-12c deel 2 (frustum-culling + budget + buffer-pooling) voor de 1 km²-FPS-gate op
  12,5 cm-schaal; worldgen-noise fijner (advies #6); first-person player-camera op 12,5 cm.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 (sessie 7) — Mijlpaal 1 + 2: space-crash + lag fix (S-13b / S-12c deel 2)
- Gebruiker rapporteerde: (a) client crasht bij spatie, (b) erg laggy. Nieuwe eis: best-of-best,
  enorme FPS, 4K+ textures; roadmap volgen; onderzoek waar nodig; verslag per mijlpaal; géén
  code-versoepeling.
- **Mijlpaal 1 (space-crash):** rootcause = `get_current_texture()` gaf `Lost`/`Outdated` bij
  focus-wissel (spatie → OS window-snap), oude code herstelde surface nooit → crash/hang. Fix:
  bij `Lost`/`Outdated` herconfigureert client surface op laatst bekende grootte + skipt frame;
  `Timeout`/`Occluded` → skipt. Gepushed (c1e1759..3d2157e).
- **Onderzoek (3 subagents, gratis :free modellen):** wgpu 0.30 FPS-strategie (frustum-culling,
  buffer-pooling, rayon-streaming, indirect-draw), 4K-texture-aanbeveling (texture-arrays per
  materiaal, triplanar, PBR, BCn-compressie op 4080S), spacebar-fix-analyse. Allen naar
  docs/research/ verplaatst. Bevestigen: P0 (per-frame alloc elimineren) = ~5–20×, P1
  (frustum-culling) = ~3–10×, P0+P1 samen ~10–30×.
- **Mijlpaal 2 (lag fix P0+P1):** `Frustum`-struct in `renderer.rs` (6 planes uit view_proj,
  AABB-test) + unit-test (Rood→Groen). GPU buffer-pooling: één herbruikbare VBO in `GpuScene`,
  `write_buffer` i.p.v. per-frame `create_buffer_init`. Client past frustum-culling per chunk toe.
  **61/61 tests groen** (60+1 nieuw). Bench 1 km²: **8,8 → 15,8 avg FPS** (p50 129→55 ms = 2,3×).
- **Mijlpaal 3 (P3, non-blocking rayon-meshing):** dedicated `rayon::ThreadPool` (1 core vrij)
  doet `generate_chunk`+`greedy_mesh` off-thread; `crossbeam_channel` stuurt kant-en-klare
  `Vec<Triangle>` + `generation` terug; `render_frame` vult `mesh_cache` binnen per-frame
  `UPLOAD_BUDGET` (4), discardt stale via gen-counter. Strict TDD: unit-test
  `mesh_chunk_offthread_streams_result` (Rood→Groen). **62/62 tests groen** (60+2 nieuw).
  Plan in docs/research/2026-07-15-milestone3-rayon-meshing.md.
- **Mijlpaal 4 (4K-texture-system)** staat nog open (onderzoek klaar: docs/research/2026-07-15-texture-4k-aanbeveling.md).
- **Hotfix wit scherm (2026-07-15):** na Mijlpaal 3 rapporteerde de gebruiker een wit scherm.
  Root cause: in de eerste frames zijn alle chunks `pending` (async workers nog niet klaar) →
  `tris.is_empty()` → `return` vóór `surface.get_current_texture()` → de surface wordt nooit
  gecleared (clear-color is [0.62,0.66,0.74] grijsblauw, dus wit = nooit renderen). Fix:
  "never go white" guard — `nearest_visible_chunk` (vrije fn, eigen frustum, géén `&self` borrow
  conflict) mesht de dichtstbijzijnde zichtbare chunk **synchroon** als frame-1 fallback; daarna
  neemt de async rayon-pool over. Ook de `tris.is_empty()`-return verwijderd (surface altijd
  gecleard). Unit-test `drained_mesh_lands_in_cache_after_one_frame` (Rood→Groen) toonde eerst de
  bug (mesh landt niet in 1 sync drain) en bewijst nu het contract. **63/63 tests groen.** Gepushed.
- Geen drift vs canoniek plan/roadmap. Code niet versoepeld ondanks complexiteit.
- Commit + push naar origin main (grens A) volgt.

## 2026-07-15 (sessie 6) — S-13b: live micro-voxel client (erin rondkijken & lopen)
- Gebruikerswens: de GPU-client moet een wereld streamen die je **live kunt open + rondlopen**
  op de 12,5 cm-schaal (S-13 / ADR-0005). Voorheen was `gpu_window` een 2×2-chunk
  stub (4×4 m op 12,5 cm = een stipje) — niet wandelbaar.
- `gpu_window.rs` volledig herschreven naar **live chunk-streaming first-person client**:
  - Wereld = lazy (World genereert chunks on-demand), géén startup-2×2-limit.
  - Elke frame: chunks binnen `VIEW_RADIUS=24` (~96 m) van de camera worden gegenereerd +
    gemesht (cache), `tris` herbouwd, gerendered via `render_to_view`.
  - First-person spawn: terrain-top gemeten bij spawn-chunk → eye ~3 voxels (37,5 cm) erboven.
  - WASD free-fly (speed 0,8 m/s × 8 voxels/m = 6,4 voxels/s) + links-drag mouse-look
    (bestaande input behouden).
- Headless smoke-test `client_smoke.rs` (zelfde streaming-path, géén winit-venster) bevestigt:
  **120/120 frames gerendered, géén panic**, spawn terrain-top=28 voxels (~3,5 m), eye 3,88 m.
- Build: `cargo build --release --example gpu_window` schoon (geén errors). README bijgewerkt
  (status → S-13, 60 tests, GPU-client run-instructie toegevoegd). ROADMAP + alignment-log
  + PROJECT_STATE bijgewerkt. Geen drift vs canoniek plan.
- **Verificatie-limiet:** de interactieve venster-ervaring (erin kijken/lopen) is iets de
  **gebruiker** zelf moet doen op zijn RTX 4080-pc met scherm (`cargo run --release
  --example gpu_window -p voxel-gpu`). Headless kon ik alleen de streaming-render-path
  bevestigen (smoke-test), niet het visuele venster.
- Volgende: S-12c deel 2 (frustum-culling) voor de 1 km²-FPS-gate op 12,5 cm; daarna
  worldgen-noise fijner (advies #6) zodat terrain op micro-schaal écht detail toont.
- Commit + push naar origin main (grens A) volgt.
