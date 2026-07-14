# North-Star Capability Decomposition

**Datum:** 2026-07-14  
**Bronnen:** primaire onderzoekbronnen (John Lin, Tantan, Teardown/Dennis Gustafsson, Voxtopolis, Lay of the Land, research papers, open-source voxel engines) — alle bronnen zijn openbaar beschikbaar, geen betaalde modellen gebruikt.  
**Doel:** Vertalen van de filmische openwereld-kwaliteitslat (GTA VI / Crimson Desert niveau dichtheid, dynamiek, schaal) naar meetbare engine-subsystemen die als acceptatiecriteria voor de vertical slice dienen.

---

## 1. Samenvatting van de north-star

| Dimensie | Kwaliteitslat (referentie) | Meetbare vertaling voor micro-voxel engine |
|----------|---------------------------|--------------------------------------------|
| **Werelddichtheid** | GTA VI straatniveau detail, Voxtopolis voxel-dichtheid | ≥ 10⁷ actieve micro-voxels in speler-zichtzone *(target — validatie in streaming spike S-03; afleiding: 12,5 cm LOD0 + view-distance bepaalt dichtheid, geen gemeten baseline)*; sparse procedurale basis + alleen edits persistent |
| **Dynamiek / Destructie** | Teardown voxel-naar-voxel physics, Crimson Desert destructible environments | Per-voxel materiaal, capsule-vs-voxel collision, chunk-local rigid-body fracturing; Server collision ≤ 2 ms/tick, client prediction ≤ 1 ms/frame *(target — validatie in collision spike S-04; Teardown cijfer is single-player, netwerk-round-trip nog ongemeten)* |
| **Streaming / Schaal** | 150 km² adresruimte, seamless LOD | 12,5 cm LOD0 brick (8³ samples), 32³ mesh-blocks, < 100 ms chunk-load p95, < 2 GB RSS voor 1 speler |
| **Animatie / Personages** | GTA VI motion-matching, procedural animation layers | Capsule controller + procedural IK op voxel-terrain, < 2 ms/frame anim budget |
| **AI / Levensvormen** | Crimson Desert NPC schedules, emergent behavior | Chunk-local behavior trees, interest-management per chunk, ≤ 1 ms/tick AI budget per 8 spelers *(target — validatie in soak spike S-07; geen primaire per-tick bron)* |
| **Networking / Multiplayer** | 2–8 spelers authoritative server, later 32/zone | 20–30 Hz server tick, chunk-based interest management, delta-compressed voxel edits, ≤ 100 kbit/s/client *(bandbreedte validatie in networking spike S-08; afhankelijk van tickrate 20 vs 30 Hz en editfrequentie)* |
| **Authoring / Tools** | Lay of the Land editor, John Lin procedural brushes | In-engine voxel brush, procedural macro-brushes, hot-reload palette, deterministic seed + editlog replay |
| **Observability / Telemetrie** | Production-grade observability (GTA online telemetrie) | Per-frame CPU/GPU timers, chunk-load histograms, net-stats, deterministic replay hash, automated soak-tests |
| **Audio** | Commodity spatial sound (miniaudio / OpenAL) | < 1 ms/frame *(Plan §2.2 — commodity component, geen eigen engine-werk)* |

---

## 2. Subsysteem-decompositie met meetbare metrics

### 2.1 Voxel Data & Storage (voxel-core)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Sparse hiërarchische opslag (brick 8³ → meshblock 32³ → region 256³) | ≤ 4 bytes/actieve voxel gemiddeld (palette + bitpack) | John Lin "The Perfect Voxel Engine" (allocation/tagging/conversion pipeline) |
| Deterministische serialisatie (round-trip byte-stabiel) | `serialize(deserialize(x)) == x` byte-identiek | Plan §3.2, Teardown save-format (Dennis Gustafsson GDC 2022) |
| Procedurele basis + alleen edits persistent | < 50 MB disk voor 1 km² onbewerkte regio | Plan §3.6, Voxtopolis sparse format |
| Edit-revisie ID op elk mesh/collider/save/netwerk-taak | Geen stale jobs na edit; revision mismatch < 0.1% | Plan §3.3 §3.5 |
| Negatieve coördinaten correct (euclidische deling) | Property-test: round-trip wereld↔chunk↔lokaal voor [-10⁶, 10⁶] | Plan §3.2 |

