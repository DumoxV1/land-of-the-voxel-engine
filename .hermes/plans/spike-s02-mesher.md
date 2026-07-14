# Spike S-02 — `voxel-mesher` (naïve → culled → greedy)

**Datum:** 2026-07-14
**Fase:** engine-startgate (S-01 groen afgerond)
**Methode:** Strict TDD — failing tests eerst (rood), dan minimale implementatie (groen).
**Afhankelijkheid:** S-01 (`voxel-core`) — gebruikt `Chunk`, `MaterialId`, `LocalVoxel`, `WorldVoxel`.
**Geen betaalde modellen:** alles lokaal; gratis `:free` alleen voor triage.

## Scope (kleinste bewezen meshing-kern)
Drie opeenvolgende mesher-backends over een voxel-chunk, die allemaal een lijst van triangles
(opposite-face, geen indices) opleveren:
1. **Naïve mesher** — één cubus (6 faces) per niet-lege voxel. Baseline.
2. **Culled mesher** — verwijder faces die tegen een niet-lege buurvoxel aanliggen (face culling).
3. **Greedy mesher** — per materiaal/normal-richting zo groot mogelijke quad's samenvoegen.

Input = een `voxel_core::Chunk` (of een `SolidFn` view). Output = `Vec<Triangle>` met positie +
normaal + materiaal. Geen GPU/renderer — puur data (ADR-0002).

## Acceptance criteria (concreet, meetbaar)
- `cargo test -p voxel-mesher` 100% groen.
- **Golden fixtures** (vier chunk-scenario's):
  - `empty` — chunk volledig leeg → 0 triangles.
  - `full` — chunk volledig gevuld → naïve = 6·N³ faces; culled/greedy = 0 (ingesloten, geen
    blootliggende faces); géén cracks (gesloten hull bij grens-interactie apart getest).
  - `checkerboard` — afwisselend gevuld → culled verwijdert interne faces t.o.v. naïve.
  - `single_voxel` — één voxel → naïve = 6 faces, culled = 6 faces, greedy = 6 faces (geen merge mogelijk).
- **Culling-correctheid**: culled triangle-count < naïve triangle-count voor `checkerboard` en
  `full-surface` scenario's (tenzij alles ingesloten → allebei 0).
- **Greedy ≤ 1,5× culled triangles** (plan §3.3 / north-star S-02): voor een `full-surface` chunk
  (holle wereld met één blootliggende laag) geldt greedy triangle-count ≤ 1,5 × culled count.
  Opmerking: voor vlakke oppervlakken is greedy vaak véél lager (6 faces → 2 quads = 4 triangles);
  de 1,5× is een bovengrens, geen streefwaarde.
- **Geen cracks**: een greedy-mesh van een volledig gevulde chunk mét blootliggende buitenkant
  (d.w.z. chunk-rand telt als "leeg" = lucht) dekt exact de buitenste 6 zijden — volume-conservatie
  check via face-normal som of triangle-boundary check.
- `cargo build -p voxel-mesher` compileert **zonder** godot/bevy/wgpu (ADR-0002).
- **Geen renderer-dependency** in Cargo.toml.

## Repository-wijziging
- Nieuwe crate `crates/voxel-mesher` toegevoegd aan workspace (`members`).
- `voxel-mesher` dependt op `voxel-core` (path).
- Criterion-benchmark (optioneel, niet vereist voor groen): naïve/culled/greedy per chunk-grootte.

## TDD-volgorde
1. Scaffold crate + `lib.rs` stub.
2. Schrijf FAILING tests (rood): golden fixtures + culling/greedy ratio + geen-cracks.
3. Run `cargo test -p voxel-mesher` → ROOD (compileert niet / assertions falen).
4. Implementeer naïve → culled → greedy minimaal → groen.
5. Run `cargo test` + `cargo build` → GROEN + renderer-agnostisch.

## Niet in S-02 (expliciete niet-doelen)
- Normaal-richting-specifieke shaders / UV's (alleen positie+normaal+materiaal).
- LOD / Transvoxel / smooth SDF (S-03+ later).
- Async remesh-jobs met revision-ID (S-03 streaming).
- Client-rendering (blijft renderer-agnostisch).
