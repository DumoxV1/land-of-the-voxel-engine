# ADR-0005 — Micro-voxel resolutie (12,5 cm)

**Status:** Accepted (2026-07-15, autonoom besluit onder gebruikersvolmacht)
**Context:** gebruiker wil micro-voxels "tussen 9,5–13,5 cm", streven **12,5 cm (1/8 m)** — de
"Lay of the Land / Voxtopolis / John Lin / Tantan"-richting. Huidige engine werkt impliciet op
**1 voxel = 1 m** (chunk = 32³ voxels = 32 m; camera eye [16,55,90] voor 2×2 chunks).

**Besluit:**
- **1 voxel = 12,5 cm = 0,125 m** (constante `VOXEL_SIZE_M`).
- **`CHUNK_SIZE` blijft 32 voxels** → chunk wordt **4 m × 4 m × 4 m** (was 32 m).
- Wereld-coördinaten blijven **integer in voxel-eenheden** (geen wijziging aan `WorldVoxel`/
  `ChunkCoord`/`LocalVoxel` of de Euclidean-divisie). "Meter" is alleen een afgeleide schaal
  voor camera, player-physics en benchmark-claims.
- **1 km² = (1000 / 4)² = 62.500 chunks** (was 1.024). View-distance in chunks wordt
  daardoor groter (radius ~60 chunks ≈ 240 m view) — parameter in de bench, géén
  architectuur-wijziging.

**Afweging (chunk-grootte):**
- Klein chunk (4 m / 32³): RAM per chunk ~16 KB (dense) tot minder (palette/uniform), resident
  set bij ~289 zichtbare chunks ≈ 4,6 MB — zéér schaalbaar. Worldgen/meshing blijven snel
  (32³ = 32K voxels/chunk, zelfde orde als nu). Beste match voor "écht fijne micro-voxels".
- Groot chunk (32 m / 256³): houdt 1 km² = 1.024 chunks, maar 256³ = 16,7M voxels/chunk →
  ~8–16 MB/chunk dense; 289 chunks ≈ 2,4–4,6 GB resident. Te zwaar voor sustained openwereld.
- Keuze: **klein chunk (4 m)**. Latere LOD (advies #5) kan grotere bricks/clipmap-ringen
  introduceren voor verre view-distance zónder de voxel-maat te veranderen.

**Gevolgen voor bestaande code:**
- `voxel-core/coords.rs`: `VOXEL_SIZE_M: f32 = 0,125` + `chunk_m_size() -> f32` helper.
- `voxel-gpu` camera's (`gpu_window`, `gpu_world`, bench): eye-hoogte in voxels
  (~14 voxels = 1,8 m i.p.v. 55), view-distance-radius in chunks omhoog.
- `voxel-worldgen`: fijnere noise-schaal (noise-grid-period in voxels, niet chunks) zodat 12,5 cm
  écht detail toont i.p.v. opgeschaalde 1m-terrain (blokkerig op kleine schaal).
- Player/physics (S-08): snelheden in m/s → voxels/s bij gebruik; eerst alleen camera getoond.

**Niet-vereist voor slice:** hiërarchische macro/micro-onderverdeling (canoniek plan §2.1) blijft
uitgesteld; vlakke 12,5 cm-resolutie is de eerste benchmark-configuratie.

**Alternatieven overwogen:** (a) voxel=12,5 cm + chunk=32 m (256³) — verworpen: RAM-explosie;
(b) voxel=10 cm — binnen band maar 12,5 cm is de onderzochte "sweet spot" (plan §10.2,
spelerhoogte 14–16 cellen) en een nette 1/8-m macht-van-twee (schone schaal-wiskunde).
