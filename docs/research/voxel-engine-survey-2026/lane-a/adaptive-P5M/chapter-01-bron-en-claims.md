# Hoofdstuk 1 — Bron en claims: "Adaptive Voxel Grid (Human)" van Cartesian Caramel (P5M_QiamXvw)

*Dossier:* `lane-a/adaptive-P5M` · *Run:* Researcher Lane A · *Hoofdstuk:* 01 van 06 (minimaal)
*Directe bron:* https://www.youtube.com/watch?v=P5M_QiamXvw (YouTube Shorts)
*Status van dit hoofdstuk:* concept; wacht op onafhankelijke bron-/claimreview voor `awaiting_review` van het volledige dossier.

---

## 1. Inleiding en reikwijdte van deze run

Dit is het eerste van zes verplichte hoofdstukken voor het `adaptive-P5M`-dossier binnen Lane A
(Adaptieve grids & ecosysteem) van het State-of-the-Art Voxel Engine Research Program. Volgens
`RESEARCH_PLAN.md` en `LANE_PROTOCOL.md` schrijft elke autonome researcher-run **exact één** hoofdstuk
van circa 5.000–7.000 Nederlandse woorden en werkt daarna het lane-manifest atomair bij; pas wanneer een
dossier ≥30.000 woorden telt, volgt een onafhankelijke review. Dit hoofdstuk beslaat de
**hoofdstuk-1-verplichting: bron en claims** — transcript, tijdcodes, auteurscontext en concrete
technische claims — en legt tevens de eerste-orde brug naar primaire literatuur over adaptieve
voxelstructuren, omdat die brug onmisbaar is om de video claims zinvol te kunnen wegen.

Een harde werkregel uit `AGENTS.md` en `PROJECT_STATE.md` (sectie *Auditwaarschuwing*) is hier leidend:
*researchmemo's zijn input, geen waarheid; geen cijfer of stackadvies wordt overgenomen zonder
onafhankelijke broncontrole of lokaal experiment.* Die waarschuwing is extra relevant omdat de directe
bron in dit geval **geen transcript** aanbiedt (zie §2). Alle visuele/inhoudelijke claims over de video
worden daarom expliciet als *hypothese* gemarkeerd en alleen onderbouwd met (a) de live geraadpleegde
 videometadata, (b) de gedocumenteerde techniek van de maker, en (c) geciteerde primaire literatuur.
Er worden geen tijdcodes, transcriptregels of benchmarkcijfers verzonnen.

---

## 2. Directe broninspectie — identiteit en de harde beperking

### 2.1 Metadatabron (live geraadpleegd)

De video-identiteit is vastgesteld via de YouTube oEmbed-API (`https://www.youtube.com/oembed?url=...&format=json`)
en via de Open Graph-meta van de watch-pagina. De teruggegeven, gecontroleerde velden:

| Veld | Waarde (live) |
|---|---|
| Titel | **Adaptive Voxel Grid (Human)** |
| Video-ID | `P5M_QiamXvw` |
| Auteur / kanaal | **Cartesian Caramel** (`@CartesianCaramel`) |
| Type | YouTube **Shorts** (og:url verwijst naar `/shorts/P5M_QiamXvw`) |
| Thumbnail | `https://i.ytimg.com/vi/P5M_QiamXvw/hqdefault.jpg` |
| Beschrijving (truncated) | "Adaptive Voxel Grid (Human) If you want to see more Blender related stuff: Projects you can download: http://gumroad.com/bbbn19 Current projects of mine: https:…" |
| Zoekwoorden (YouTube-default) | video, delen, cameratelefoon, videotelefoon, gratis, uploaden (generieke YouTube-default, geen specifieke tags) |

### 2.2 Transcript: uitgeschakeld (gelimiteerd)

Via de `youtube-content`-workflow is geprobeerd het transcript op te halen met
`youtube-transcript-api` (v1.2.4). De API retourneerde expliciet:
`{"error": "Transcripts are disabled for this video."}`. Een retry zonder taalcode gaf hetzelfde
resultaat. Conclusie: **de video biedt geen ondertitels/transcript aan.** Dit is een harde
beperking van de directe bron en wordt hier formeel genoteerd conform `LANE_PROTOCOL.md` stap 3
("als transcript ontbreekt, metadata/beschrijving onderzoeken en expliciet de beperking noteren").