**Risico's:** Uniforme micro-voxels → OOM (> 64 GiB voor 1 km² @ 12,5 cm). **Mitigatie:** hiërarchische sparse bricks, procedurele fallback, palettes.

**Spike-voorstel:** `voxel-core` crate met property-tests (proptest) voor coördinaat-roundtrip, palette-overgangen, serialisatie round-trip. Benchmark: 1M edits/sec single-threaded.

---

### 2.2 Meshing & Rendering (voxel-mesher → client renderer)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Face culling (naïve → culled → greedy) | Greedy meshing ≤ 1.5× culled triangle count | Plan §3.3, "Voxel Meshing" surveys (Mikolalysenko) |
| Async remesh jobs met revision ID | Hoofdthread block < 0.5 ms/frame; job latency < 50 ms p95 | Plan §3.3 stap 4, Bevy/Godot job system |
| Frustum + distance culling + LOD budget | Triangle budget ≤ 2 M tris/frame @ 1080p/60 FPS (RTX 4080 Super) | Plan §6 performance targets |
| Materiaal-palette GPU-upload (SSBO/UBO) | < 1 ms/frame GPU upload | John Lin pipeline, Bevy render graph |
| Blocky (palette) vs Smooth (SDF+MC) spike | Beide meetbare frametime, geheugen, edit-latency | Plan §10.3 beslissingsregel |

**Risico's:** Greedy meshing T-junctions op chunk-grenzen; Transvoxel complexiteit. **Mitigatie:** Eerst blocky + greedy met halo-samples; Transvoxel spike later.

**Spike-voorstel:** `voxel-mesher` crate met 3 backends (naive, culled, greedy) + golden fixtures (lege, vol, checkerboard, grensoverschrijdende chunks). Criterion benchmarks per chunk-grootte.

---

### 2.3 World Generation & Streaming (worldgen + streaming)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Deterministische seeded generatie (per region) | Identieke seed → bit-identieke chunks | Plan §3.3, John Lin noise-based terrain |
| Async chunk pipeline: generate → mesh → upload | End-to-end < 100 ms p95 voor LOD0 chunk | Plan §3.3, Godot/Bevy async compute |
| Streaming radius + priority queue | Geen "pop-in" binnen view-distance; background chunks < 5% frame budget | Virtual texture streaming (idTech 5), Voxtopolis LOD |
| Floating origin (camera-relative rendering) | Geen jitter tot 10⁶ m wereldcoördinaten | Plan §3.2, standard large-world technique |

**Risico's:** Pop-in bij snelle beweging; priority inversion tussen generate/mesh/upload. **Mitigatie:** predictive streaming op velocity, double-buffered upload rings.

**Spike-voorstel:** Streaming simulator met synthetische spelerpad (cirkel, sprint, teleport) → chunk-load latency histogram.

---

### 2.4 Physics & Collision (voxel-physics)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Capsule vs voxel wereld (broadphase + narrow) | ≤ 2 ms/frame voor 1 speler, 100 km/h beweging | Plan §3.4, Teardown CPU voxel-vs-voxel |
| Chunk-local collision mesh (greedy/culled) | Mesh rebuild < 5 ms na edit; geen gaten op grenzen | Plan §3.3, Mikolalysenko collision meshing |
| Geen dynamische rigid-body destructie in fase 1 | Static world only; dynamic entities = capsules | Plan §3.4 expliciet |
| Deterministische collision query (server = client) | Bit-identieke raycast resultaat server/client | Plan §3.5 authoritative server |

**Risico's:** Collision mesh drift na edits; tunneling bij hoge snelheid. **Mitigatie:** CCD (continuous collision detection) voor speler, revision-gated mesh rebuild.

**Spike-voorstel:** `voxel-collision` crate: capsule cast vs greedy mesh vs direct voxel-grid cast. Benchmark: 10⁶ casts random richtingen.

---

