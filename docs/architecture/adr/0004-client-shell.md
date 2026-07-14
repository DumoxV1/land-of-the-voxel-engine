# ADR-0004: Client Shell for the First Playable Slice

- **Status:** Proposed (pending Fase 2 benchmark gate)
- **Datum:** 2026-07-15
- **Deciders:** independent technical decision reviewer (gratis model, geen benchmarks gedraaid); bekrachtigen na Fase 2 spikes. Gedelegeerd door de gebruiker (volmacht 2026-07-15).

## Context
Fase 2 vereist één client-shell voor de eerste slice (2–8 spelers, persistent bewerkbare wereld,
Windows desktop eerst, FP/TP-camera, blocky greedy meshing). De voxel-core (`voxel-core`,
`voxel-mesher`, `voxel-worldgen`, `voxel-render`) is pure Rust en renderer-agnostisch (ADR-0002)
en bewezen headless zonder GPU. De authoritative server is GPU-vrije pure Rust (ADR-0003). Twee
gelijkwaardige kandidaten waren Godot 4 + GDExtension en Rust + Bevy/wgpu. De gebruiker is een
niet-technische bouwer die snel een runnable artifact nodig heeft; hardware is RTX 4080 Super
(Vulkan), 32 GB RAM, Core Ultra 7 265K, Windows-doel.

## Decision
Neem **Rust + Bevy/wgpu** als client-shell voor de eerste slice. De pure-Rust core wordt native
geconsumeerd (geen FFI), de client en headless server delen één workspace, en het render-pad breidt
de bestaande software-rasterizer uit naar wgpu/Vulkan op Windows. Godot + GDExtension wordt
uitgesteld; het zou een C-ABI-shim vereisen om een al-complete Rust-core opnieuw bloot te leggen en
splits de codebase in twee ecosystemen.

## Consequences
- Positief: nul FFI/copy-overhead op de core-grens; één Rust-toolchain/debugger voor client+server;
  volledige eigendom van camera/input/render; renderer-agnostische core (ADR-0002) houdt Bevy-churn
  geïsoleerd tot de dunne client-shell.
- Negatief: input, physics en UI moeten zelf gebouwd worden (Godot levert die gratis); Bevy pre-1.0
  API-churn-risico; `bevy-voxel-engine` is pre-0.1 — pin een commit-hash.
- Actie: pin Bevy + `bevy-voxel-engine` commit; houd alle gameplay/sim in renderer-agnostische
  crates; de client-shell blijft het enige Bevy-afhankelijke component.

## Alternatives Considered
- **Godot 4 + GDExtension (afgewezen voor eerste slice):** beste editor/UI/physics-ergonomie, maar
  forceert een GDExtension-FFI om de bestaande Rust-core bloot te leggen en scheidt server (Rust)
  van client (GDScript/C++). Het research-memo's schatte 4 weken, maar die schatting ging uit van
  het bouwen van voxel-terrain vanaf nul; die kost is in Rust al betaald, dus het relatieve voordeel
  krimpt.
- **Custom C++20-stack (buiten scope):** hoogste overhead; gereserveerd als langetermijnoptie.

## Benchmark Gate (pre-lock-in)
Fase 2 MOET B-06 (determinisme-replay, verwacht neutraal) en B-07 (headless 2–8 client soak) draaien,
plus render-throughput: laad bestaande `greedy_mesh`-output in beide shells, capsule-camera, meet FPS
+ time-to-first-textured-frame op een identieke seeded 1 km³ wereld op de RTX 4080. Minimale spike:
scaffold beide clients die `voxel-mesher` voeden; lock B als het target-FPS haalt met <1 week
integratie terwijl Godot >2 weken FFI nodig heeft.

## Herzieningstrigger
- Bevy-churn forceert >2 weken re-pin/rewrite van de client-shell.
- Gebruiker vereist snelle niet-technische content-authoring waar Godot's editor de iteratietijd
  materieel verkort.
- wgpu/Vulkan-driverproblemen op de dev-box blijken onoplosbaar.
- Een eerste-slice LOD/physics-feature blijkt dramatisch goedkoper in Godot.