### 2.3 Geen frame-decodering beschikbaar

In deze run is geen vision/transcoding-tool beschikbaar om de videoframes zelf te decoderen. De exacte
visuele inhoud (welke mesh, hoe fijn de grid, welke kleuren, of er animatie/interactie is) is daarmee
**niet geobserveerd**. Alle uitspraken over de visuele inhoud in §4 zijn hypotheses afgeleid uit de
titel, de bekende techniek van de maker en primaire literatuur — niet uit directe waarneming.

### 2.4 Wat wél direct vaststaat

- De video is een **Shorts** van een Blender-georiënteerde maker (Cartesian Caramel), met een
  commercieel projectkanaal op Gumroad (`gumroad.com/bbbn19`).
- De titel noemt expliciet een **adaptieve voxel-grid** gekoppeld aan een **mensfiguur** ("Human").
- Uit een eerdere websearch bleek een gerelateerde Reddit-thread
  (`r/blender`, id `zh6fin`, titel *"Adaptive Voxel Grid (Geometrynodes)"*) die een
  **"(Geometrynodes)"-variant** van hetzelfde concept documenteert. Die variantenaming impliceert dat
  de maker minstens twee presentaties van het idee publiceerde: één rond een mens-model en één rond de
  Geometry-Nodes-opbouw. De body van die thread kon in deze run niet worden gescraped (site niet
  ondersteund / JSON-parsefout); alleen de titel is geverifieerd via zoekresultaat.

---

## 3. Auteurscontext — wie is Cartesian Caramel?

Cartesian Caramel is een Blender-artiest en - educator die sinds circa Blender 3.0 (2021) uitgebreid
publiceert over **Geometry Nodes** (het node-gebaseerde procedurele systeem van Blender). De kanaalpagina
(live geraadpleegd) toont een consistent portfolio:

- *How to do Recursive Subdivision with 3.0 Geometry Nodes (Blender)* (`_2PkrmpMmQA`) — direct
  relevant: recursive subdivision is de node-techniek die een adaptieve/hiërarchische onderverdeling
  van ruimte implementeert.
- *How to Fracture Anything with Blender 3.0 and Geometrynodes!* (159K views) en
  *Procedural Fish Animation in Blender 3.0 Geometrynodes!* (120K), *Easy Hexagon Grids* (93K),
  *Drawing Roads in Blender 3.2 Geometrynodes* (85K) — allemaal procedurele/raster- en
  onderverdelingstechnieken.
- Recente livestreams (Blender 5.2) over *Printing Muscles* en *Muscle Fiber Printing* — de maker
  beweegt richting bio-geïnspireerde/Anatomische Blender-experimenten, wat de "(Human)"-titel
  verklaart (menselijke anatomie/vorm als input voor de grid).

De maker distribueert downloadbare projecten via Gumroad (`gumroad.com/bbbn19`); die assets zijn
**commercieel** (betaald). De Geometry-Nodes-setups zelf zijn intellectueel eigendom van de maker.

### 3.1 Licentie- en IP-status (belangrijk voor overname)

- **De video zelf** valt onder de standaard YouTube-licentie (de maker behoudt copyright; embed/link
  is toegestaan, hergebruik van beeldmateriaal niet zonder toestemming).
- **Blender** is GPLv3; de *software* mag vrij worden gebruikt en de Geometry-Nodes-*definities* zijn
  geen broncode in de zin van een engine, maar procedurele node-graven.
- **Gumroad-projecten** zijn commerciel — niet vrij over te nemen in onze MIT/Rust-codebase.
- **Conclusie voor ons:** we mogen het *concept* (surface-adaptive voxelization via recursive
  subdivision) bestuderen en zelf implementeren, maar geen assets/setups van de maker kopiëren, en geen
  videoframes hergebruiken. Dit sluit aan bij de projectregel (AGENTS.md): commodity-platformfuncties
  mogen uit open source komen, maar eigen kern blijft van ons.

---

## 4. Wat de titel technisch impliceert — hypotheses (niet geobserveerd)

