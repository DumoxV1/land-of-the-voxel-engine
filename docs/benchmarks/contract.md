# Benchmarkcontract

## Doel
De north star vereist extreme rijkdom zonder oncontroleerbare frametime, geheugen-, edit- of netwerkproblemen. Iedere technische keuze moet daarom vergelijkbaar en reproduceerbaar zijn.

## Vaste metingen
- p50/p95/p99 CPU- en GPU-frametime
- resident RAM en VRAM
- bytes per voxel/brick/chunk
- mesh-buildtijd en triangles/vertices
- edit-to-visible latency
- edit-to-collision latency
- chunk generation/load/upload latency
- server ticktijd en queue depth
- bytes per client per seconde
- save-, replay- en restoreduur
- deterministische world/chunk hash

## Scenario’s
1. Lege/uniforme wereld
2. Natuurlijk terrein en grot
3. Checkerboard/worst-case oppervlak
4. Dichte gebouwde stad
5. Herhaalde destructie op chunkgrenzen
6. Snelle camerabeweging door streaminggebied
7. Meerdere clients rond dezelfde hotspot
8. Crash/restart met pending edits

## Gates
Een kandidaat wint niet op gemiddelde FPS alleen. p99 frametime, correctness, geheugen, editlatency, physics, netwerkbaarheid en ontwikkelcomplexiteit tellen mee. Exacte drempels worden na hardwarebaseline als ADR vastgelegd.
