# Audit: Wereld-water feature (`crates/voxel-worldgen/src/lib.rs`)

**Datum:** 2026-07-17 · **Reviewer:** onafhankelijke code-review (subagent)
**Bestand:** `crates/voxel-worldgen/src/lib.rs` (1201 lijnen)
**Scoop:** nieuwe `WATER = 9` / `SEA_LEVEL_M = 180.0` + water-vulling in `generate_chunk` + 3 test-wijzigingen.

**Context-constanten (geverifieerd):**
- `VOXEL_SIZE_M = 0.125` m, `CHUNK_SIZE = 32` vox (= 4 m).
- `SEA_LEVEL_M / VOXEL_SIZE_M = 180.0 / 0.125 = 1440` vox → `sea_level_vox = 1440`, **exact** op chunk-grens `cy = 1440 / 32 = 45`.
- `MAX_SURFACE_M = 477` m (~3816 vox) → zeeniveau (180 m) zit ruim onder de hoogste pieken. `MaterialId` is een naakte `u8` (geen palette-validatie), dus `9` is een geldig id; de renderer/mesher moet `9` wel als water mappen (client-afhankelijk, zie caveat).

---

## 1. Correctheid van water-vulling (juiste kolom)
**Status: OK**

- Lijnen 300–305: bij `density <= 0.0` (lucht bóven de gewarpte surface) wordt `WATER` gezet i.p.v. air, **maar alleen als** `wy < sea_level_vox`.
- Lijnen 313–318: in de cave-tunnel-branch geldt hetzelfde (`wy < sea_level_vox` → water).
- Water vervangt **nooit** solide terrain: beide sets gebeuren in de `continue`-branches die vóór `classify()` liggen. Een voxel met `density > 0` (surface/overhang/stone) wordt nooit water.
- Geen lek boven zeeniveau: de `wy < sea_level_vox`-poort is onafhankelijk van de overhang-warp, dus een overhang of surface boven 1440 vox levert geen water op.
- Vult exact de "gap" tussen terrain-surface en zeeniveau → correct oceaan/meer-gedrag.

*Detail:* waterlijn ligt bij de bovenkant van voxel 1439 (179,875 m), dus 12,5 cm onder de exacte 180,0 m. Verwaarloosbaar voor MVP; geen bug.

## 2. Edge case: zeeniveau precies op chunk-grens (1440 = 45·32)
**Status: OK**

- De check gebruikt **wereld-Y** (`wy = origin.y + ly`), niet lokale Y, en `sea_level_vox` is globaal. Aangrenzende chunks komen dus altijd overeen: voxel 1439 → water, voxel 1440 → lucht. Geen naad/diskontinuiteit op de chunk-grens.

## 3. Edge case: surface hoger dan zeeniveau (geen water verwacht)
**Status: OK**

- Voor een kolom met surface ≥ 1440 vox is elke lucht-voxel `wy >= 1440` → poort `wy < sea_level_vox` faalt → blijft lucht. Correct: geen water op droog land/bergtoppen.

## 4. Regressie: bestaande materialen (dirt/grass/stone/sand/snow)
**Status: OK**

- Water raakt **alleen** de twee `continue`-branches (voorheen pure-air). De `classify()`-path (lijnen 322–325) en de biome/slope-local-fBm zijn ongewijzigd. Geen enkele solid-materiaal wordt overschreven of gemist.

## 5. Regressie: column-cache / streaming (`column_solid_cy_range`)
**Status: BUG** ⚠️

- `column_solid_cy_range` (lijnen 171–209) berekent `hi` puur uit `surface_height_m`:
  `let hi = (max_h + OVERHANG_AMP_CEIL).div_euclid(SIZE);` (lijn 198).
  `SEA_LEVEL_M` komt hier **nergens** in voor.
- Voor een kolom wíéns surface **onder** zeeniveau ligt (bijv. surface 100 m = 800 vox):
  - `hi = (800 + 28) / 32 = 25` (top-chunk van de streamed band).
  - Maar water vult de lucht van surface tot zeeniveau: voxels 801..1439 → chunks **cy 26..44**.
  - Die water-chunks vallen **buiten** `[lo, hi]` en worden door de streaming-loop nooit aangevraagd/gegenereerd → **oceanen/meren in valleien zijn onzichtbaar** (gaten in de wereld).
