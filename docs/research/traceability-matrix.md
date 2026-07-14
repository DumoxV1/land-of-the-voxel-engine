# Cross-Memo Traceability Matrix

**Datum:** 2026-07-14
**Doel:** Voldoen aan de acceptatiecriteria van `review-initial-bundle.md` — elke plan-eis wordt
gespord aan een memo-claim, een primaire bron en een spike/benchmark. Alleen geverifieerde
bevindingen mogen naar ADR's en het spikeplan.

## Plan-eis → Memo-claim → Bron → Spike/Benchmark

| Plan-ref | Eis | Memo (claim) | Primaire bron | Spike / Benchmark |
|----------|-----|--------------|---------------|-------------------|
| §2.1 Micro-voxel-definitie | 12,5 cm LOD0, 8³ bricks, palette | north-star §1, voxel-data §Candidate 1 | John Lin "The Perfect Voxel Engine" | S-01 (storage), S-02 (meshing) |
| §2.2 Custom engine = eigen core | Renderer-agnostische `voxel-core` crate | engine-stack B-07 (core portability) | Plan §2.2 | B-06 Determinism replay |
| §2.2 Twee equal client-spikes | Godot + Bevy gelijkwaardig vóór keuze | engine-stack §6 (equal-priority) | Plan §4 Phase 2 | Phase 2 spikes |
| §2.3 Blocky/palette Phase 1 | Greedy meshing + culling eerst | voxel-data §Recommendation | Mikolalysenko greedy meshing | S-02 (3 backends) |
| §3.2 Coördinaten | Integer wereld, euclidische deling, 3 types | north-star §2.1, voxel-data | Plan §3.2 | S-01 (property tests) |
| §3.2 Opslag | Uniform / palette / dense; ≤4 B/voxel | north-star §2.1 | John Lin pipeline | S-01 (bench <100 ns/voxel) |
| §3.3 Meshing | Naïve→culled→greedy, geen cracks | voxel-data §Candidate 1, north-star §2.2 | Mikolalysenko | S-02 (golden fixtures) |
| §3.4 Physics | Capsule vs voxel, geen destructie fase 1 | north-star §2.4 | Teardown GDC 2022 | S-04 |
| §3.5 Authoritative server | 20–30 Hz, client input only | network-persistence Claim 1 | Valve Source Networking wiki | S-07 |
| §3.5 Interest management | Chunk-granulair | network-persistence Claim 4 | Mikolalysenko blog / Teardown 80.lv | S-07 |
| §3.5 Delta-compressie | Baseline + deltas, idempotent | network-persistence Claim 3/5 | Gaffer Snapshot Compression (CC-BY-NC-SA, research only) | S-08 (fuzz), Experiment 2 |
| §3.5 Edit-idempotentie | Revisie + afwijzen duplicaten | network-persistence Claim 5 | Facepunch.Steamworks #254 | S-01 (edit tests) |
| §3.6 Persistence | Append-only + SQLite WAL snapshots | network-persistence Claim 6 | Teardown save format (GDC 2022) | S-09 |
| §3.7 Steam = identity/relay, niet hosting | Partnership vereist voor GNS-prod | network-persistence Claim 2/7 | Valve ISteamNetworkingSockets | — (fase 6 spike) |
| §4 Vertical slice (geen MMO) | 2–8 spelers, persistent edits | alle memo's | Plan §4 | S-03…S-10 |

## Gecorrigeerde/onzeker gebleven claims (geen ADR zonder meting)

| Claim | Status | Actie vóór definitief |
|-------|--------|----------------------|
| network-persistence "< 1 KB/tick" (was Claim 3) | Verwijderd → "techniek aangetoond, voxel-benchmark vereist" | Experiment 2 meet delta-grootte |
| voxel-data benchmarkcijfers (45% / 3.2× / 4.1×) | "unverified — requires Criterion reproduction" | B-04: reproduceren op RTX 4080 Super |
| voxel-data Blocky "Production Readiness: High" | Gedegradeerd naar "Medium" | Pas na RPG-scale spike |
| north-star "≥10⁷ micro-voxels" | Target, geen afleiding | S-03 streaming spike |
| north-star "≤2 ms collision" | Gesplitst server ≤2 ms/tick, client ≤1 ms/frame | S-04 collision spike |
| network-persistence GNS-licentie | Gecorrigeerd: Valve BSD-like + crate MIT | — |

## Beslisstatus per memo (na correctie)

- `network-persistence.md` → Kandidaat (B-01/02/03 toegepast)
- `voxel-data-rendering-candidates.md` → Kandidaat (B-04/05 toegepast)
- `north-star-capabilities.md` → Hypothese (B-08 toegepast; metrics = targets)
- `engine-stack-and-reuse.md` → Kandidaat (B-06/07 toegepast)

## Synthese-uitkomst

Van bovenstaande geverifieerde claims zijn drie ADR's gesynthetiseerd:
- `adr/0001-voxel-representation.md` (blocky/palette + sparse bricks als Phase-1 start)
- `adr/0002-renderer-agnostic-core.md` (`voxel-core` zonder renderer-dependency)
- `adr/0003-multiplayer-target.md` (2–8 spelers, authoritative, chunk-interest)

De engine-startgate is hiermee geopend voor S-01 (`voxel-core`) onder strict TDD.
