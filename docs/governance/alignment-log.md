# Plan-alignment log

Hier wordt iedere verplichte terugstap en iedere materiële koerscorrectie vastgelegd.

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
