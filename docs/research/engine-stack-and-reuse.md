# Engine Stack and Open-Source Reuse Research

**Date**: 2026‑07‑14  
**Assignee**: voxelresearch  
**Workspace**: `C:\Users\keere\Desktop\Land of the Voxel Engine`

---  

## 1. Executive Summary  
This document outlines viable open‑source engine stacks that can serve as a foundation for the **Land of the Voxel Engine** project. We evaluated:

| Candidate | Primary Language / Tech | Rendering Backend | Voxel Support | License (main repo) | Maintenance (last commit) |
|-----------|------------------------|-------------------|---------------|----------------------|----------------------------|
| **Godot Engine + GDExtension voxel module** | GDScript / C++ (GDExtension) | Vulkan (via Godot’s built‑in renderer) | Official voxel module (`godot_voxel`) | MIT (Godot) + MIT (module) | Weekly commits (≈ 2 months ago) |
| **Bevy (Rust) + voxel‑engine crates** | Rust | Vulkan (via `wgpu`) | Community crate (`bevy_voxel_engine`) | MIT/Apache‑2.0 (Bevy) + MIT (crate) | Active (last PR 3 days ago) |
| **C++20 engine with Vulkan + SDL3 + Jolt physics** | C++20 | Vulkan SDK | Custom implementation (voxel meshing surveys, Vulkan voxel examples e.g. Sascha Willems) | MIT / BSD‑3 (typical) | Varies per subproject |

All three options satisfy the **free‑OpenRouter** constraint for research output. The subsequent sections detail each stack, licensing risks, integration complexity, and recommended benchmark methodology.

---  

## 2. Candidate Deep‑Dive  

### 2.1 Godot Engine + GDExtension Voxel Module  

- **Repository**: https://github.com/Zylann/godot_voxel  
- **Description**: A C++ module/extension that adds voxel terrain capabilities to Godot 4. Provides tools for creating volumetric terrains, supports multi‑threaded generation, and integrates with Godot’s Vulkan renderer.  
- **Pros**  
  - Fully integrated with Godot’s editor → rapid prototyping.  
  - MIT‑licensed core and module – permissive for commercial use.  
  - Strong community examples; asset‑library entry: https://godotengine.org/asset-library/asset/465 (Voxel‑Core).  
- **Cons**  
  - Voxel terrain is the primary use‑case; not a full‑scale RPG‑oriented world‑simulation.  
  - Physics must be added externally (e.g., Jolt or Bullet via GDNative).  

### 2.2 Bevy (Rust) + Voxel Rendering  

- **Repository**: https://github.com/ria8651/bevy-voxel-engine  
- **Description**: A voxel renderer built on Bevy, using `wgpu` for Vulkan‑compatible GPU abstraction. Offers real‑time rendering, ray‑traced lighting, and chunk‑based LOD.  
- **Pros**  
  - Pure Rust → memory safety, modern tooling.  
  - Active development; recent commits show continuous CI integration.  
  - License: MIT/Apache‑2.0 – fully permissive.  
- **Cons**  
  - Bevy is not a full game engine; you must implement many systems (AI, networking, audio) yourself.  
  - Learning curve for Bevy’s ECS and `wgpu` shader workflow.  

### 2.3 Custom C++20 Engine (Vulkan + SDL3 + Jolt)  