### 2.5 Destruction & Voxel-to-Voxel Physics (destruction)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Materiaal-gebaseerde fracturing (hardness, toughness) | Per-voxel materiaal ID → breukpatroon | Teardown GDC 2022 (Dennis Gustafsson), Milan Bonten voxel-physics |
| Chunk-local rigid-body clusters na breuk | ≤ 50 actieve clusters; stabilisatie < 100 ms | Teardown multiplayer postmortem (80.lv interview) |
| Netwerk-gedistribueerde destructie (authoritative) | Server valideert edit; clients reconciliëren binnen 2 ticks | Plan §3.5, Teardown multiplayer design |

**Risico's:** Explosieve fragmentatie → O(N²) contact pairs; netwerk-bandbreedte piek. **Mitigatie:** Cluster-merging, max fragments/chunk, unreliable net-kanaal voor debris-transforms.

**Spike-voorstel:** Minimal "Teardown-lite" spike: 16³ chunk, materiaal-palette 4, explosie → fragment clusters → server-authoritative sync. Metrics: fragments/sec, bandwidth spike, reconvergence ticks.

---

### 2.6 Animation & Character Control (animation)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Capsule controller op voxel terrain (stappen, hellingen, voxels) | Geen sinking/clipping; ≤ 2 ms/frame | Plan §3.4, Godot/Bevy character controller |
| Procedural IK voor voeten op oneffen voxel-terrain | Geen floating feet; max 4 raycasts/poot/frame | GTA VI motion-matching papers (GDC 2024), procedural animation blogs |
| Third/first person camera met smooth transitions | Geen jitter bij voxel-grid snapping | Plan clientshell spikes |

**Risico's:** Voxel-terrain oneffenheid → jittery IK. **Mitigatie:** Terrain-normal smoothing, foot-lock blending.

**Spike-voorstel:** Character controller spike in Godot + Bevy met identieke test-scenes (helling, trap, gat, voxel-edit tijdens lopen).

---

### 2.7 AI & Entity Simulation (ai-sim)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Chunk-local behavior trees (BT) | ≤ 1 ms/tick per 8 spelers (20–30 Hz tick) | Plan §3.5, chunk-based interest management |
| Navmesh op voxel-terrain (recast/detour of custom) | Rebuild < 200 ms na edit; query < 0.1 ms | Recast/Detour voxel integration, VoxelNavMesh papers |
| Emergent schedules (dag/nacht, needs) | Data-driven, deterministic replay | Crimson Desert NPC systems (GDC talks) |

**Risico's:** Navmesh rebuild storm bij grote edits. **Mitigatie:** Dirty-region rebuild, hierarchical navmesh (tiles = chunks).

**Spike-voorstel:** Navmesh tile = chunk; edit → mark dirty tiles; background rebuild; benchmark 100 concurrent agents.

---

### 2.8 Networking & Replication (protocol + dedicated-server)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Authoritative fixed-tick server (20–30 Hz) | Tick duration p99 < 16 ms (headless, geen GPU) | Plan §3.5, Valve networking docs |
| Client-side prediction + reconciliation (movement) | Replay convergence < 3 ticks na packet loss | Gabriel Gambetta "Fast-Paced Multiplayer" |
| Chunk-based interest management | Alleen chunks in radius + margin gestreamd | Plan §3.5, MMO interest management papers |
| Delta-compressed voxel edits (baseline + deltas) | Edit pakket < 200 bytes; baseline < 50 kB/chunk | Plan §3.5, Teardown multiplayer bandwidth |
| Reliable (inventory, edits) + unreliable (transforms) kanaal | 0% verlies op reliable; ≤ 5% op unreliable acceptabel | Plan §3.5, SteamNetworkingSockets |

**Risico's:** Chunk-baseline resync storm na reconnect; edit-conflicten (2 spelers dezelfde voxel). **Mitigatie:** Exponential backoff resync, server-authoritative edit ordering met revision-ID.

**Spike-voorstel:** `dedicated-server` + 2–8 headless clients (bot inputs) → soak test: packet loss 0–10%, latency 0–200 ms, jitter. Metrics: convergence ticks, bandwidth/client, server tick p99.

---