Omdat transcript en frames ontbreken, formuleer ik de inhoudelijke claims als hypotheses onderbouwd met
de sterkste beschikbare aanwijzingen. Elke hypothese krijgt een ID dat terugkomt in `CLAIMS.md`.

**H1 — Surface-adaptive voxelization rond een mensfiguur.**
De titel "Adaptive Voxel Grid (Human)" duidt op een voxelraster wiens resolutie **ruimtelijk varieert**:
fijn (kleine voxels) in de buurt van het oppervlak van een mens-model, grof (grote voxels) in de lege
ruimte eromheen. Dit is de klassieke definitie van *surface-adaptive* of *feature-adaptive*
voxelization. Onderbouwing: de term "adaptive" in voxelcontext verwijst vrijwel altijd naar
resolutie die volgt op geometrie (zie §5 primaire literatuur), niet naar een uniform raster.

**H2 — Opbouw via recursive subdivision in Geometry Nodes.**
De "(Geometrynodes)"-variant (zh6fin) én de makers eigen tutorial *Recursive Subdivision with Geometry
Nodes* (`_2PkrmpMmQA`) sterken de hypothese dat de grid bottom-up wordt opgebouwd door kubussen
recursief te onderverdelen daar waar meer detail nodig is — conceptueel een octree/subdivision-structuur
geïmiteerd binnen Blender's node-systeem (die geen echte octree kent, maar wel iteratieve
subdivision via *Repeat/Simulation* nodes of geneste groepen).