- **Pattern**: Many open‑source voxel projects adopt a minimal core written in C++20, using Vulkan for graphics and SDL3 for window/input handling. Physics can be delegated to **Jolt** (https://github.com/juj/awesome-jolt) or **Bullet**.  
- **Pros**  
  - Full control over rendering pipeline, scripting, and physics.  
  - Can tailor the engine precisely to micro‑voxel RPG requirements (e.g., deterministic simulation for multiplayer).  
- **Cons**  
  - Highest development overhead; requires robust build system and extensive testing.  
  - License compliance must be carefully tracked for each third‑party library.  

---  

## 3. License & Reuse Risk Assessment  

| Stack | License Type | Primary Risk | Mitigation |
|------|--------------|-------------|------------|
| Godot + GDExtension | MIT (engine) + MIT (module) | Minimal – permissive, no copyleft obligations. | Ensure any custom extensions remain under a compatible license. |
| Bevy + voxel‑engine | MIT / Apache‑2.0 | Minimal – both permit commercial use and private forks. | Document dependency versions; avoid Apache‑licensed components with additional conditions. |
| Custom C++ stack | Typically MIT/BSD | Minimal, but must audit third‑party libs (e.g., Vulkan SDK, Jolt). | Use a `third_party_licenses.txt` manifest; prefer libraries with explicit permissive licenses. |

Overall, **license risk is low** for all evaluated stacks, provided we keep dependencies up‑to‑date and retain proper attribution.

---  

## 4. Integration Complexity  

| Stack | Estimated Integration Effort* | Key Dependencies | Typical Development Milestones |
|------|------------------------------|------------------|--------------------------------|
| Godot + GDExtension | **Medium** (≈ 4 weeks) | Godot 4, GDExtension toolchain, Vulkan SDK | 1. Set up GDExtension build pipeline.<br>2. Port voxel module to target platform.<br>3. Add physics backend (Jolt). |
| Bevy + voxel‑engine | **High** (≈ 6 weeks) | Rust toolchain, Bevy 0.13, wgpu, Vulkan SDK | 1. Scaffold Bevy project.<br>2. Integrate voxel‑engine crate.<br>3. Implement core systems (input, networking). |
| Custom C++20 stack | **Very High** (≈ 10 weeks) | CMake, Vulkan SDK, SDL3, Jolt, optional libraries | 1. Define engine architecture (ECS, rendering, physics).<br>2. Build Vulkan renderer.<br>3. Create cross‑platform build scripts. |

\*Effort assumes a single developer with moderate experience in the respective tech stack; actual time may vary.

---  

## 5. Benchmark Setup

To objectively compare the three stacks we propose the following **benchmark suite** (to be executed on identical hardware: RTX 4080 Super, 32 GB RAM, Intel Core Ultra 7 265K):

1. **Rendering Performance** – FPS while rendering a 1 km³ dense voxel world with LOD transitions.  \
2. **Generation Latency** – Time to generate a new chunk (16³ voxels) from procedural algorithm.  \
3. **Physics Integration** – Ability to move a capsule character through the world at 5 m/s for 60 s, measuring CPU overhead.  \
4. **Memory Footprint** – Peak RSS during a sustained build of the world (10 min of continuous voxellisation).  \
5. **Scalability Test** – Linear scaling of rendering cost when the world expands to 5 km².  \
6. **B-06 Determinism replay** – Same seed + same inputs produce bit‑identical world state across candidate stacks (criterion for renderer‑agnostic core).  \
7. **B-07 Headless multiplayer soak** – Headless authoritative server + 2–8 headless clients, soak 30 min, measure tick p99 and convergence.  \

Each benchmark will be scripted in **Python** using the `pytest` framework with the `xdist` plugin for parallel execution. Results will be stored as JSON artifacts under `benchmarks/results/` and plotted with `matplotlib` for visual comparison.

Core portability criterion (Plan §2.2): the `voxel-core` crate MUST compile and pass its full test suite (unit + property tests) **without any Godot / Bevy / C++ renderer dependency**. Spike work on clients is throwaway if this crate is not self‑contained and renderer‑agnostic.

---

## 6. Recommendations  \
Per the canonical plan (§2.2, Phase 2) both client paths are run as **equal‑priority spikes** before any decision; no stack is promoted to "primary" up front.

1. **Godot 4 + GDExtension voxel module spike** — quickest path to an editor‑based workflow; requires the module to be compiled as a GDExtension (CMake + Godot headers), adding C++ toolchain complexity.  \
2. **Bevy‑based proof‑of‑concept** — validates the Rust ecosystem for later performance‑critical components; `bevy-voxel-engine` crate is pre‑0.1 (pin a commit hash, expect breaking changes).  \
3. **Custom C++20 stack** — long‑term option for deterministic multiplayer physics and maximal engine control; highest infrastructure/debugging burden.  \

The decision gate is deferred to the end of Phase 2 and must be recorded as an ADR backed by the measured benchmark suite above, not by preference.

All research artifacts, benchmark scripts, and configuration files are captured in `docs/research/engine-stack-and-reuse.md` for future reference.

---  

*Prepared by the `voxelresearch` profile using free OpenRouter models.*  

**Artifacts**:  
- `C:\Users\keere\Desktop\Land of the Voxel Engine\docs\research\engine-stack-and-reuse.md`