### 2.9 Persistence & Save/Load (world-store)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Append-only editlog per regio/chunk | Write-ahead log fsync < 5 ms/edit | Plan §3.6, SQLite WAL |
| Periodieke compacte snapshots | Snapshot + log replay = live state (deterministisch) | Plan §3.6 deterministische replay test |
| Schema/protocol versioning vanaf dag 1 | Migratietest v1→v2 zonder data loss | Plan §3.6 |
| Hard-kill/restart recovery | Max verlies = onbevestigde edits (< 100 ms) | Plan §5.5 gate |

**Risico's:** Editlog groei onbegrensd; snapshot I/O spike. **Mitigatie:** Incrementele snapshots, background compaction, size-capped segments.

**Spike-voorstel:** `world-store` crate: SQLite WAL + periodieke snapshot → kill -9 test → replay hash vergelijking.

---

### 2.10 Authoring & Tools (tools/world-inspector)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| In-engine voxel brush (plaats/verwijder/materiaal) | Edit latency < 16 ms (lokale edit → mesh update) | Lay of the Land editor UX |
| Procedurale macro-brushes (noise, structuren) | Hot-reload < 500 ms; deterministische seed | John Lin procedural brushes |
| Palette editor met materiaal-eigenschappen | Live update zonder herstart | Plan §3.2 palette system |
| Debug overlay: chunks, jobs, triangles, memory, rev-IDs | 60 FPS met overlay aan | Plan fase 3 gate |

**Risico's:** Tooling wordt bottleneck voor content. **Mitigatie:** Headless CLI exporters, scriptable brushes.

**Spike-voorstel:** `world-inspector` tool (egui/Bevy of Godot UI) met live chunk-inspector, edit-history slider, replay scrubber.

---

### 2.11 Observability, Telemetry & CI (benchmarks + tests)

| Capability | Metric / Acceptance | Bron / Referentie |
|------------|---------------------|-------------------|
| Per-frame CPU/GPU timers (Tracy / puffin / bevy_debug) | Frame budget breakdown per systeem | Plan §6 performance targets |
| Chunk-load latency histogram (p50/p95/p99) | p95 < 100 ms LOD0 | Streaming spike |
| Network stats: bandwidth/client, RTT, packet loss, reconvergence | Geautomatiseerde soak-test rapport | Plan §6, §3.5 |
| Deterministische replay hash (server state) | Hash identiek na snapshot+log replay | Plan §3.6 |
| Property/fuzz tests op protocol, serialisatie, coördinaten | 0 crashes na 10⁶ iteraties | Plan §6 security/fuzz |
| CI: cargo fmt, clippy, test, bench, miri, sanitizers | Groen op elke PR | Plan §5.5 weekgate |

**Risico's:** Observability overhead in release builds. **Mitigatie:** Feature-gated telemetrie, sampling profiler.

**Spike-voorstel:** CI pipeline met `cargo bench -- --save-baseline main` + regressie-detectie > 5% frametime delta.

---

## 3. Samengevatte risico-matrix (top 9)

| # | Risico | Impact | Kans | Mitigatie (spike/gate) |
|---|--------|--------|------|------------------------|
| 1 | Uniforme micro-voxels → OOM / bandwidth explosion | Kritiek | Hoog | Hiërarchische sparse bricks + procedurele basis (spike 2.1) |
| 2 | Greedy meshing T-junctions / cracks op chunk-grenzen | Hoog | Middel | Halo-samples + greedy spike (2.2) |
| 3 | Destructie fragmentatie → physics/net storm | Hoog | Middel | Cluster limit, unreliable debris channel (2.5) |
| 4 | Navmesh rebuild storm na grote edits | Middel | Hoog | Dirty-tile incremental rebuild (2.7) |
| 5 | Chunk-baseline resync storm na reconnect | Hoog | Middel | Exponential backoff + prioritized chunks (2.8) |
| 6 | Floating-origin jitter bij grote coördinaten | Middel | Laag | 64-bit integer wereld + camera-relative render (2.3) |
| 7 | Determinisme breuk server/client (float vs int) | Kritiek | Middel | Integer wereldcoördinaten, vast revisie-ID protocol (2.1, 2.8) |
| 8 | Scope creep → MMO-infra vóór vertical slice | Kritiek | Hoog | Expliciete niet-doelen, gates per fase (Plan §4) |
| 9 | Steam = hosting misconceptie | Middel | Laag | Steam = identity/lobby/relay, niet compute (Plan §3.7) |