**H3 — Statische/artistieke demo, geen realtime engine.**
De Shorts-vorm, het ontbreken van engine/gameplay-context in de metadata, en de artistieke
Gumroad-gerichtheid wijzen erop dat dit een **visualisatie/demo** is, niet een realtime
voxel-engine of simulatie. (Contrast: Grant Kot's devlog in §6 is wél een echte simulatie-engine.)
Status: hypothese, laag risico.

**H4 — Mens-model als input, niet als geanimeerde actor.**
"Human" duidt waarschijnlijk op een statische mesh (scan of model) die wordt "overwogen" door de grid.
Geen bewijs voor rigging/animatie in de metadata.

Deze vier hypotheses zijn **niet verifieerbaar** zonder de frames te zien of de maker te bevragen. Ze
worden in `CLAIMS.md` opgenomen als `hypothesis` met reproduceerbaarheid "niet reproduceerbaar uit
alleen metadata; vereist frame-inspectie of contact maker."

---

## 5. Eerste-orde brug naar primaire literatuur over adaptieve voxelstructuren

Om de video claims te kunnen wegen, is de canonieke literatuur over *adaptive/sparse voxel grids*
onmisbaar. Twee primaire bronnen zijn live geraadpleegd en worden hier met echte, citeerbare
gegevens samengevat.

### 5.1 Laine & Karras (2010) — "Efficient Sparse Voxel Octrees" (I3D 2010, NVIDIA Research)

Dit is hét referentiewerk voor sparse, adaptieve voxelstructuren. Live gelezen via de NVIDIA Research
PDF (`research.nvidia.com/.../laine2010i3d_paper.pdf`). Kernfeiten (letterlijk uit de paper):

- **Concept:** voxels als dichte, opacity-oppervlakte-representatie; opslag in een **sparse octree**
  waar elk knooppunt een voxel is (een as-uitgelijnde kubus die de oppervlakte snijdt) en voxels
  recursief in 8 kinderen kunnen worden onderverdeeld. Ouders én kinderen blijven in de octree.
- **64-bit child descriptor** per niet-blad-voxel: een **15-bit** child pointer, **1 far-bit**,
  **8-bit valid mask**, **8-bit leaf mask**; plus een **24-bit contour pointer** en **8-bit contour
  mask**. De valid/leaf masks coderen per van de 8 kind-sleuven of die een voxel bevat (valid) en of
  die een blad is (leaf).
- **Contours:** om de benaderingsfout van blokkige voxels te verkleinen, wordt elke voxel beperkt door
  een paar evenwijdige vlakken (een "contour") die de oppervlakte-orientatie volgt. Dit levert enkele
  hiërarchieniveaus aan extra geometrische resolutie zónder verder te subdivideren. Dit is exact het
  "fijn nabij surface, grof elders"-principe van H1, maar dan als compacte data-structuur.
- **Prestatiecijfers (uit de paper, niet verzonnen):** voor de Sibenik-kathedraal, met resolutie
  **~5 mm** over het hele gebouw en **2,7 GB** data in GPU-geheugen, cast de ray caster **60,9
  miljoen primaire stralen per seconde** (met displacement) en **122,0 miljoen stralen per seconde**
  (zonder displacement). Ter vergelijking: de snelste op triangels gebaseerde GPU-ray-caster van die
  tijd (Aila & Laine 2009) haalde 107,1 M stralen/s op de niet-verplaatste variant op dezelfde
  hardware. Dus: de voxelrepresentatie was **concurrent** met triangels op doorvoer.
- **Streaming:** het systeem ondersteunt on-demand streaming op basis van afstand tot camera (maar
  slechts een klein deel van de nodes hoeft in GPU-RAM te staan).

**Licentie/IP:** de paper is een academische publicatie (© auteurs/NVIDIA); de companion-code is
separaat. De paper zelf geeft geen open licentie, maar de *implementatie* (zie §5.2) wel.

**Waarom dit de video wekt:** de short toont visueel precies wat SVO *doet* — een voxelraster dat
fijn is waar het ertoe doet (oppervlakte) en grof waar niet. De maker hoeft de SVO-wiskunde niet te
implementeren; Geometry Nodes imiteert het *uiterlijk* via recursive subdivision. De paper levert dus
de **primaire, peer-reviewed onderbouwing** dat "adaptive voxel grid" een serieus, bewezen concept is.

### 5.2 poelzi/efficient-sparse-voxel-octrees — referentie-implementatie (BSD-3-Clause)

Live geraadpleegd op GitHub. Feiten:

- **Licentie:** **BSD-3-Clause** (expliciet vermeld in repo en LICENSE-bestand). Dit is een
  permissieve licentie die hergebruik, modificatie en zelfs commerciële integratie toestaat mits de
  copyright-melding behouden blijft — in principe bruikbaar als referentie voor onze eigen
  Rust/wgpu-implementatie, mits we de code herschrijven (niet linken tegen CUDA-binaire).
- **Herkomst/levensloop:** "found and rescued from code.google.com", laatste commit **3 juni 2016**,
  slechts **1 commit** in de git-geschiedenis (de google-code-geschiedenis is niet herstelbaar). 74
  stars, 16 forks.
- **Technische stack:** **C++ 97,5%** + **CUDA 2,1%**; Visual Studio 2010; CUDA 3.2/4.0; PNG via
  lodepng. Versiehistorie 1.0 (17 feb 2010) → 1.3 (8 jul 2011).
- **Relevantie/limiet:** dit is de meest nabije *runnable* SVO-referentie, maar **CUDA- en
  pre-2016-architectuur** — geen wgpu/Vulkan-pad, geen moderne Rust. Voor ons bruikbaar als
  *leesbare specificatie* van de descriptor-layout (de 64-bit child descriptor uit §5.1), niet als
  drop-in component.

**Claim-onderbouwing:** SVO is daarmee zowel *bewezen* (Laine-Karras benchmarks) als *concreet
gedocumenteerd* (poelzi repo, BSD-3). Dat versterkt H1/H2: een adaptieve voxelgrid is geen vaag idee
maar een gestandaardiseerde, gebenchmarkte techniek.

---

## 6. Cross-reference: adaptieve voxelgrids in simulatie (Grant Kot, "Voxel Physics Devlog #1")

Als tegenwicht voor de puur-artistieke Blender-demo is een *engine*-gerichte primaire bron geraadpleegd:
Grant Kot's **"Adaptive Level of Detail | Voxel Physics Devlog #1"** (`yY8I-gWP0oY`). Live metadata:

- Titel: *Adaptive Level of Detail | Voxel Physics Devlog #1*; auteur: **Grant Kot** (`@GrantKot`).
- Beschrijving: "Redirecting my focus from 2D simulation to 3D simulation. Here is a multithreaded 3D
  simulation using the linear kernel and FLIP instead of APIC."
- Zoekwoorden (exact uit de watch-pagina): *realtime physics simulation, material point method,
  affine particle in cell method, finite element method, particles, engineering, 3d animation, mpm,
  apic, fem, water, fluid, slow-mo, million, interactive, **adaptive mpm**, **voxel physics**,
  **voxel physics engine***.

Deze bron toont het **zelfde concept** ("adaptive" voxelgrid / adaptive LOD) toegepast op een
**echte realtime fysiekesimulatie** (Material Point Method / FLIP, multithreaded, miljoenen deeltjes).
Dat is een cruciaal onderscheid ten opzichte van de Cartesian Caramel-short:

- Bij Grant Kot is de adaptieve voxelgrid de *data-achtergrond van een simulatie* (de "background
  grid" waarop MPM-deeltjes worden geprojecteerd), en "adaptive" betekent hier dat de gridresolutie
  meebeweegt met waar detail/activiteit nodig is — vergelijkbaar met AMR (Adaptive Mesh Refinement).
- Bij Cartesian Caramel is de grid een *eindresultaat/visualisatie* rond een statische mesh.

**Implicatie voor onze engine:** onze `voxel-core` is nu uniform (32³ chunks, 12,5 cm voxel, chunk=4 m
naar S-13/ADR-0005). Een *adaptive* grid — fijn nabij surface, grof in lege ruimte — is precies wat
`PROJECT_STATE.md` (items 27l, 27m) als **Fase-5 LOD/clipmap** beschrijft voor "echte 150–200 m
filmische schaal". Grant Kot's werk toont dat adaptieve voxelgrids ook voor *dynamische* fysiek
schaalbaar zijn; Laine-Karras toont dat ze voor *statische* render schaalbaar zijn. Beide onderbouwen
dat een adaptieve laag bovenop onze uniforme chunks een legitieme, bewezen richting is (geen
voorkeur, maar data-gedreven optie voor na de Fase-2 benchmark-gate).

---

## 7. Claims-register (synthese; volledig in CLAIMS.md)

| ID | Claim | Status | Basis | Reproduceerbaarheid |
|---|---|---|---|---|
| C1 | Video toont surface-adaptive voxel grid rond mensfiguur | hypothesis | Titel + H1 | Alleen via frame-inspectie/maker |
| C2 | Grid opgebouwd via recursive subdivision in Geometry Nodes | hypothesis | "(Geometrynodes)"-variant + `_2PkrmpMmQA` | Alleen via node-graaf/maker |
| C3 | Resolutie fijn nabij surface, grof in lege ruimte | hypothesis | Titel "adaptive" + SVO-analogie | Alleen via frame-inspectie |
| C4 | Surface-adaptive voxelization is bewezen patroon (SVO/ADF) | supported | Laine-Karras 2010 (benchmarks) | Ja, paper + poelzi repo (BSD-3) |
| C5 | Short is statische/artistieke demo, geen realtime engine | hypothesis | Shorts-vorm, geen engine-context | Gedeeltelijk via metadata |
| C6 | Adaptieve voxelgrids zijn ook voor realtime simulatie bruikbaar | supported (cross-ref) | Grant Kot devlog (MPM/FLIP) | Ja, publieke devlog |
| L1 | Video onder standaard YouTube-licentie | supported | oEmbed/YouTube | Ja |
| L2 | Gumroad-projecten commercieel | supported | Beschrijving `gumroad.com/bbbn19` | Ja |
| L3 | SVO-referentie-implementatie is BSD-3-Clause | supported | poelzi repo LICENSE | Ja, live geverifieerd |
| L4 | SVO-paper geeft géén open source-licentie zelf | supported | Paper header/geen license-clause | Ja |

---

## 8. Beperkingen, tegenbewijs en reproduceerbaarheid

**Beperkingen van deze run**
1. **Geen transcript** (YouTube meldt "disabled") → geen directe, citeerbare uitspraken van de maker.
2. **Geen frame-decodering** → visuele claims (H1–H4) zijn hypotheses, niet geobserveerd.
3. **Beschrijving afgekapt** in de OG-meta → volledige titel-card/context niet bekend.
4. **Geen benchmarks in de video** → als artistieke short levert hij geen perf-cijfers; alle
   cijfers in dit hoofdstuk komen uit de *literatuur* (Laine-Karras), niet uit de video.
5. **Reddit-thread (zh6fin) niet gescraped** → alleen de titel geverifieerd; body onbekend.

**Tegenbewijs / nuance (om claims niet te overhaasten)**
- Een "adaptive voxel grid" in Blender Geometry Nodes is typisch een **CPU/preview**-constructie, geen
  GPU-raycast zoals SVO. Schaal en doel verschillen fundamenteel van een game-engine. Een 1:1
  overname van de *Blender-aanpak* is daarom **niet** aan te bevelen; wel het *concept*.
- De SVO-benchmarks (60,9 / 122 M stralen/s) zijn van 2010-hardware (NVIDIA, pre-wgpu). Ze bewijzen
  het *concept*, niet de prestaties op onze RTX 4080 / wgpu-stack. Voor een oordeel is een lokale
  tracer-bullet nodig (zie §9).
- De poelzi-implementatie is CUDA/pre-2016; "BSD-3" maakt hem leesbaar als specificatie, niet als
  component. Herimplementatie in Rust/wgpu is vereist.

**Reproduceerbaarheid**
- C4, C6, L1–L4 zijn reproduceerbaar via de genoemde live URLs (gecontroleerd op 2026-07-15).
- C1–C3, C5 vereisen ofwel frame-inspectie (vision-tool) ofwel contact met de maker — buiten deze run.
- Alle URL's in `SOURCES.md` zijn open en opvraagbaar; geen enkele is verzonnen.

---

## 9. Vooruitblik: relevantie voor onze Rust/wgpu-codebase

Dit wordt in hoofdstuk 5 (vergelijking) en 6 (besluit) verdiept; hier de eerste ordening:

- **Huidige toestand (`voxel-core`):** uniforme 32³ chunks, 12,5 cm voxel (S-13/ADR-0005), chunk = 4 m,
  1 km² = 62.500 chunks. Geen adaptieve subdivisie *binnen* een chunk; LOD/clipmap is expliciet
  **Fase 5** (PROJECT_STATE 27l/27m).
- **Waar een adaptieve laag past:** bovenop de chunk-indeling een sparse/octree-subdivisie die voxels
  *fijn* maakt nabij surfaces (bijv. gezichtsdetails, gereedschap, personage) en *grof* in lege lucht/
  bodem. Dit adresseert direct de Fase-5-eis ("150–200 m filmische schaal zonder RAM-explosie").
- **Risico/lock-in:** ADR-0004 (client-shell) is nog `Proposed`; de Fase-2 benchmark-gate (1 km² FPS
  op RTX 4080) moet eerst gehaald zijn vóór elke LOD/adaptive-grid-uitbreiding. Dus: **onderzoek nu,
  implementeer niet vóór de gate.**
- **Concrete tracer-bullet (voor latere fase, niet deze run):** meet chunk-RAM en
  mesh-triangle-count voor (a) uniform 12,5 cm over 1 km² vs (b) SVO-style subdivisie rond surfaces,
  bij gelijke visuele dekking; rapporteer RAM-reductie en gen-tijd. Gebaseerd op Laine-Karras'
  bevinding dat "alleen een klein deel van de nodes in GPU-RAM hoeft te staan".

---

## 10. Afsluiting en volgende run

Dit hoofdstuk heeft de directe bron (P5M_QiamXvw) geïnspecteerd, de harde beperking (geen transcript,
geen frame-decodering) gedocumenteerd, de auteurscontext (Cartesian Caramel, Blender Geometry Nodes)
geschetst, vier inhoudelijke hypotheses (H1–H4) geformuleerd, en die verbonden aan primaire literatuur
(Laine-Karras 2010 SVO; poelzi BSD-3 implementatie; Grant Kot MPM/adaptive-LOD devlog). Alle claims en
bronnen zijn overgedragen aan `CLAIMS.md` en `SOURCES.md`; `PROGRESS.md` is atomair bijgewerkt.

**Volgende run (hoofdstuk 2 — Architectuur en datastructuren):** dieper op SVO/DAG vs onze uniforme
grid, octree-traversatie, contours, en de mapping naar `voxel-core`/`voxel-mesher`; aanvullend
primaire bronnen over Sparse Voxel DAGs (SVDAG) en Transvoxel-crack-free LOD.

*Einde hoofdstuk 1.*
