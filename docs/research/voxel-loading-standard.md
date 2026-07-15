# Onderzoek: voxel-loading — zichtbare shell vs ondergrond

**Datum:** 2026-07-15
**Auteur:** Hermes
**Context:** Gebruikersvraag tijdens perf/quality-sessie: "worden nu enkel de zichtbare
voxels geladen of ook de ondergrond? Ik heb het idee dat niet alleen de zichtbare kant
van de voxel geladen wordt maar elke voxel."

## Conclusie (kort)
De ondergrond wordt **wel opgeslagen** (in de `Chunk`-data, geheugen) maar **niet
getekend** (de mesh is alleen de zichtbare shell). Dat is de universele voxel-standaard.
Onze code volgt die standaard correct op het **mesh**-vlak, maar verspilt werk op het
**genereer**-vlak: we vullen elke chunk tot de bodem (y=0 lokaal) terwijl alleen de
bovenste ~paar voxels ooit zichtbaar of collision-relevant zijn.

## Hoe de standaard werkt (Minecraft, Veloren, Terasology, eigen engines)
1. **Storage:** een chunk slaat de *volledige* voxel-data op (32³ bij ons). Reden: snelle
   wereldqueries (collision, raycast, edit, mijnen/grotten) mogen geen chunk hoeven
   te regenereren. Dit is goedkoop in geheugen (32³ × 1 byte = 32 KB/chunk).
2. **Mesh:** per chunk wordt een *mesh van de zichtbare shell* gebouwd via face-culling
   (of greedy meshing). Een face tussen twee vaste voxels wordt weggelaten. Resultaat:
   alleen de buitenkant + de surface worden getekend. Ondergrondse intern-faces bestaan
   niet in de GPU-mesh.
3. **Hybrid:** Minecraft splitst soms "render chunks" (shell) van "storage chunks" (vol),
   en genereert diepe lagen lazily. Voor een open-world zonder mijnbouw is volledige
   ondergrond vaak overkill.

## Onze code vandaag
- `crates/voxel-worldgen/src/lib.rs::generate_chunk` — `for ly in 0..SIZE` vult elke
  voxel waar `classify(wy, h, slope, biome) != AIR`. Dus een surface-chunk krijgt
  STONE/DIRT van de surface tot **y=0** (chunk-bodem). Dit is duizenden ondergrondse
  voxels die nooit gerenderd worden.
- `crates/voxel-mesher/src/lib.rs::greedy_mesh` — face-culling via `is_solid(neighbour)`.
  Correct: ondergrondse faces worden niet geëmitteerd. De GPU ziet alleen de shell.
- `crates/voxel-gpu/.../gpu_window.rs` — streamt chunks rond de camera en mesht ze; de
  mesh is de shell. Geen ondergrond-geometry naar de GPU.

**Dus:** de gebruiker ziet terecht dat "elke voxel geladen wordt" (in geheugen), maar
niet dat "elke voxel getekend wordt" (alleen de shell gaat naar de GPU). De mesh-kant
is already optimaal; de genereer-kant is verspilling.

## Aanbevolen verbeteringen (geen breaking changes)
### P1 — Beperk ondergrond-diepte bij generatie (veilig, directe winst) ✅ GEÏMPLEMENTEERD (2026-07-15)
In `generate_chunk`: vul niet tot `y=0` maar tot `max(0, h - BEDROCK_DEPTH)` met
`BEDROCK_DEPTH = 8` voxels (1 m). Alles daaronder blijft AIR. 
- **Winst:** gen-tijd per chunk daalt (minder `chunk.set` voor diepe chunks).
- **Veilig:** de mesh verandert niet (afgesneden diepte zit onder de surface-shell en
  wordt toch niet getekend). Collision gebruikt alleen de top-1 voxel, dus de speler
  blijft staan. Test `chunk_underground_truncated` bevestigt: diepe chunk (0,0,0) is
  leeg, surface-chunk heeft terrain.
- **Caveat:** breekt wereld-determinisme alleen als een diepere query (mijnen/grotten)
  later bijkomt. Voor nu (geen ondergrondse gameplay) is het neutraal.

### P0 — Walkable terrain (2026-07-15) — toegevoegd naar aanleiding van "kan amper rondlopen"
Oorzaak: `fbm01` had octaves met periodes van 32 en 4 voxels (= 4 m en 0,5 m). De
4-voxel octave maakte traptreden van 50 cm elke halve meter → onbegaanbaar ruig.
Fix: alleen de drie laagfrequente octaves behouden `(2048, 0.5), (512, 0.28), (128, 0.14)`.
Steepste locale helling nu ~0.14 m/voxel (zachte heuvels); amplitude 40 m blijft
behouden voor filmische schaal. Test `terrain_is_walkable` eist max slope < 1 m/voxel.

### P2 — "Solid bedrock" i.p.v. AIR onder de grens (optioneel)
Als we later edits/mijnen willen zonder oneindige gen: zet onder `BEDROCK_DEPTH` een
constante STONE-laag i.p.v. AIR, zodat de speler niet door de bodem valt bij edits.
Niet nodig voor de huidige vertical-slice.

### P3 — Mesh is al optimaal
`greedy_mesh` + face-culling is de standaard. Geen actie. (Legacy `naive_mesh`/
`culled_mesh` zijn ongebruikt — kandidaat voor verwijdering bij cleanup.)

## Beslissing nodig (Kanban)
**Titel:** Ondergrond-diepte begrenzen in `generate_chunk` (P1)
**Waarom nu:** directe gen-tijdwinst, geen render-regressie, ondersteunt de 1,90 m
speler + latere performance-doelen.
**Opties:**
- A: Ja, `BEDROCK_DEPTH=8` (1 m) — veilig, meteen.
- B: Ja, maar `BEDROCK_DEPTH=32` (volle chunk) zodat diepe edits later mogelijk zijn.
- C: Nee, ondergrond volledig houden (determinisme voor toekomstige grotten).
**Aanbeveling:** A (1 m grens) — maximale winst, minimale risico voor de huidige slice.
**Gevolg bij uitstel:** gen-tijd blijft ~2× hoger dan nodig; geen blokkade.
