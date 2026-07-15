# Terrain Gen 2.0 — 3D density field (Stap 4)

**Datum:** 2026-07-15
**Status:** ✅ GEÏMPLEMENTEERD + GEverifieerd (2026-07-15, optie A gekozen en gebouwd)
**Probleem:** Huidige `voxel-worldgen` is een 2D hoogtekaart met 1-voxel shell
(`surface_height_m(x,z)`). Geen grotten, geen overhangs, geen canyons, en de
walkability-invariant (octaves ≥128 vox) maakt het vlak. Gebruiker ziet "alles erg
vlak, bijna geen hoogteverschil, geen grotten".

## Research-bevindingen (best practices / SOTA)

- **Minecraft**: 2D hoogte (continentaal + biome-ruwheid) + **3D density noise** voor
  grotten/overhangs. Caves vaak als aparte carve-pass.
- **Vintage Story**: multi-stage pipeline — landforms via octave-thresholds +
  "Terrain Y key positions/thresholds", daarna aparte **caves**-pass. Rocks in strata.
- **gamedev.stackexchange (caves+overhangs)**: tussenweg = heightmap als basis + 3D
  density-field dat overhangs/caves toevoegt; of 2e carve-pass voor grotten langs
  steile hellingen. "Puurd 3D noise = alien/sponge terrain".
- **StackOverflow (overhangs)**: puur 3D noise → "sponge terrain" (luchtgaten). Je MOET
  combineren met een surface-term, anders geen loopbare grond.

**Kernles:** een density field `d(x,y,z)` waarbij solid ⟺ `d > 0`, met de surface-term
die de loopbare grond domineert en een 3D warp die overhangs (boven surface) en caves
(onder surface) toevoegt. Seamless, chunk-parallel, geen "sponge".

## Voorgestelde aanpak (hybride density-field, optie A)

```
density(x,y,z) = (surface_height_m(x,z) - y)            // basis: loopbare grond
               + overhang_warp(x,y,z)                    // 3D noise: richels/overhangs boven surface
cave(x,y,z)    = 3D_ridged_noise(x,y,z) > threshold      // grotten-netwerk onder surface
solid ⟺ density > 0  EN  NOT cave(x,y,z)
```

- `surface_height_m` + biome blijven (bewezen, filmisch, walkable).
- `overhang_warp`: lage-freq 3D value/ridged noise, alleen actief in een band rond de
  surface (bv. ±8 m) → natuurlijke overhangs zonder de grond te breken.
- `cave`: 3D noise threshold → vertakkende grotten onder de surface. Met de nieuwe
  zonlicht-BFS (Stap 3) worden die nu écht donker gerenderd.
- Material-classificatie blijft biome-gedreven, nu op basis van `y` vs `surface_height`.

## Performance (bestaande caches blijven geldig)

- `column_height_buffer` / `column_solid_cy_range` blijven: de surface-term is nog
  steeds een pure f(x,z). De 3D warp is goedkoop (1-2 extra fBm-evals per voxel-column).
- `MAX_SURFACE_M` early-out blijft; caves/overhangs zitten binnen de surface-envelope.
-walkability-test blijft groen (surface-term domineert bij y≈surface).

## Acceptance criteria (TDD)

1. `terrain_has_caves_and_overhangs`: scan een gebied, beweer dat er voxels solid zijn
   BOVEN de lokale `surface_height_m` (overhang) én lucht-pockets ONDER de surface
   (grot) binnen enkele chunk-Y-lagen.
2. `terrain_is_walkable` blijft groen (surface-term domineert).
3. `chunks_span_multiple_y_layers` blijft groen (caves gaan dieper).
4. `chunk_gen_stays_fast` blijft groen (budget < 1500 ms / 200 chunks).
5. Deterministisch + seed-isolatie blijft groen.
6. Live-client: gpu_window_main toont nu heuvels + overhangs + donkere grotten.

## Implementatiestappen (na akkoord)

1. `density(x,y,z)` + `cave()` toevoegen in `voxel-worldgen/src/lib.rs`.
2. `generate_chunk` herschrijven: per (lx,lz) column de surface-term, per (ly) de
   density+warp+cave evalueren i.p.v. `wy < h` check.
3. `classify` aanpassen voor 3D (rock-strata banden optioneel).
4. Tests: nieuwe `terrain_has_caves_and_overhangs`; bestaande behouden/afstemmen.
5. `cargo test -p voxel-worldgen` + `client_smoke` + gpu_window_main visueel.
6. Plan + PROJECT_STATE + SESSION_HANDOFF bijwerken, commit + push.

## Implementatie-notitie (2026-07-15, afwijkingen t.o.v. voorstel)

- **`overhang_warp` is alleen OMHOOG** (`warp = (overhang*0.5+0.5).max(0)`): een negatieve
  warp zou de surface onder de heightfield induiken → grass-cap weg + walkability gebroken.
  De overhangs zijn daardoor conservatief (kleine richels), niet sculptureel.
- **Solid body tot y=0**: de oude 1-voxel shell (`BEDROCK_DEPTH=1`) is verwijderd. De
  ondergrond is nu een massief steen-lijf (caves als lucht-pockets in een ~12 m band onder
  de surface). Zijwanden + grotten zijn nu zichtbaar van opzij/onder. `column_solid_cy_range`
  vereenvoudigd naar `[0, (max_h + OVERHANG_AMP_CEIL)/32]`.
- **`classify` geeft STONE** voor voxels boven de heightfield (overhangs renderen echt,
  niet als AIR die wegvallen).
- **`CAVE_BAND_DEPTH=96` vox (~12 m)**, `CAVE_THRESH=0.5` op `fbm3` (één breed octaaf, 96 vox
  periode → 12 m-grotnetwerken). Top-3 voxels onder surface nooit cave (vloer blijft intact).
- **Nieuw:** `fbm3` + `hash3` (8-hoek trilineaire value-noise in 3D), `OVERHANG_AMP_VOX=28`
  (~3,5 m richels/kliffen, verhoogd van 6 op 2026-07-15 voor zichtbare variatie),
  `OVERHANG_OCTAVES` = 2 octaven (128 vox @0,7 + 48 vox @0,3) voor gevarieerde overhangs,
  `MAX_SOLID_M` (surface+overhang bound voor de air-chunk early-out).
- **Tests:** nieuwe `terrain_has_caves_and_overhangs` (Rood→Groen); 5 bestaande aangepast
  aan de solid-body. **19/19 worldgen groen**, workspace 36/36 binaries, 28/28 GPU-lib
  (zonlicht) onaangetast. Ad-hoc kwantificatie: 16/64 sample-kolommen hebben caves,
  2/64 overhangs → wereld is nu 3D.
- **Volgende (optioneel):** grotere overhang-amplitude + rots-strata banden in `classify`;
  grot-netwerken met vertakkingen (meer octaven in `cave`); LOD/clipmap vóór volledige
  view-distance (VBO-groei bij massieve ondergrond).
