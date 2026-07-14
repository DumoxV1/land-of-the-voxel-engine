# Spike-plan: Fase-2 benchmark-gate (FPS op 1 km²) + view-distance streaming

**Doel:** de harde Fase-2 gate uit `ROADMAP.md` / alignment-log S-12b uitvoeren vóór ADR-0004 lock-in:
meet reproduceerbaar de **FPS** van de wgpu-client bij een **1 km²** wereld op de RTX 4080 Super,
met een **view-distance chunk-streamer** (S-12 deel 3 / advies #2) in plaats van alles-in-één-keer renderen.

**Schaal-afleiding (geverifieerd uit gpu_window/gpu_world):**
- 1 chunk = `32³` voxels; worldgen gebruikt wereldcoördinaten 1:1, dus **1 voxel = 1 m, 1 chunk = 32 m**.
- 1 km² = 1.000.000 m² / (32² = 1024 m²/chunk) ≈ **977 chunks ≈ 32×32 chunks**.

**Aanpak (strict, geen wijziging aan bewezen S-10/S-12b code):**
1. `GpuScene` krijgt een kleine, geïsoleerde pub-fn `render_triangles(tris, camera) -> Result<()>`
   die naar een offscreen target rendert + `device.poll(wait)` (echte GPU-uitvoering), zónder
   readback/PNG-save. Meetbaar frame = encode + submit + poll. De bestaande
   `render_triangles_png` (S-10) blijft ongewijzigd in gedrag.
2. Nieuw `examples/gpu_bench.rs`:
   - Genereert een `World` (seed) van `side×side` chunks (default 32×32 = 1 km²).
   - View-distance-streamer: per frame worden alleen chunks binnen `radius` (default 8) van de
     camera getoond; nieuwe chunks worden ge-mesht (cache), verdwenen chunks blijven in cache
     (simpele LRU, geen rayon-pool nog — dat is Fase-2b #2).
   - Camera beweegt langs een pad (cirkel/vuistvol voorwaarts) zodat streaming + frametime
     realistisch worden.
   - Meet per frame de frametime (Instant rond `render_triangles`); na `frames` (default 300)
     worden **p50 / p95 / p99** + gemiddelde FPS berekend.
   - Schrijft `crates/voxel-gpu/bench_1km2.json` (side, radius, frames, tris_totaal,
     zichtbare_chunks, p50/p95/p99_ms, avg_fps, gpu_adapter).
3. Draai op RTX 4080 (background, notify). Analyseer.
4. Schrijf `docs/benchmarks/2026-07-15-fase2-fps-1km2.md`, update ROADMAP + alignment-log,
   commit + push (grens A).

**Acceptatiecriteria:**
- Bench draait zonder panic/crash op de GPU; JSON bevat geldige p50/p95/p99.
- FPS-meting is reproduceerbaar (2e run binnen ~10% van de eerste).
- Uitslag bepaalt de volgende stap: bij < target-FPS → eerst streaming/meshing-optimalisatie
  (advies #2/#3) vóór ADR-0004 lock-in; bij gezonde FPS → ADR-0004 naar Accepted.

**Scope-grens:** dit is een meet-harness, géén product-crate-wijziging van de renderer-pipeline.
Streaming-logica leeft in de bench (voorbeeld), niet in `voxel-gpu` zelf — pas als de meting
een echte architectuur vereist (Fase-2b #2) verhuist het naar een crate.