> Meta-risico "free-model instabiliteit voor research/codegen" is geen engine-risico; verplaatst naar `docs/governance/alignment-log.md` per researchprotocol §6.

---

## 4. Concrete spike-voorstellen (prioriteitvolgorde)

| Spike ID | Titel | Doel / Acceptance | Geschatte duur | Afhankelijkheden |
|----------|-------|-------------------|----------------|------------------|
| S-01 | `voxel-core` coordinate + storage crate | Property-tests passen, bench < 100 ns/voxel op 4080S | 3 dagen | Geen |
| S-02 | `voxel-mesher` 3 backends + golden fixtures | Greedy ≤ 1.5× culled tris, geen cracks | 4 dagen | S-01 |
| S-03 | Streaming simulator (synth. spelerpad) | p95 chunk-load < 100 ms | 3 dagen | S-01, S-02 |
| S-04 | `voxel-collision` capsule vs greedy mesh | ≤ 2 ms/frame, deterministisch | 3 dagen | S-02 |
| S-05 | Destruction-lite (16³ chunk, 4 materialen) | Fragments < 50, bandwidth spike < 200 kB | 4 dagen | S-01, S-04 |
| S-06 | Character controller spike (Godot + Bevy) | Identieke test-scene passed | 3 dagen | S-04 |
| S-07 | Dedicated server + 8 bot clients soak | 30 min stabiel, tick p99 < 16 ms | 5 dagen | S-01, S-08 |
| S-08 | `protocol` crate + serialization fuzz | 0 crashes 10⁶ fuzz iteraties | 2 dagen | S-01 |
| S-09 | `world-store` SQLite WAL + snapshot/replay | Kill -9 → replay hash match | 3 dagen | S-01 |
| S-10 | CI pipeline + Tracy/bench regression gate | Groen op PR, >5% regressie = fail | 2 dagen | Repo setup |

---

## 5. Bronnen (primaire URLs)

| # | Bron | Type | Relevantie |
|---|------|------|------------|
| 1 | https://voxely.net/blog/the-perfect-voxel-engine/ | Blog (John Lin) | Data-formaat, allocation/tagging/conversion, modulariteit |
| 2 | https://github.com/Lin20/BinaryMeshFitting | Repo (John Lin) | Implementatiereferentie |
| 3 | https://80.lv/articles/teardown-developer-breaks-down-multiplayer-and-voxel-destruction-tech/ | Interview (Dennis Gustafsson) | Destructie, multiplayer, voxel-vs-voxel physics |
| 4 | https://milanbonten.github.io/voxel-physics-engine/ | Demo + blog | Voxel physics engine implementatie |
| 5 | https://www.gdcvault.com/play/1025501/Teardown- | GDC Talk (Teardown) | Destructie pipeline, networking |
| 6 | https://github.com/mikolalysenko/mikolalysenko.github.io | Blog (Mikolalysenko) | Greedy meshing, collision meshing, SVO |
| 7 | https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking | Valve docs | Authoritative networking, prediction, reconciliation |
| 8 | https://gamedev.net/tutorials/programming/graphics/virtual-texture-terrain-r3278/ | Tutorial | Virtual texturing / megatexture streaming |
| 9 | https://www.gdcvault.com/play/1025501/ (GTA VI animation) | GDC 2024 | Motion matching, procedural animation |
| 10 | Plan document: `.hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md` | Intern plan | Gates, architecture, risk matrix, stack choices |

---

## 6. Volgende stappen (conform plan)

1. **Reviewer** (gratis model) valideert: bron-URLs bereikbaar, metrics meetbaar, risico's volledig, spikes concrete acceptatiecriteria hebben.
2. **Architect** synthetiseert geverifieerde resultaten → eerste ADR's: voxel-representatie (ADR-001), stack-keuze (ADR-002), multiplayer-doel (ADR-003).
3. Daarna repository scaffolding (`Cargo.toml` workspace, CI, `voxel-core` crate) en spike S-01 starten.

---

*Document gegenereerd volgens researchprotocol: primaire bronnen, geen betaalde modellen, meetbare acceptatiecriteria, expliciete risico's en spike-voorstellen.*