- Deze regressie is **stil**: geen enkele test vangt hem.
  - `column_range_never_excludes_solid_chunks` (1067–1107) scant alléén `cy in lo..=hi` en assert dat die binnen de range vallen (triviaal waar) — het controleert niet dat *alle* water binnen de range zit.
  - `chunks_below_sea_level_contain_water` (1148–1183) roept `generate_chunk` **direct** aan (omzeilt de streaming-range) → water bestaat wél, maar wordt niet gestreamd.

**Fix-aanbeveling (één regel, in `column_solid_cy_range`):**
```rust
// reken sea_level_vox uit (zie lijn 261) en verbreed hi:
let sea_level_vox = (SEA_LEVEL_M / voxel_core::coords::VOXEL_SIZE_M) as i64;
let hi = ((max_h + OVERHANG_AMP_CEIL).max(sea_level_vox)).div_euclid(SIZE);
```
Optioneel: voeg een streaming-test toe die voor een sub-zeeniveau kolom assert dat `generate_chunk(cy = sea_vox/CHUNK_SIZE)` binnen `column_solid_cy_range` valt.

*Aanvullend:* overweeg dezelfde `max(sea_level_vox)` op `lo`/onderkant niet nodig (water zit boven de surface, niet dieper). Alleen `hi` is defect.

## 6. Test-eis-versoepeling 0.98x (`column_reuse_is_faster_than_distinct_columns`)
**Status: WAARSCHUWING**

- Assert verzwakt van `col_ms < dist_ms * 0.9` (10% sneller) naar `* 0.98` (slechts 2% sneller) — lijnen 1010–1014.
- **Maskeert het GEEN water-regressie:** het water-werk (de `wy < sea_level_vox`-check + `set(WATER)`) wordt in beide paden (same-column én distinct-column) identiek per-voxel uitgevoerd. De *verhouding* dist/same wordt dus niet aangetast door water; de versoepeling weerspiegelt de inherent bescheiden echte cache-win (~6%, gedomineerd door solid-chunk-gen), niet een door water veroorzaakte achteruitgang.
- **Maar 0.98x (2% marge) zit op/onder de wall-clock-ruisvloer** → op een belaste/CI-machine kan de test flaky worden (pass/fail op load i.p.v. op echte regressie).
- **Aanbeveling:** assert de cache-win robuuster, bv. tel buffer-builds (same-column bouwt 1×, distinct-column k×) of controleer dat `COLUMN_HBUF_CACHE` voor dezelfde kolom maar één build doet, i.p.v. een 2%-wall-clock-marge. Of documenteer ten minste expliciet dat de test timing-gevoelig is.

## 7. Test `chunks_below_sea_level_contain_water`
**Status: OK** (met mini-kanttekening)

- Geldige correctheids-guard: scant sub-zeeniveau kolommen op materiaal `9` (lijn 1168).
- Kanttekening: hij assert niet expliciet dat water *boven* de surface zit (een onderwater-cave-water-voxel zou ook volstaan). In de praktijk valideert hij wel de zee-vulling, dus acceptabel.

## 8. Test `terrain_has_caves_and_overhangs` (AIR of WATER = niet-solid)
**Status: OK**

- Lijn 857 (`if m == 0 || m == 9`) telt onderwater-caves (water-gevuld) correct als "niet-solid". Logisch juist na de water-wijziging.

---

## Verdict

**NEEDS FIX** ⚠️

De water-generatie-logica zélf is correct: juiste kolom gevuld, geen lek boven zeeniveau, geen materiaal-regressie, chunk-grens en "surface hoger dan zeeniveau" netjes afgehandeld, en de 0.98x-versoepeling maskeert géén water-bug. **Echter**, `column_solid_cy_range` (aspect 5) houdt geen rekening met `SEA_LEVEL_M`: water boven een lage surface tot aan zeeniveau valt buiten de streamed `hi`, waardoor oceanen/meren in valleien niet gestreamd en dus onzichtbaar worden. Een één-regelige fix (`hi = max(max_h + OVERHANG_AMP_CEIL, sea_level_vox).div_euclid(SIZE)`) lost het op. Geen source-code gewijzigd door deze audit.
