# Filmic Micro-Voxel Texturing (12.5cm, wgpu 0.30, RTX 4080S)

## Core principle: never bind per-voxel
Greedy meshing already gives a *flat material index per triangle*. Store that index in the vertex/instance data; the shader indexes a `texture_2d_array` by it. VRAM scales with **#materials × tex-size**, not voxel count. This alone defeats the 1-texture-per-voxel blowup.

## Prioritized recommendations

**P0 — Texture arrays, one array per map channel (do first)**
- Create arrays as `Texture::create_view(TextureViewDimension::D2Array)`. Bind as `BindingType::Texture { view_dimension: D2Array, multisampled:false }`.
- WGSL: `texture_2d_array<f32>` + `textureSample(tex, samp, uvw, mat_id)`. Use BCn compression (Vulkan native on 4080): `Bc7RgbaUnorm` (albedo), `Bc5Unorm` (normal xy), `Bc4Unorm` (ORM/height). 4× VRAM savings vs rgba8.
- Layout: `group(1)` = read-only storage `array<Material>` (PBR params); `group(2)` = albedo array + normal/ORM array + one anisotropic sampler.
- *Tradeoff:* ~3 texture binds total regardless of material count. VRAM for 128 materials @2K BC7 ≈ 128×4.2MB ≈ **540MB**; trivial on 16GB.

**P1 — Triplanar projection (kills UV stretch from greedy quads)**
- Big greedy quads break traditional UVs. Sample each array 3× along world X/Y/Z, blend by `pow(abs(N), k)` weights. No UVs needed → no stretching, no seams.
- *Tradeoff:* 3× samples per map. On 4080S that's ~5% FPS vs single-sample. Worth it for filmic look.

**P2 — Anisotropic filtering + PBR per voxel type**
- `SamplerDescriptor { anisotropy_clamp: 16, mag_filter:Linear, min_filter:Linear, mipmap_filter:Linear, ..Default }` (linear required when aniso>1; use *-srgb formats for albedo so sRGB decode is free).
- PBR params live in the `Material` storage buffer indexed by mat_id: albedo tint, metallic, roughness, emissive, normalScale, tiling. Decoupled from texture memory → cheap per-type variation.
- *Tradeoff:* 16× aniso adds ~2–4× sample cost at glancing angles; <5% FPS on this GPU.

**P3 — Procedural detail + hero 4K (the "filmic" layer)**
- Blend a high-frequency WGSL value/Worley noise or a small 512 detail-normal array over the base for micro-detail without inflating base textures.
- Satisfy "4K+": keep bulk materials @1–2K; reserve ~8–16 **hero** materials (walls, hero props) at true 4K BC7 ≈ 16×16.8MB ≈ **270MB**. Total stays <1GB.
- *Tradeoff:* 4K hero reads cost bandwidth; ~10–15% FPS if every surface is 4K+16× aniso. Confine 4K to hero/near-camera surfaces.

## VRAM / FPS summary (vs current flat-color)
| Config | VRAM | FPS impact | Notes |
|---|---|---|---|
| Flat color (now) | ~0 | baseline | no detail |
| P0+P1, 1–2K BCn, 128 mats | ~0.5–1 GB | −5% | recommended default |
| +P2 aniso 16× | same | −10% | sharp glancing angles |
| +P3 hero 4K (16 mats) | +0.3 GB | −12–15% | filmic hero surfaces |
| Naive 1-tex/voxel @4K | **blows up (GBs→TB)** | unrunnable | avoid |

## Integration checklist (wgpu 0.30)
1. `TextureDimension::D2Array`, non-filterable only if needed; prefer filterable BCn/srgb.
2. Bind group: globals | `var<storage,read> materials` | `albedoArray`,`normalOrmArray`,`sampler`.
3. Mesh: append `mat_id` (u32) per vertex from greedy material index.
4. Fragment: triplanar sample ×3, mix procedural detail, apply ORM + tint, output PBR.
5. Generate mipmaps per array layer (wgpu `CommandEncoder` blit or `utils`); required for aniso + triplanar LOD.
