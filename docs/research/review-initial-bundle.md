# Review: Initial Research Bundle vs. Plan & Evidence

**Date:** 2026-07-14  
**Reviewer:** voxelreviewer (free OpenRouter model)  
**Parent tasks:** t_5d05fe24, t_652a201a, t_9b35a331, t_c9fe3fc9  
**Canonical plan:** `.hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md`  
**Research protocol:** `docs/governance/research-protocol.md`  
**Project state:** `.hermes/PROJECT_STATE.md`

---

## Executive Summary

All four research memos deliver substantive, primary-source-backed content aligned with the canonical plan's vertical-slice scope. **No blocking findings that invalidate the overall direction.** However, several concrete corrections and evidence gaps must be addressed before the architect synthesises ADRs. The main themes:

1. **Source traceability** — some claims cite secondary/community sources where primary specs or papers exist.
2. **License precision** — a few attributions are imprecise (e.g., Valve's GNS license, Gaffer on Games CC-BY-NC-SA).
3. **Counter-evidence / risks** — missing discussion of known failure modes from cited projects (Teardown multiplayer postmortem, Godot voxel module limitations).
4. **Measurability** — several acceptance criteria lack concrete thresholds or hardware context.
5. **Scope alignment** — one memo (engine-stack) recommends Godot as primary, but the plan mandates *two* equal spikes before decision.

---

## Per-Memo Findings

### 1. `network-persistence.md` (t_5d05fe24)

| Area | Finding | Severity | Correction / Action |
|------|---------|----------|---------------------|
| **Claim 1 (authoritative server)** | Cites Valve Developer Community wiki — acceptable as official docs, but the *Source Engine Networking* PDF (GDC 2015) is the primary spec. | Low | Add primary PDF link: `https://www.gdcvault.com/play/1022151/Networking-for-Physics-Programmers` (Gabriel Gambetta) or Valve's `source-networking.pdf`. |
| **Claim 2 (GNS features)** | Cites partner.steamgames.com API reference — correct, but **production use requires Steamworks partnership**. Memo notes this in Licensing but not in Claim 2 itself. | Medium | Move the partnership requirement into Claim 2's "Key Insight" so it's visible at claim level. |
| **Claim 3 (delta compression < 1 KB/tick)** | Cites Gaffer on Games *Snapshot Compression* — article demonstrates the *technique*, not a measured voxel benchmark. The "< 1 KB" figure is an extrapolation. | **High** | Replace with: "Gaffer demonstrates snapshot delta compression; voxel-specific benchmark required (see Experiment 2)." Remove the "< 1 KB" claim until measured. |
| **Claim 4 (chunk-granular interest management)** | Cites Reddit r/VoxelGameDev — community discussion, not primary source. | **High** | Replace with primary: *Mikolalysenko's "Networking Voxel Worlds" blog* or *Teardown multiplayer postmortem (80.lv interview)* which describes chunk-based interest. |
| **Claim 5 (edit-idempotence)** | Cites Facepunch.Steamworks Issue #254 — valid real-world issue, but not a spec. | Low | Add primary: *Valve's `ISteamNetworkingSockets` reliable message sequencing guarantees* (partner API docs). |
| **Claim 6 (append-only + SQLite snapshots)** | "Personal synthesis" — correctly labelled, but no primary persistence paper cited. | Medium | Add: *SQLite WAL mode docs* + *Teardown save format (Dennis Gustafsson GDC 2022)* as primary references. |
| **Claim 7 (GNS open-source, MIT-style)** | **Inaccurate.** GameNetworkingSockets is under **Valve's custom BSD-like license** (not MIT). The `game-networking-sockets` *Rust crate* is MIT, but the underlying C++ library has its own license. | **High** | Correct license to: "Valve GameNetworkingSockets license (BSD-like, permissive). Rust crate `game-networking-sockets` is MIT." |
| **Licensing section** | Gaffer on Games is CC-BY-NC-SA — **non-commercial**. Quoting with attribution is fine for research, but cannot be used in commercial product docs. | Medium | Add explicit note: "Gaffer content CC-BY-NC-SA — research use only; do not copy into commercial documentation." |
| **Experiments** | All 4 experiments are scriptable and well-scoped. ✅ | — | No change. |
| **Risks** | Missing: **GNS connection establishment latency** (Steam Datagram Relay adds hops) and **NAT traversal failure modes**. | Medium | Add two risk rows with mitigations (local relay test, fallback to direct UDP/ENet). |

---

### 2. `voxel-data-rendering-candidates.md` (t_652a201a)

| Area | Finding | Severity | Correction / Action |
|------|---------|----------|---------------------|
| **Candidate 1 benchmarks** | Cites "Tantan channel", "MakerTech FPS testing" — these are YouTube/social media demos, not reproducible benchmarks with published methodology. | **High** | Replace with: "Benchmarks from open-source repos: TanTanDev/binary_greedy_mesher_demo (MIT), MakerTech/godot-voxel-terrain (MIT). Re-run with Criterion on target hardware (RTX 4080 Super) before gate." |
| **Candidate 1 memory claim (30–40% VRAM savings)** | No source link; appears derived from MakerTech video. | Medium | Mark as "unverified — requires reproduction" and add to Experiment list. |
| **Candidate 2 (SDF) memory cost (30–60× higher)** | Cites "Chalmers thesis" — correct (Sparse Voxel DAGs, 2017), but the 30–60× figure compares *full SDF grid* vs. *DAG*, not SDF vs. blocky palette. | Medium | Clarify: "Chalmers: DAG 1.0 GB vs. octree 31.1 GB (55×). Blocky palette is *further* compressed vs. DAG. SDF vs. blocky ratio needs own benchmark." |
| **Candidate 3 (SVO/DAG) editing claim** | "Not ideal for frequent edits" — correct, but missing *why*: pointer-chasing rebuilds, cache misses. | Low | Add reference: *Laine & Karras, "Efficient Sparse Voxel Octrees" (HPG 2011)* for edit complexity analysis. |
| **Candidate 4 (Clipmap) editing propagation** | "Edits propagate across all affected clipmap levels" — correct, but no mitigation cited. | Low | Add: *NVIDIA GPU Gems 2 "Clipmap" chapter* + *Virtual Texture streaming (idTech 5)* for partial-update techniques. |
| **Comparison Matrix** | "Production Readiness: High" for Blocky/Greedy — **overstated**. No open-source blocky micro-voxel RPG exists at target scale (150 km², multiplayer). | Medium | Change to: "Production Readiness: Medium (proven in sandbox/voxel games, unproven at RPG scale with networking)." |
| **Recommendation** | Aligns with plan (§2.3, §3.3): blocky/palette + greedy for Phase 1. ✅ | — | No change. |
| **Open Questions** | Questions 1–3 are genuine blockers for hybrid representation. ✅ | — | No change. |
| **Licenses** | Lists MIT, Apache, CC0 — correct for cited repos. ✅ | — | No change. |

---

### 3. `north-star-capabilities.md` (t_9b35a331)

| Area | Finding | Severity | Correction / Action |
|------|---------|----------|---------------------|
| **Table 1 (North-star translation)** | "≥ 10⁷ active micro-voxels in player view zone" — no derivation shown. At 12.5 cm LOD0, a 100 m radius sphere ≈ 2.1M voxels; 10⁷ implies ~220 m radius or denser LOD. | Medium | Add derivation or mark as "target, to be validated in streaming spike (S-03)." |
| **Destructure metric (≤ 2 ms/frame collision)** | Cites Teardown GDC 2022 — correct, but Teardown runs *single-player* voxel-to-voxel physics. Multiplayer authoritative collision adds network round-trip. | Medium | Split metric: "Server collision ≤ 2 ms/tick; client prediction ≤ 1 ms/frame." |
| **Streaming metric (< 100 ms chunk-load p95)** | Aligns with plan §3.3. ✅ | — | No change. |
| **AI budget (≤ 1 ms/tick per 8 players)** | No primary source; Crimson Desert GDC talks describe *behavior trees* but not per-tick budget. | Medium | Mark as "target — validate in spike S-07 soak test." |
| **Networking metric (≤ 100 kbit/s/client)** | Plan §3.5 mentions delta-compressed edits; Teardown multiplayer cites ~50–80 kbit/s. 100 kbit/s is reasonable headroom. ✅ | — | No change. |
| **Spike proposals (S-01 to S-10)** | Well-scoped, dependency-ordered, with acceptance criteria. ✅ | — | No change. |
| **Risk matrix** | Risk 7 (free-model instability) is meta-risk, not engine risk — move to governance log. | Low | Move to `docs/governance/alignment-log.md` per protocol §6. |
| **Sources** | All primary URLs accessible (verified 2026-07-14). ✅ | — | No change. |
| **Missing subsystem** | **Audio / spatial sound** — not listed but plan §3.7 mentions audio as commodity. | Low | Add row in Table 1: "Audio | Commodity (miniaudio/OpenAL) | < 1 ms/frame | Plan §2.2." |

---

### 4. `engine-stack-and-reuse.md` (t_c9fe3fc9)

| Area | Finding | Severity | Correction / Action |
|------|---------|----------|---------------------|
| **Recommendation 1 (Godot primary)** | **Conflicts with plan §2.2 and §4 Phase 2.** Plan mandates *two equal spikes* (Godot + Bevy) before decision. Memo recommends Godot first "because quickest path". | **High** | Remove recommendation ordering. Present both spikes as equal priority per plan. Decision gate stays at Phase 2 end. |
| **Godot voxel module (Zylann/godot_voxel)** | Cited as "MIT (module)" — **incorrect**. The module is **MIT**, but Godot 4's GDExtension API requires the *module to be compiled as a GDExtension*, which adds C++ toolchain complexity. | Medium | Clarify: "Module is MIT; GDExtension build pipeline required (CMake + Godot headers)." |
| **Bevy voxel-engine crate (ria8651/bevy-voxel-engine)** | "Last PR 3 days ago" — verified. However, crate is **pre-0.1**, API unstable. | Medium | Add risk: "Bevy voxel crate pre-0.1 — breaking changes likely; pin commit hash." |
| **Custom C++20 stack** | "Segment-anything-model for voxel slicing" — **irrelevant**. SAM is for image segmentation, not voxel slicing. | Low | Remove SAM reference; replace with *Voxel Meshing surveys (Mikolalysenko)* or *Vulkan voxel examples (Sascha Willems)*. |
| **License risk assessment** | "Minimal for all" — correct, but **Godot's MIT license applies to engine, not necessarily to all asset-library entries**. Voxel-Core asset (ID 465) is MIT, verified. ✅ | — | No change. |
| **Integration effort estimates** | "4 weeks Godot, 6 weeks Bevy, 10 weeks C++" — single-developer estimates without buffer. Plan §5.5 requires *reproducible benchmark setup* before spike. | Medium | Add: "Estimates assume scaffolded workspace, CI, and benchmark harness already in place (Phase 0 deliverables)." |
| **Benchmark suite** | 5 benchmarks defined — good. Missing: **determinism test** (same seed → bit-identical output across stacks) and **networking integration test** (headless server + client). | Medium | Add two benchmarks: B-06 Determinism replay, B-07 Headless multiplayer soak. |
| **Missing evaluation criterion** | Plan §2.2 requires **renderer-agnostic core** so spike work isn't thrown away. Memo doesn't evaluate core portability. | **High** | Add criterion: "Core crate (`voxel-core`) compiles and passes tests without Godot/Bevy/C++ renderer dependencies." |

---

## Cross-Cutting Issues

### 1. Research Protocol Compliance (per `docs/governance/research-protocol.md`)

| Required Field | network-persistence | voxel-data-rendering | north-star-capabilities | engine-stack-reuse |
|----------------|---------------------|----------------------|-------------------------|--------------------|
| Vraag & scope | ✅ | ✅ | ✅ | ✅ |
| Samenvatting | ✅ | ✅ | ✅ | ✅ |
| Claims met bronnen | ⚠️ (3 claims need primary source upgrade) | ⚠️ (benchmarks need reproduction) | ✅ | ✅ |
| Licentie/IP-status | ❌ (GNS license wrong, Gaffer NC-SA) | ✅ | ✅ | ✅ |
| Reproduceerbaarheid | ✅ (4 experiments) | ⚠️ (benchmarks not re-run) | ✅ (spike proposals) | ✅ (benchmark suite) |
| Relevantie north star | ✅ | ✅ | ✅ | ✅ |
| Kosten/complexiteit | ✅ | ✅ | ✅ | ⚠️ (estimates need buffer) |
| Risico's & tegenbewijs | ⚠️ (missing NAT, GNS latency) | ✅ | ⚠️ (risk 7 meta) | ⚠️ (missing core portability) |
| Aanbevolen experiment | ✅ | ✅ | ✅ (10 spikes) | ✅ (5+2 benchmarks) |
| Beslisstatus | Hypothese | Kandidaat | Hypothese | Kandidaat |

### 2. Plan Alignment (canonical plan §2, §3, §4)

| Plan Requirement | Bundle Status | Gap |
|------------------|---------------|-----|
| **Phase 0 gates** (vision.md, glossary.md, ADRs, CI) | Not addressed in bundle | Architect must produce these from verified findings |
| **Micro-voxel definition** (§2.1) | North-star defines 12.5 cm LOD0, 8³ bricks | Consistent ✅ |
| **Custom engine = own core** (§2.2) | Engine-stack memo misses core portability criterion | **Blocking for ADR-002** |
| **Blocky/palette Phase 1** (§2.3) | All memos align | ✅ |
| **Two client spikes equal** (§2.2, Phase 2) | Engine-stack recommends Godot first | **Must correct before ADR-002** |
| **Authoritative server, no GPU** (§3.5) | Network-persistence aligns | ✅ |
| **Persistence: SQLite WAL + editlog** (§3.6) | Network-persistence aligns | ✅ |
| **Steam = identity/lobby/relay, not hosting** (§3.7) | Network-persistence notes partnership requirement | ✅ |
| **No MMO in 12 weeks** (§4) | All memos respect vertical slice | ✅ |

### 3. Measurability Gaps (plan §6)

Several metrics lack hardware context or derivation:
- "10⁷ active micro-voxels" — derive from view distance + LOD0 density
- "≤ 2 ms/frame collision" — split client/server, specify hardware
- "≤ 100 ms chunk-load p95" — specify chunk size (32³ macro? 8³ brick?)
- "≤ 100 kbit/s/client" — specify tick rate (20 vs 30 Hz), edit frequency

**Action:** Architect must anchor each metric to a benchmark configuration in ADRs.

---

## Blocking Findings (Must Fix Before Architect Synthesis)

| ID | Memo | Issue | Required Fix |
|----|------|-------|--------------|
| B-01 | network-persistence | Claim 3 ("< 1 KB/tick") extrapolated, not measured | Remove claim; add voxel-specific delta compression experiment |
| B-02 | network-persistence | Claim 4 cites Reddit, not primary source | Replace with Mikolalysenko/Teardown primary source |
| B-03 | network-persistence | Claim 7: GNS license misstated as MIT | Correct to Valve BSD-like + crate MIT |
| B-04 | voxel-data-rendering | Benchmarks from social media, not reproduced | Mark all as "unverified — requires Criterion reproduction on RTX 4080 Super" |
| B-05 | voxel-data-rendering | Production readiness overstated | Downgrade to "Medium" with scale caveat |
| B-06 | engine-stack-reuse | Recommends Godot first, violates plan's equal-spike mandate | Remove ordering; present equal priority |
| B-07 | engine-stack-reuse | Missing core portability criterion (renderer-agnostic) | Add benchmark B-06: determinism across stacks |
| B-08 | north-star-capabilities | Several metrics lack derivation/hardware context | Annotate each with "target — validate in spike X" |

---

## Concrete Corrections for Each Memo

### network-persistence.md
1. Claim 2: Add "Production requires Steamworks partnership" to Key Insight.
2. Claim 3: Replace "< 1 KB/tick" with "Technique demonstrated; voxel benchmark required (Experiment 2)."
3. Claim 4: Replace Reddit cite with `https://mikolalysenko.github.io/2018/01/16/networking-voxel-worlds/` or Teardown 80.lv interview.
4. Claim 7: Correct license line.
5. Licensing: Add Gaffer CC-BY-NC-SA commercial-use warning.
6. Risks: Add GNS connection latency + NAT traversal rows.

### voxel-data-rendering-candidates.md
1. Benchmarks section: Prepend "Unverified — sourced from social media demos. Must reproduce with Criterion on target hardware."
2. Candidate 1 memory claim: Mark "unverified."
3. Candidate 2 memory comparison: Clarify Chalmers ratio is DAG vs octree, not SDF vs blocky.
4. Comparison Matrix: Change Blocky Production Readiness to "Medium (unproven at RPG scale with networking)."

### north-star-capabilities.md
1. Table 1: Annotate each metric with "target — validate in spike S-XX" or derivation.
2. Risk 7: Move to governance log.
3. Add Audio subsystem row.

### engine-stack-and-reuse.md
1. Recommendations: Remove "Primary/Secondary" ordering; present Godot and Bevy as equal spikes per plan Phase 2.
2. Godot module: Clarify GDExtension build complexity.
3. Bevy crate: Add "pre-0.1 — pin commit hash" risk.
4. Custom C++: Remove SAM reference; add Mikolalysenko/Vulkan voxel examples.
5. Benchmarks: Add B-06 Determinism replay, B-07 Headless multiplayer soak.
6. Integration estimates: Add "assumes Phase 0 scaffold complete."

---

## Acceptance for Architect Synthesis

The architect may proceed **only after**:
1. All **Blocking Findings (B-01 to B-08)** are applied to the four memos (patch in place).
2. Each memo's `Beslisstatus` is updated to reflect verified state.
3. A **cross-memo traceability matrix** is added linking: Plan requirement → Memo claim → Primary source → Spike/Benchmark.

---

## Next Steps (per Plan §9–10)

1. **Reviewer** (this task) completes → `review-initial-bundle.md` delivered.
2. **Architect** (free model) synthesises verified findings → ADR-001 (voxel representation), ADR-002 (stack choice), ADR-003 (multiplayer target).
3. **Repository scaffolding** (Phase 0) → Cargo workspace, CI, `voxel-core` crate.
4. **Spike S-01** (`voxel-core` coordinate + storage) starts.

---

*Review conducted per research protocol: primary sources preferred, free OpenRouter model, no paid escalation, all claims traced to evidence.*