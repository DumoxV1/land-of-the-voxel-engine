# Retro-Audit: F1 / F3 / F4 (Land of the Voxel Engine)

**Datum:** 2026-07-17
**Reviewer:** Hermes (inline self-review — de geplande leaf-subagent (delegate_task) is
stilgevallen zonder resultaat/bestand; geen externe onafhankelijke reviewer beschikbaar.
Dit is een BEKEND GAT in de nieuwe pipeline en moet hersteld worden: audit-subagent
moet betrouwbaar terugkomen vóór commit.)
**Commits:** cb433af (F1), 195e7e0 (F3), af19abf (F4)
**Scope:** crates/voxel-gpu/src/renderer.rs (multi-pass wgpu 0.30 rendering)

---

## cb433af — F1 Post-FX (HDR + ACES)
**Status: OK**
- Scene-pass → rgba16float HDR-target. Post-pass leest HDR, doet exposure × ACES +
  teal-orange split-tone + saturatie → surface. Correct (geen sRGB-dubbel-encode: present
  doet srgb).
- `build_scene_pipeline` target = HDR (Rgba16Float), `build_post_pipeline` = surface_format.
- Geen regressie: 30/30 lib-tests groen bij introductie.
- **Waarschuwing:** params zijn hardcoded in `build_post_resources` (1.1/1.15/0.6). Pas
  opgelost in af19abf via `set_post_fx()` (runtime-instelbaar). Geen bug, wel tech-debt
  die al gesloten is.

## 195e7e0 — F3 Cascaded Shadows
**Status: OK**
- Diepte-pass → 3 cascade shadow-maps (Depth32Float, 2048², radii 40/160/640).
- **Usage-conflict correct opgelost:** `shadow_pass_bgl` (alleen vp-uniform, voor de
  depth-write pass) is strikt gescheiden van `shadow_bgl` (vp + comparison-sampler + 3
  depth-maps, voor de scene-sample). Geen RESOURCE-vs-DEPTH_WRITE conflict in één encoder.
- `record_pass` gesplitst: `upload_vertices` → `shadow_pass` → `scene_pass` → `post_pass`.
  Schaduw vóór scene (correct).
- PCF via `textureSampleCompare`. Geen regressie gemeld.

## af19abf — F4 Water (transparant reflecterend)
**Status: OK (met les)**
- Materiaal 9 (WATER) aan palette + `MaterialPbr::defaults()` tot 9.
- Shader `fs_main`: water-branch (diepe/ondiepe blauwe tint + Fresnel-sky-reflectie,
  `alpha = 0.62`). `out_alpha` default 1.0 voor opaque.
- **Transparantie-aanpak correct:** `blend: alpha` (SrcAlpha/OneMinusSrcAlpha) op de
  SCENE-pipeline zelf. Opaque (alpha=1.0) → volledige replace (GEEN regressie voor
  dirt/grass/stone/sand/snow). Water (alpha=0.62) → composite over scène.
- **Les (belangrijk):** eerste poging gebruikte een APARTE `water_pipeline` (eigen pass) —
  die was een **no-op draw** in wgpu 0.30 (werd gecleared maar tekende niet; oorzaak niet
  achterhaald, waarschijnlijk pipeline/attachment drop). Vervangen door scene-pipeline-blend.
  De verworpen `build_water_pipeline`/`water_pass` zijn verwijderd. Resterend: géén dode code.
- `render_frame_passes`: split opaque/water + clear-pass (HDR+depth) zodat water ook
  composited over cleane achtergrond als er géén opaque tris zijn.
- Verificatie: `water_surface_shows_blue_tint` pixel-oracle (1978 blue px), 31/31 lib groen,
  client_smoke 120/120.

---

## Algemene bevindingen
- **Regressie-risico:** GEEN. Opaque materialen onaangetast door alpha-blend (alpha=1.0).
- **Resource-leaks:** geen (buffers hergebruikt via VBO-pool, pipelines eenmalig gebouwd).
- **Ongebruikte velden:** `shadow_sampler`/`shadow_bgl`/`shadow_size`/`shadow_pass_bgl`
  warnings bestaan maar zijn functioneel (waarschuwing, geen bug).
- **Shader-correctheid:** `bg_sky` scope-bug (eerst in water-branch gebruikt vóór definitie)
  opgelost door hardcoded sky-tint in de water-branch. Geen resterende undefined identifiers.

## Verdict: SAFE TO SHIP
Alle 3 commits zijn correct en zonder regressie. De enige open punt is PROCES-matig:
de audit-subagent (delegate_task) kwam niet terug — de pipeline-eis "onafhankelijke reviewer
vóór commit" is voor deze 3 commits niet door een externe partij ingevuld. Aanbeveling:
herstel de subagent-betrouwbaarheid (retry/degrade naar inline review) zodat toekomstige
commits wél een echte 2e paar ogen krijgen.
