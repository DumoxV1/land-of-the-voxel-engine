# Spike-plan: S-13 micro-voxel resolutie (12,5 cm)

**ADR:** ADR-0005 (Accepted 2026-07-15). **Doel:** engine draait op **1 voxel = 12,5 cm** i.p.v.
impliciet 1 m, zónder architectuur-breuk (chunk blijft 32³ voxels, wereld-coördinaten
blijven integer-in-voxels).

**Acceptatiecriteria (meetbaar, strict TDD):**
1. `coords::VOXEL_SIZE_M == 0,125` en `chunk_m_size() == 4,0`.
2. **1 km² = 62.500 chunks** (1000 m / 4 m = 250 per zijde → 250²). Failing test
   `s13_resolution::one_km2_is_62500_chunks` beweert dit (berekend uit `chunk_m_size()`).
3. Camera-eye op 12,5 cm-schaal: `gpu_window`/`gpu_world` tonen de wereld vanaf ~14 voxels
   (~1,8 m) hoogte, niet 55 m. Bench view-radius default verhoogd naar ~60 chunks (≈240 m).
4. `cargo test --workspace` blijft groen (geen regressie in coords/worldgen/physics door de schaal).
5. Bench (S-12c) hermeten op 1 km² met de nieuwe schaal → nieuwe `bench_1km2.json`.

**Stappen (Rood → Groen):**
- [ ] `crates/voxel-core/tests/spike_s13.rs` schrijven: 2 failing tests (VOXEL_SIZE_M, one_km2_chunks).
      `cargo test -p voxel-core --test spike_s13` → compileerfout (const bestaat niet).
- [ ] `coords.rs`: `pub const VOXEL_SIZE_M: f32 = 0,125;` + `pub fn chunk_m_size() -> f32`
      (`CHUNK_SIZE as f32 * VOXEL_SIZE_M`). Test wordt groen.
- [ ] `voxel-worldgen`: fijnere noise-schaal (grid-period in voxels, niet chunks) zodat 12,5 cm
      écht detail toont. Geen API- change nodig voor de test, maar wel visueel zichtbaar in bench.
- [ ] `voxel-gpu` camera's: `gpu_window.rs` + `gpu_world.rs` eye op ~14 voxels; bench default
      radius 60. `render_triangles`/pipeline ongewijzigd.
- [ ] `cargo test --workspace` groen; bench herdraaien (1 km² → 62.500 chunks).
- [ ] ADR-0005 + ROADMAP + alignment-log updaten; commit + push (grens A).

**Scope-grens:** geen hiërarchische macro/micro-onderverdeling (plan §2.1) — vlakke 12,5 cm
is de eerste configuratie. LOD/bricks (advies #5) komen later, op dezelfde voxel-maat.
