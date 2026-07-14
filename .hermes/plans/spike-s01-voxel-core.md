# Spike S-01 — `voxel-core` coordinate + storage crate

**Datum:** 2026-07-14
**Fase:** 0 → engine-startgate (geopend na researchreview + plansynthese)
**Methode:** Strict TDD — failing tests eerst (rood), dan minimale implementatie (groen).
**Geen betaalde modellen:** alle code/testen lokaal; gratis `:free` alleen voor triage indien nodig.
**Blocker:** geen — engine-startgate is geopend (zie `.hermes/PROJECT_STATE.md`).

## Scope (uiterst klein, bewezen kern)
Alleen de kleinste bewezen kern voor de hele engine:
1. **Coördinaten**: `WorldVoxel`, `ChunkCoord`, `LocalVoxel` als aparte types; integer wereldcoördinaten; euclidische deling zodat negatieve coördinaten correct zijn.
2. **Chunk**: vaste chunkgrootte (start `32³`); drie chunktoestanden beginnen als `Uniform` (één materiaal) — palette/dense volgen in latere S-01-iteraties.
3. **Palette**: per-chunk materiaalpalette, geplette materiaal-ID's (start: uniforme chunk heeft impliciet één materiaal; bitpacking volgt wanneer dense/palette state wordt toegevoegd).
4. **Edits**: edit-event met wereldpositie, oud/nieuw, actor, server-tick, monotone revisie; idempotentie-contract.
5. **Serialisatie**: round-trip `serialize(deserialize(x)) == x` byte-stabiel (canonieke vorm).
6. **Property-tests**: coördinaat-roundtrip wereld↔chunk↔lokaal voor `[-10⁶, 10⁶]`; negatief-correct.

## Acceptance criteria (concreet, meetbaar)
- `cargo test -p voxel-core` 100% groen.
- `cargo test -p voxel-core --features proptest` property-tests groen (roundtrip + negatief).
- Coördinaat-roundtrip property-test faalt op negatieve Euclidische deling vóór fix (rood aantoonbaar).
- Serialisatie-roundtrip is byte-identiek (assert `serialize(x)` == `serialize(deserialize(serialize(x)))`).
- Edit-idempotentie: twee identieke edits op zelfde (pos, revisie) leveren één revisie-opname.
- `cargo build -p voxel-core` compileert **zonder** godot/bevy/wgpu-import (ADR-0002).
- Benchmark-harnas (Criterion) aanwezig; target <100 ns/voxel is een latere meting, geen gate nu.

## Repository-indeling (Phase 0 scaffold)
```
Land of the Voxel Engine/
├── Cargo.toml                # workspace
├── crates/
│   └── voxel-core/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── coords.rs     # WorldVoxel / ChunkCoord / LocalVoxel
│           ├── chunk.rs      # Chunk + ChunkState
│           ├── palette.rs    # Material palette
│           ├── edit.rs       # Edit event + idempotence
│           └── serialize.rs  # byte-stable round-trip
└── tests/                    # (bestaande project guards blijven)
```

## TDD-volgorde (deze sessie)
1. Scaffold workspace + `voxel-core` crate (compiles, leeg).
2. Schrijf FAILING tests (rood):
   - `coords_test`: roundtrip + negatief (euclidische deling).
   - `chunk_test`: uniform-chunk get/set, chunkstate.
   - `edit_test`: idempotentie bij dubbele edit.
   - `serialize_test`: byte-stabiele roundtrip.
   - property-test: willekeurige wereldcoördinaten roundtrip.
3. Run `cargo test` → toon ROOD (compileert niet / assertions falen).
4. Implementeer minimale code → groen.
5. Run `cargo test` + `cargo build` → toon GROEN + renderer-agnostisch.

## Niet in S-01 (expliciete niet-doelen)
- Greedy/culled meshing (S-02).
- Procedurele worldgen (S-03).
- Networking/persistence (S-08/S-09).
- Dense/palette-packed chunk states (volgen in S-01-iteratie 2).
