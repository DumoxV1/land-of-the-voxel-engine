# Visuele bronnotities — transcriptloze / beeldgedreven video's

Datum inspectie: 2026-07-15

## P5M_QiamXvw — Adaptive Voxel Grid (Human)

- Titel: **Adaptive Voxel Grid (Human)**
- Kanaal: **Cartesian Caramel**
- Publicatiedatum: 2022-12-07
- Duur: 10 seconden
- Metadata/hashtags: Blender, Geometry Nodes.
- Transcript: uitgeschakeld.
- Directe visuele inspectie: 4×4 contact sheet, 16 gelijkmatig verdeelde frames.

### Waarneembaar

De video toont een humanoïde vorm die progressief wordt opgebouwd/verfijnd. De eerste frames tonen zeer grote, afgeronde rechthoekige cellen; daarna ontstaat een torso/hoofd/armen met een mix van grote en kleine cellen; de laatste frames bestaan vrijwel volledig uit kleine groene cellen met incidentele gele/rode cellen. De celgrootte varieert ruimtelijk en de silhouetdetailgraad neemt toe. Op de getoonde afstand zijn geen opvallende gaten tussen resolutieniveaus zichtbaar.

### Niet bewezen door deze video

De clip bewijst niet dat er een runtime voxel-engine, octree, AMR-grid, dynamische streaming, collision, destructie of crack-free LOD-algoritme achter zit. Gezien de metadata is het waarschijnlijk een Blender Geometry Nodes-visualisatie/procedurele modellering. Het effect is inspiratie, geen architectuurbewijs.

### Onderzoekssplitsing

1. Reproduceer het artistieke effect in Blender/Geometry Nodes alleen als visuele referentie.
2. Onderzoek apart echte runtime adaptive voxel grids: octrees, sparse voxel DAGs, AMR, clipmaps, Transvoxel/crack stitching, edit propagation, collision en GPU traversal.

## fS3VVlx49ao — I Built a C++ Micro Voxel Engine using AI

- Titel: **I Built a C++ Micro Voxel Engine using AI (Integrated Graphics, 60+ FPS, 8GB)**
- Kanaal: **saladmander**
- Publicatiedatum: 2026-06-15
- Duur: 293 seconden
- Transcript: uitgeschakeld.
- Metadata-hardware: Intel Core i5-1235U, 8 GB RAM, Intel UHD geïntegreerde GPU.

### Door maker expliciet geclaimd in metadata

- C++/OpenGL-engine, custom ECS en data-oriented design.
- 8×8×8 micro-voxel grid.
- Greedy meshing.
- Drie HLOD-tiers en 24-chunk render distance.
- Hybride cellular automata voor water/lava/zand/gravel.
- AABB-collision.
- BFS-lighting (beschrijving was in de beschikbare metadata verder afgekapt; volledige claim nog live verifiëren).

### Visueel waarneembaar

Contact sheet toont een volledige procedurele voxelgame: terrein/biomes, bomen, third-person character, instelbare renderer/enginepanelen, water/lava, grotten, zwemmen, inventory/material-grid en character animation. In beeld staat onder meer: “10–18 render distance”, shadow on, godlight on, screen recording/software, >60 FPS, Intel Core i5-1235U, 8 GB RAM, Intel UHD Graphics, en dat world/texture/trees/character/animation procedureel gegenereerd zijn.

### Voorbehoud

De video/metadata alleen bewijzen geen benchmarkmethodologie, frametime-stabiliteit, wereldpersistentie, editcorrectheid of schaal buiten de demo. Claims moeten aan broncode/repo en reproduceerbare build worden gekoppeld als die beschikbaar zijn.

## -vqWzDaWUKk — Raytracing Volumetric Clouds using Voxels (Devlog #5)

- Titel: **Raytracing Volumetric Clouds using Voxels (Devlog #5)**
- Kanaal: **MishMash**
- Publicatiedatum: 2026-07-14
- Duur: 199 seconden
- Transcript: vrijwel alleen muziek.

### Visueel/tekstueel waarneembaar

- “Raytracing Voxel Cloud Volumes”.
- “Volumetric horizon scatter at sunset”.
- Weather event simulation en cyclische weather states/events.
- Sneeuw die een hele biome zichtbaar verandert.
- Volledig 3D-wolken geïntegreerd in de wereld.
- Rays door volume met jitter, in beeld genoemd als ongeveer 1/4-resolutie.
- Dag/nacht, sneeuwstorm, regen en belichte nachtelijke wolken.

### Relevantie

Dit is primair onderzoek voor onze toekomstige filmische atmosfeer-/weatherstack, niet bewijs voor een betere terrein-chunkdatastructuur. Waarschijnlijke relevante technieken zijn volumetric ray marching/ray tracing, temporal jitter/reprojection, lage-resolutie volume-rendering, light transmittance en weather-state coupling. Exacte implementatie moet via makerbronnen/repo/papers worden geverifieerd.

## Bewijsbestanden

- Gedownloade video's en `.info.json`: `docs/research/voxel-engine-survey-2026/media/`
- Contact sheets: `docs/research/voxel-engine-survey-2026/contact-sheets/`
- Automatische transcripts: `docs/research/voxel-engine-survey-2026/transcripts/`

Deze notities zijn visuele bronanalyse; speculatieve algoritme-identificatie is geen geaccepteerde claim zonder aanvullende primaire bron.
