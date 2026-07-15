# Visual Upgrade — hogere heuvels + Lay-of-the-Land palette + echte 4K textures (Taak 5)

**Datum:** 2026-07-15
**Status:** ✅ GEÏMPLEMENTEERD + GEVERIFIEerd
**Probleem (uit live screenshots):** de wereld is vrijwel grijstinten. Drie oorzaken:
1. `TILE=16` — de "4K textures" van Mijlpaal 4 zijn 16×16 px gegenereerde hash-noise tiles.
   Geen mipmaps → moiré op afstand. Geen echte detail.
2. Shader `rock_mix` mixt 60% koude grijze `rock=[0.45,0.45,0.48]` op hellingen → heuvels grijs.
3. `material_tint` waarden zijn gedempt (lage verzadiging).

**Doel (gebruiker, autonoom):** hogere heuvels, palette enorm verhogen (Lay of the Land-stijl:
verzadigde biome-kleuren, warme steen i.p.v. koud grijs), kleuren naar 4K (echte texture-resolutie
+ mipmaps). Lay of the Land = voxel-sandbox met distinct, vibrant biomes, biome-blending,
glowing vegetation, weather — kleurrijk, geen grijstinten.

## Aanpak

### A) Hogere heuvels (worldgen)
- `surface_height_m` amplitude verhogen: mid-tier `40 * roughness` → `~90 * roughness` (max ~126 m
  i.p.v. 56 m). Dramatischere relief. `MAX_SURFACE_M` + `MAX_SOLID_M` bounds mee omhoog.
- Walkability blijft intact (surface-term domineert bij y≈surface; overhang-warp alleen omhoog).

### B) Palette enorm verhogen (Lay of the Land-stijl)
- `material_tint` → hoog-verzadigde, warme kleuren: gras diep groen, aarde rijk bruin,
  steen **warm beige/grijs** (niet koud grijs), zand goud-geel, sneeuw warm wit, rots roodbruin.
- Shader `rock`/`snow` kleuren aanpassen: rots = warm bruin-grijs (geen koude grijs), zodat
  hellingen kleur houden in plaats van grijs wegvallen. `rock_mix` verlagen (0.6→0.35) zodat
  de biome-tint doorschijnt.

### C) Echte 4K-scale textures
- `TILE` 16 → **1024** (1024² per materiaal, 9 materialen ≈ 60 MB met mipmaps — binnen VRAM).
- Tile-inhoud: echte **fBm-value-noise** per materiaal (meerdere octaven) i.p.v. `(x*73+y*191)%31`
  hash → natuurlijke steen/gras/grond structuur. Tint als basis, noise als luminantie-variatie.
- **Mipmaps** toevoegen (`mip_level_count` + `generate_mipmaps`) → geen moiré op afstand,
  scherpe nabij-detail (4K-scale). Anisotropy 16 (reeds).

## Acceptance criteria (TDD)
1. `terrain_has_taller_relief`: max surface-span over een gebied stijgt (hogere heuvels).
2. `material_palette_is_saturated`: nieuwe test — gemiddelde verzadiging van `material_tint`
   per materiaal ligt boven een drempel (geen grijstinten meer).
3. `texture_tiles_are_4k_scale`: nieuwe test — `TILE >= 1024` en mip_level_count > 1.
4. Bestaande `grass_surface_shows_texture_variation_not_flat_tint` blijft groen.
5. `cargo test -p voxel-worldgen` + `cargo test -p voxel-gpu` groen.
6. `client_smoke` 120/120, release `gpu_window_main` gebouwd.
7. Live: heuvels hoger, rijke kleuren, scherpe textures op nabij, geen grijstinten.

## Implementatiestappen
1. Worldgen amplitude + bounds.
2. `material_tint` + shader rock/snow.
3. Texture-gen (TILE 1024, fBm, mipmaps).
4. Tests (failing→green) + build + commit/push.
