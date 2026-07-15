# Mijlpaal 4 — 4K-texture-system (P0→P3)

**Doel:** de flat-color voxelrenderer vervangen door een filmisch texture-system:
texture-arrays + triplanar + PBR + BCn/sRGB op wgpu 0.30, zonder de 60+ tests te breken
en zonder FPS onder de huidige ~3750 avg (RTX 4080, 1 km²) te laten zakken.

**Canonieke bron:** `docs/research/2026-07-15-texture-4k-aanbeveling.md`.
**Voorwaarde:** strikt TDD — elke stap een failing test vóór implementatie.

## Acceptance criteria (per stap)

### P0 — Texture arrays + triplanar + PBR (verplicht) ✅ GEDAAN 2026-07-15
- `GpuScene` krijgt een `MaterialPbr`-storage-buffer (albedo tint, metallic, roughness,
  emissive, normal_scale, tiling) geïndexeerd op `material: u32`.
- Albedo als `texture_2d_array` (D2Array), één anisotropic sampler (`anisotropy_clamp: 16`,
  linear filters, sRGB albedo-formaat).
- WGSL fragment: triplanar sample langs wereld X/Y/Z, blend `pow(abs(N), k)`, pas
  PBR (tint × albedo, eenvoudige Lambert + ambient + fog) toe.
- **Failing→passing test:** `grass_surface_shows_texture_variation_not_flat_tint` — offscreen-PNG
  van een gras-quad (mat 2) toont na fix **>1 distincte groentint** op één vlak oppervlak
  (triplanar textuurvariatie), waar de oude flat-tint er exact één vlakke kleur gaf.
- Geen per-voxel texture-bind; VRAM schaalt met #materials × tex-size.
- **Bijkomende bug gefixt:** VBO-pool groeide ongeremd voorbij `max_buffer_size` (256 MB) bij
  grote view-radius → client-panic. Gepakt op device-limiet met veilige growth-stappen.
- **Geverifieerd:** live client capture toont 3072 unieke kleuren (was 434 bij wit-scherm-fix),
  gras/steen met zichtbare textuurvariatie. Benchmark 1 km²: p50=0.24ms, avg_fps≈3636
  (geen FPS-daling t.o.v. pre-texture 3753). Workspace 36 test-binaires groen.

### P1 — BCn-compressie + mipmaps
- Albedo `Bc7RgbaUnormSrgb`, normal `Bc5Unorm`, ORM `Bc4Unorm` (Vulkan native op 4080).
- Mipmaps per array-laag (encoder-blit of utils) — vereist voor aniso + triplanar-LOD.
- **Test:** texture-array heeft `mip_level_count > 1`; shader sampled zonder validation-error.

### P2 — Anisotropic + PBR per materiaaltype (reeds deels in P0)
- 16× aniso bij glancing angles; PBR-params per type uit storage-buffer.

### P3 — Procedural detail + hero 4K
- WGSL value/Worley detail-blend over de base voor micro-detail.
- ~8–16 hero-materialen op ware 4K BC7; bulk op 1–2K. Totaal < 1 GB VRAM.
- **Test:** hero-laag bestaat; near-camera surfaces gebruiken 4K-array.

## Niet-acceptabel
- 1-texture-per-voxel (blows up VRAM).
- FPS onder huidige baseline op 1 km².
- Tests breken of "groen" via aanname-ipv pixel-oracle.

## Stappen
1. Plan + alignment (deze file).
2. P0 failing test (pixel-oracle: gras toont >1 groentint).
3. P0 implementatie (storage-buffer + albedo-array + sampler + triplanar WGSL).
4. P0 groen + benchmark (FPS mag niet zakken).
5. P1 BCn + mipmaps (failing test: mip_level_count>1).
6. P2/P3 naar behoefte.
7. Commit/push + verslag.
