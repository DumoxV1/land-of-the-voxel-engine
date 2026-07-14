# Spike S-10 — `voxel-gpu` (wgpu renderer op de RTX 4080, headless/offscreen)

**Datum:** 2026-07-15
**Fase:** Fase 2 client-shell (ADR-0004: Rust + Bevy/wgpu). Eerste concrete GPU-renderer.
**Methode:** strict TDD waar zinvol; eerst een technische FEASIBILITY-PROBE (wgpu op de hardware).
**Autonomie:** volledige volmacht gebruiker (2026-07-15) — GPU-renderer gewenst vóór hij test.

## Doel
Een `voxel-gpu` crate die de bestaande `voxel-mesher::greedy_mesh`-output (triangles) rendert
via **wgpu** op de GPU (Vulkan op Windows, RTX 4080 Super) — eerst offscreen naar een PNG, zodat
de gebruiker de engine op de GPU ziet draaien zónder dat er eerst een volledige Bevy-client nodig
is. Later: een winit-window voor interactieve weergave.

## Visuele stijl (uit gebruikers-referentie)
- ~90% *Lay of the Land*: sfeervolle, filmische diorama-voxelwereld; zachte, warme belichting;
  uitnodigende openwereld-survival-sändbox-look; blokkerig maar met depth/atmospheer.
- ~10% *John Lin*: scherpe "pixel-art"-voxel-uitstraling, heldere materialen, strakke edges.
- Concreet: per-normaal/directionele belichting (zoals de software-raster), een warme
  hemelkleur, eenvoudige fog/atmospheer voor diepte, en materialen met warme grass/dirt/stone
  tinten. Geen textures in de eerste GPU-spike (flat-shaded + normals + directional light + fog),
  wel de filmische kleur-grading-achtige tint.

## Stappen
1. **Feasibility-probe** (deze spike): minimale wgpu-pipeline die offscreen rendert en als PNG
   opslaat. Bewijst: (a) wgpu init op RTX 4080, (b) offscreen readback werkt, (c) build op
   Windows/MSYS. Als dit faalt, stop ik en rapporteer de blocker.
2. **Mesh → GPU**: `greedy_mesh`-triangles uploaden als vertex-buffer; per-vertex position +
   normal + material-id; uniform camera; vertex-shader projecteert, fragment-shader doet
   normal-based shading + fog + materiaal-tint.
3. **World → scene**: meerdere chunks (zoals `render_world`) via de bestaande coord-offset.
4. **Demo-PNG** (offscreen) + later winit-window.

## Acceptance criteria (na probe)
- [ ] wgpu initialiseert een Vulkan-device op de RTX 4080 (geen fout).
- [ ] Offscreen render → PNG met zichtbare geometrie (geen lege/CRTL-frame).
- [ ] Buildt op Windows (cargo build), géén systeem-Vulkan-SDK vereist (wgpu bundelt).
- [ ] Renderer-agnostisch: `voxel-gpu` dependeert op voxel-core/mesher/world, niet op voxel-render
      (de software-raster blijft bestaan als fallback/tooling).

## Risks
- wgpu op MSYS/bash: Vulkan-loader moet de NVIDIA-driver vinden. Als de driver niet zichtbaar is
  voor wgpu, faalt de probe — dan rapporteer ik en zoek ik een fallback (bijv. wgpu met
  `vulkan-portability` / dx12-backend).
- Build-tijd: wgpu +相依 crates compileren traag (~minuten). Geduld.
