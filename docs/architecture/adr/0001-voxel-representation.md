# ADR-0001: Voxel Representation for Phase 1

- **Status:** Accepted (via free-model architect synthesis, reviewed against `review-initial-bundle.md`)
- **Datum:** 2026-07-14
- **Deciders:** voxelarchitect (synthese), voxelreviewer (review)

## Context
De engine moet een filmische, rijke micro-voxel openwereld ondersteunen zonder dat een uniforme
voxel-grid het RAM/disk/netwerk opblaast (risico R-1: OOM bij 150 km²). Het canonieke plan (§2.1,
§2.3, §3.2) eist hiërarchische sparse data, integer coördinaten en blocky/palette als laagste-risico
MVP. Vier representaties zijn onderzocht: blocky/palette + sparse bricks, smooth SDF/Marching
Cubes, sparse octree/DAG, en clipmap+LOD.

## Beslissing
Voor **Fase 1 (vertical slice)** gebruiken we **blocky/palette micro-voxels met hiërarchische sparse
bricks**:
- LOD0-cel ≈ 12,5 cm (hybride tot 10–25 cm toegestaan na benchmark).
- `8³` brick als kleinste sample-eenheid; `32³` meshblock; `256³` persistence-region.
- Per-chunk materiaalpalette (≤16 materialen), bitpacked materiaal-ID's.
- Drie chunktoestanden: uniform, palette-packed, dense.
- Procedurele basiswereld + alleen sparse edits persistent (geen volledige wereld naar disk).
- Integer wereldcoördinaten; `WorldVoxel`, `ChunkCoord`, `LocalVoxel` als aparte types; negatieve
  coördinaten via euclidische deling.

Smooth SDF/Marching Cubes/Transvoxel wordt **niet** verworpen maar uitgesteld: een parallelle
benchmark-spike kwantificeert de visueel-vs-performance tradeoff vóór elk besluit (Plan §10.3).

## Alternatieven
- **Smooth SDF + Marching Cubes/Transvoxel** — organischer, maar 30–60× hogere geheugenclaim (SDF vs
  blocky ratio nog ongemeten), complexe seams/material-blending, moeilijker determinisme/edits.
  (voxel-data Candidate 2)
- **Sparse Voxel Octree/DAG** — sterke compressie voor statische data, maar dure updates/pointer-
  chasing; ongeschikt voor frequente edits. (voxel-data Candidate 3)
- **Clipmap + LOD hybrid** — adaptief aan view-distance, maar edits propagaten over alle niveaus.
  (voxel-data Candidate 4)

## Bewijs / benchmarks
- John Lin, "The Perfect Voxel Engine": allocation/tagging/conversion pipeline, ≤4 B/actieve voxel
  als target.
- Mikolalysenko, "Meshing in a Minecraft Game": greedy meshing trade-offs (greedy ≤ 1,5× culled
  triangles als doel).
- Voxtopolis / Lay of the Land: sparse format referentie (<50 MB disk per 1 km² onbewerkte regio).
- Teardown save-format (Dennis Gustafsson, GDC 2022): deterministic serialize round-trip.
- **Nog te meten (B-04/B-05):** benchmarkcijfers uit sociale media zijn "unverified" — S-01/S-02
  reproduceren ze op RTX 4080 Super met Criterion voordat "Production Readiness" hoger dan Medium
  wordt gezet.

## Gevolgen
- Coördinaten-, chunk- en palettelogica moeten renderer-onafhankelijk zijn (zie ADR-0002).
- Meshing-spike S-02 bouwt naïve→culled→greedy met golden fixtures (lege/volle/checkerboard/
  grensoverschrijdende chunks).
- Memory budget wordt bewaakt via S-01 property/benchmark-harnas.

## Herzieningstrigger
- S-02 toont dat smooth SDF een superieure visueel/performance-ratio haalt binnen het RAM-budget.
- Benchmarks tonen dat blocky palette het 4 B/voxel-budget structureel overschrijdt op doelhardware.
