# Tracy Profiler — Volledige Integratie (Plan + Onderzoek)

**Datum:** 2026-07-16 · **Status:** VOLTOOID (gecommit + geverifieerd) · **Autonomie:** volledig (user aanvaard)

## Doel
Real-time nanosecond-frame-profiling in de live GPU-client zodat we exact zien waar de
CPU-tijd naartoe gaat (chunk-gen, mesh, scheduler, draw, wgpu-submit) en welke worker
bottlenecks de FPS drukt. Verder: een herbruikbare **skill** `tracy-profiling-integration`
die dit patroon voor elke Rust/wgpu-engine vastlegt.

## Onderzoek (GitHub / intern / papers)
- **Tracy** (wolfpld/tracy): real-time, nanosecond, remote telemetry, hybrid frame+sampling
  profiler. C/C++/Lua/Python/Fortran first-class; Rust via C API (`rust_tracy_client`).
- **Rust-binding**: crate `tracy-client` (nagisa) — huidige `0.18.4` → `tracy-client-sys 0.28.0`
  → **Tracy protocolverensie v0.13.1**. Gebruik `tracy-client` met de `enable` feature-flag.
  ⚠️ Tracy stuurt LAN discovery-broadcasts + kan source/assembly blootstellen → **alleen
  conditioneel enable'n** via een cargo feature (`tracy`), nooit in de default build.
- **C API** (`public/tracy/TracyC.h`): `TracyCZone(ctx,active)` / `TracyCZoneEnd(ctx)` /
  `TracyCFrameMark` / `TracyCPlot(name,val)` / `TracyCMessageL(text)`. De Rust-crate
  wrapt dit in `Client::span(...)`, `frame_mark()`, `plot(name, val)`.
- **Rust API- Details (gverifieerd tegen tracy-client 0.18.4 source):**
  - `tracy_client::span!("name")` → returned een `Span` die de zone afsluit bij **drop**
    (dus altijd binden: `let _span = span!("name");`). Intern roept het `span_location!`
    aan (die macro is NIET gepubliceerd, dus je kunt `span!` niet zelf her-implementeren —
    gebruik de crate z'n `span!` direct).
  - `tracy_client::plot!("name", f64_val)` → macro (geen functie!). Val is `f64`.
  - `tracy_client::frame_mark()` → functie (geen macro).
  - `tracy_client::Client::running()` → lazy-start van de capture-thread (broadcast naar GUI).
- **Overhead**: ~50 ns per zone-markup (macro). Tientallen-duizenden zones/frame zijn
  haalbaar. Voor onze doelen: grove zones (frame, render_frame, gen-worker, mesh-worker,
  scheduler.plan, draw-loop) + per-frame plots (FPS, chunks, tris).
- **GPU-timing**: wgpu heeft geen directe Tracy-hook. Aanpak: CPU-zone rond `queue.submit`
  + (later) wgpu `TimestampQuery` voor echte GPU-durations. Eerste fase = CPU-zones.
- **Server/GUI**: gebruiker downloadt de **Tracy v0.13.1** profiler (prebuilt Windows x64
  `.7z` op GitHub Releases, vereist AVX2 — Core Ultra 7 265K OK). Start `Tracy.exe`,
  client verbindt automatisch via localhost discovery.

## Implementatie (wat er daadwerkelijk staat)
1. **Feature-flag** `tracy` in `voxel-client` Cargo.toml → `tracy-client = { version="0.18",
   features=["enable"], optional=true }`. Default OFF.
2. **Wrapper-module** `src/profiling.rs`:
   - `span!`, `plot!`, `frame_mark!` zijn **macros** (bij `cfg(not(feature="tracy"))` no-ops;
     bij `cfg(feature="tracy")` forwarden ze naar `tracy_client::{span, plot, frame_mark}`).
   - Bij no-op moet `span!` een waarde returnen (`()`) zodat `let _span = span!(..)` compileert.
   - `start()` functie: `Client::running()` (feature) / no-op (geen feature).
   - **NIET** de `profile!("name", { ... })` blok-wrapper gebruiken — die botst met de
     `tracy_client::span!`-expansie (mismatched delimiter bij blok-args). Gebruik in plaats
     daarvan `let _span = span!("name");` vóór de code die je wilt meten.
3. **Zones** (cfg-gated via de macro-import `use tracy_client::{plot, span};` bovenaan lib.rs):
   - `RedrawRequested` handler: `let _span = span!("frame");` + `frame_mark!();` per frame.
   - `render_frame`: `let _span = span!("scheduler_plan");` rond `scheduler.plan(...)`.
   - Worker-thread (job-loop): `let _span = span!("worker_job");` per `run_mesh_job`.
4. **Plots** (cfg-gated block in `render_frame`, na de draw): `plot!("fps", ...)`,
   `plot!("chunks", ...)`, `plot!("tris", ...)`. Waarden zijn `f64`.
5. **GPU**: CPU-zone rond `queue.submit` (fase 1). TODO: wgpu TimestampQuery (fase 2).
6. **Bouw**: `cargo run --release --features tracy --example gpu_window_main -p voxel-client`.

## Verificatie (2026-07-16, groen)
- Normale build: `cargo build -p voxel-client` → Finished, geen Tracy-dep.
- Tracy build: `cargo build --release --features tracy --example gpu_window_main -p voxel-client`
  → Finished. Zones + plots + frame_mark compileren.
- Suite: 30/30 voxel-gpu + 15/15 voxel-worldgen groen (macros compileren in beide configs).
- Live: gebruiker opent Tracy v0.13.1, draait client met `--features tracy`, ziet flamegraph.

## Bronnen
- Repo: https://github.com/wolfpld/tracy
- Rust crate: https://crates.io/crates/tracy-client (docs.rs/tracy-client)
- Manual (C API hoofdstuk 3.14): geladen uit lokale zip `tracy-master`
- NVIDIA Isaac Sim Tracy tutorial, IREE Tracy profiling (hybrid instrumentation+sampling)
- HN/Reddit: ~50 ns zone-overhead, tienduizenden zones/frame haalbaar

## Risico's / afwegingen
- **LAN broadcast**: default OFF via feature-flag. Nooit in release-build zonder explicit opt-in.
- **Protocolverensie**: client (v0.13.1) moet matchen met de GUI-versie die de gebruiker
  downloadt. Pin `tracy-client = "0.18"` (→ tracy-client-sys 0.28.0 → v0.13.1).
- **Compile-tijd**: `tracy-client-sys` bouwt een C++ translatie-unit (~tientallen ms, eenmalig).
- **Overhead in debug**: zones ~50 ns; verwaarloosbaar t.o.v. gen/mesh (ms-orde).
