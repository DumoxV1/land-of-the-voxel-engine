# HER-AUDIT — World-Water Fix (BUG aspect 5)

**Target:** `crates/voxel-worldgen/src/lib.rs` — `column_solid_cy_range` (regel ~197)
**Datum:** 2026-07-17
**Verdict: ✅ SAFE TO SHIP**

## Per-aspect bevindingen

1. **Regel 196 `hi` bereikt zeeniveau — OK**
   `let hi = ((max_h + OVERHANG_AMP_CEIL).max(sea_level_vox)).div_euclid(SIZE);`
   `.max(sea_level_vox)` dwingt `hi` ≥ zeeniveau-chunk. Voor sub-zeeniveau kolommen reikt
   de streamed band nu tot chunk 45 (180 m). Oude bug (alleen surface+overhang) is weg.

2. **Nieuwe test `sub_sea_level_columns_stream_water_to_sea_level` (regel 1114) — OK**
   Scand 80×80, voor elke sub-zeeniveau kolom geassert `sea_cy ∈ [lo,hi]`. Faal-bericht
   "oceaan onzichtbaar" is exact de oude symptoom. **Test PASSES.**

3. **Aangepaste test `column_range_never_excludes_solid_chunks` (regel 1066) — OK**
   Tightness-check nu guarded door `if !sub_sea` (regel 1103); sea-chunk draagt WATER
   i.p.v. terrain, dus terecht overgeslagen. Geen valse failure. **Test PASSES.**

4. **Geen regressie — OK**
   Alle 17 unit-tests + 5 spike-tests groen (`cargo test -p voxel-worldgen`),
   incl. materialen (`chunks_below_sea_level_contain_water`, `underground_is_solid_stone_body`),
   column-cache (`column_cache_...`, `column_range_is_deterministic_...`) en streaming.

**Conclusie:** Bug aspect 5 is correct opgelost; fix + tests valideren; geen regressie.
