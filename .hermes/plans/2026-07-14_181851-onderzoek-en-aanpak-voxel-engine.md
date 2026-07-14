# Land of the Voxel Engine — onderzoeks- en aanpakplan

> **Voor Hermes:** voer dit plan later gefaseerd uit met kleine, verifieerbare taken. Gebruik gratis modellen voor routinewerk en reserveer betaalde modellen voor architectuur, debugging en reviews.

**Doel:** In drie maanden een technisch bewezen, speelbare vertical slice bouwen die de moeilijkste fundamenten van een custom micro-voxel multiplayer-RPG valideert, zonder te doen alsof een volwaardige MMO in die periode haalbaar is.

**North star:** een filmische, rijke en dynamische openwereld-RPG — qua ambitie “de GTA VI / Crimson Desert onder micro-voxel-engines”. Dit is een kwaliteitslat voor werelddichtheid, interactie, animatie, systeemdiepte en presentatie; geen opdracht om beschermde assets of concrete ontwerpen te kopiëren. Maximale dichtheid of extremiteit is alleen waardevol wanneer speelbaarheid, determinisme, editlatency, netwerkbaarheid en frametime aantoonbaar goed blijven.

**Levend-plancontract:** dit document is de canonieke bron voor scope en architectuur. Iedere agent leest het vóór werk, en past het alleen aan op basis van geverifieerde bronnen, benchmarks of expliciete productbesluiten. Na iedere derde voltooide uitvoeringsstap wordt één stap teruggegaan: het voorgaande artifact wordt opnieuw gecontroleerd, plan-alignment wordt beoordeeld en eventuele drift wordt gecorrigeerd voordat de volgende stap start.

**Architectuur:** Een eigen, headless-draaibare voxel/world-kern met deterministische dataformaten en een authoritative server. Voor de eerste slice gebruiken we bestaande gratis infrastructuur waar dat geen onderscheidend voordeel oplevert; de voxeldata, chunkstreaming, meshing, wereldwijzigingen, replicatie en persistence blijven eigen code. Steam wordt aanvankelijk als distributie-, identity-, lobby- en netwerklaag gezien, niet als gratis serverhosting.

**Voorgestelde stack voor de eerste spike:** Rust-workspace voor `voxel-core`, server en benchmarks; twee concurrerende clientspikes: (A) Godot 4 + GDExtension voor de snelste productiviteit, (B) Bevy/wgpu voor maximale engine-eigendom. Pas na objectieve benchmarks kiezen we één clientpad.

---

## 1. Samenvatting en haalbaarheidsgrens

De eindvisie combineert minimaal vijf afzonderlijk grote projecten:

1. een voxel-engine;
2. een 3D-renderer en tooling;
3. een openwereld-streaming- en persistentiesysteem;
4. een realtime multiplayer-backend;
5. een MMORPG met content, economie, security en live operations.

Dat is niet realistisch als één drie-maandenproject. Wat **wel** realistisch en waardevol is: een verticale technische slice waarin 2–8 spelers in één persistente, aanpasbare wereld kunnen bewegen, voxels kunnen plaatsen/verwijderen en na herstart dezelfde wijzigingen terugzien. Daarmee bewijzen we de moeilijkste architectuurkeuzes zonder maanden te verspillen aan menu’s, lore of premature MMO-infrastructuur.

### Definition of success na circa 12 weken

- Desktopclient op Windows met first/third-person camera.
- Procedurele wereld rond de speler, asynchroon geladen in chunks.
- Plaatsen en verwijderen van micro-voxels met beperkte materialen.
- Stabiele frametime binnen een afgesproken testscenario.
- Authoritative dedicated server zonder grafische afhankelijkheid.
- 2–8 lokale/internetclients, met interest management per chunk.
- Wereldwijzigingen persistent na serverrestart.
- Geautomatiseerde unit-, property-, integratie- en soaktests.
- Reproduceerbare benchmarkrapporten voor geheugen, meshing, streaming en netwerk.
- Gedocumenteerde beslissing of Godot-shell of Bevy/wgpu verdergaat.

### Uitdrukkelijk niet in de eerste 12 weken

- duizenden gelijktijdige spelers;
- één naadloze wereld over meerdere serverprocessen;
- volledig RPG-, crafting-, quest-, economie- of combat-systeem;
- volledige Steam-release;
- anti-cheat op productieniveau;
- geavanceerde destructie, vloeistoffen, GI of pathfinding over de hele wereld;
- een eigen audio-, UI-, animatie-, assetimport- en scriptingstack.

---

## 2. Belangrijkste ontwerpbeslissingen om samen te nemen

### 2.1 Wat betekent “micro-voxel” precies?

Dit moet vóór implementatie meetbaar worden gemaakt. Voorstel voor de eerste slice:

- 1 wereldmeter is een logische macrocel;
- een macrocel kan lokaal worden onderverdeeld in bijvoorbeeld `8×8×8` micro-voxels;
- alleen bewerkte of detailrijke macrocellen krijgen microdata;
- onbewerkte ruimte blijft procedureel/compact opgeslagen;
- materiaal-ID is gepaletteerd, niet per voxel een zwaar object.

Hiermee vermijden we een uniforme wereld waarin elke kubieke meter altijd 512 afzonderlijke voxels kost. Alternatieven die in een spike vergeleken moeten worden:

1. **Uniform grid:** eenvoudig, voorspelbaar, maar snel te groot.
2. **Hiërarchische macro/micro-chunks:** aanbevolen startpunt; detail alleen waar nodig.
3. **Sparse voxel octree/DAG:** sterke compressie voor statische data, maar complexe updates en GPU-traversal.
4. **Density field + surface meshing:** organisch terrein, maar materiaalbewerking, collisions en determinisme zijn ingewikkelder.

### 2.2 Wat betekent “custom engine”?

Aanbevolen definitie: we bezitten de systemen die de innovatie bepalen—voxelopslag, terrain, meshing, LOD, streaming, networking en persistence—maar hergebruiken commodity-onderdelen zoals windowing, input, audio, UI en platform-API’s.

Twee paden worden kort gespiket:

| Pad | Pluspunten | Minpunten | Gebruik |
|---|---|---|---|
| Godot 4 + native extension | Editor, input, UI, audio, physics en headless export direct beschikbaar | Minder volledige engine-eigendom; integratiegrenzen | Snelste kans op een speelbare slice |
| Rust + Bevy/wgpu | Veel controle, één taal voor core/server/client, portable renderer | Meer infrastructuur, tooling en editorwerk | Alleen kiezen als benchmark/controlewinst dit rechtvaardigt |

De core moet renderer-onafhankelijk blijven, zodat de spike geen weggegooid werk wordt.

### 2.3 Blocky of smooth?

Voor fase 1: **blocky/palette micro-voxels met culled/greedy meshing**. Dit is eenvoudiger te testen en synchroniseren. Smooth density terrain, Marching Cubes en Transvoxel worden pas toegevoegd nadat data, streaming en netwerk stabiel zijn. Transvoxel lost specifiek scheuren tussen meshes met verschillende resoluties op, maar brengt nu onnodige complexiteit mee.

---

## 3. Voorgestelde technische architectuur

### 3.1 Repository-indeling

```text
Land of the Voxel Engine/
├── AGENTS.md                    # compacte regels voor alle coding-agents
├── README.md                    # visie, quick start, actuele status
├── Cargo.toml                   # Rust workspace
├── docs/
│   ├── vision.md
│   ├── glossary.md
│   ├── architecture/
│   │   ├── overview.md
│   │   ├── adr/                 # één Architecture Decision Record per keuze
│   │   └── diagrams/
│   ├── benchmarks/
│   └── protocols/
├── crates/
│   ├── voxel-core/              # coords, chunks, palettes, edits, serialization
│   ├── voxel-mesher/            # culling/greedy meshing; later LOD
│   ├── worldgen/                # seeded, deterministische generatie
│   ├── world-store/             # snapshots + append-only editlog
│   ├── protocol/                # versieerbare netwerkberichten
│   ├── game-sim/                # authoritative fixed-tick simulatie
│   └── dedicated-server/        # headless serverbinary
├── clients/
│   ├── godot-spike/             # dunne Godot-shell + GDExtension
│   └── bevy-spike/              # dunne Bevy/wgpu-shell
├── tools/
│   ├── world-inspector/
│   └── benchmark-runner/
├── tests/
│   ├── fixtures/
│   └── soak/
└── .github/workflows/
```

Definitieve paden worden pas na de stackspike vastgezet.

### 3.2 Voxeldata en coördinaten

- Gebruik integer wereldcoördinaten; floats uitsluitend voor lokale rendering.
- Maak negatieve coördinaten expliciet correct met euclidische deling.
- Scheid `WorldVoxel`, `ChunkCoord` en `LocalVoxel` als verschillende types.
- Start met een vaste chunkmaat die via benchmarks gekozen wordt, bijvoorbeeld 32³ macrocellen of een kleinere micro-chunk.
- Gebruik een palette per chunk en bitpacking voor materiaalindices.
- Ondersteun minstens drie chunktoestanden: uniform, palette-packed en dense.
- Procedurele basiswereld + alleen afwijkingen opslaan; geen volledige onbewerkte wereld naar disk schrijven.
- Ieder edit-event bevat wereldpositie, oude/nieuwe waarde, actor, server-tick en monotone revisie.

### 3.3 Meshing en rendering

Eerste implementatiereeks:

1. referentiemesher: één kubus per voxel;
2. face culling van verborgen vlakken;
3. greedy meshing per materiaal/normaalrichting;
4. asynchrone remesh-jobs met generatie/revisie-ID’s;
5. frustum culling en afstandsbudget;
6. pas later multiresolution LOD en eventueel Transvoxel.

Belangrijk: het netwerk verstuurt voxeldata/edits, **nooit render-meshes**. Clients bouwen meshes lokaal. De server heeft geen GPU nodig.

### 3.4 Physics

- In de slice alleen capsule-vs-voxelwereld en eenvoudige AABB’s.
- Collisionmesh wordt per vuile chunk gegenereerd of er wordt direct tegen het voxelveld gecast.
- Physics gebruikt dezelfde authoritative voxelrevisie als gameplay.
- Geen dynamische rigid-body destructie in fase 1.

### 3.5 Server en networking

- Server-authoritative fixed tick, aanvankelijk 20–30 Hz.
- Client verstuurt intenties/input, niet zijn “ware” positie of voxelresultaat.
- Client-side prediction en reconciliation alleen voor spelerbeweging.
- Chunkgebaseerd interest management: server verstuurt alleen relevante entiteiten, snapshots en edits.
- Betrouwbare berichten voor login, inventaris, chunkbaseline en edits; onbetrouwbaar/sequenced voor frequente transforms.
- Per chunk: baseline/revisie + oplopende deltas; bij gat of mismatch opnieuw baseline aanvragen.
- Alle invoer valideren: bereik, rate, permissions, materiaal, revisie en maximaal pakketformaat.
- Begin met één serverproces en één zone. Cross-zone handoff en sharding zijn latere architectuurspikes, geen MVP-vereiste.

### 3.6 Persistence

Aanbevolen start:

- SQLite voor accounts/testspelers, metadata en indexen;
- append-only editlog per regio/chunk;
- periodieke compacte snapshots;
- write-ahead voordat een edit als definitief wordt bevestigd;
- deterministische replaytest die snapshot + log vergelijkt met live state;
- expliciete schema- en protocolversies vanaf dag één.

### 3.7 Steam

“Steam hosting” moet worden opgesplitst:

- Steam kan distributie, identity/auth, lobbies, server discovery, Steam Networking Sockets en mogelijk Steam Datagram Relay bieden.
- Steam levert niet automatisch gratis CPU/RAM waarop onze dedicated MMORPG-server draait.
- Dedicated servers moeten door ons, spelers/community of een hostingprovider worden uitgevoerd.
- Steam Datagram Relay routeert verkeer en kan IP-adressen beschermen; hosted dedicated-serverintegratie vereist extra coordinator/ticketwerk.
- Publicatie op Steam vereist bovendien de Steam Direct-procedure en momenteel een vergoeding van USD 100 per app; dit valt buiten het genoemde LLM-budget van €40.
- Daarom: eerst LAN/direct-IP/localhost; daarna Steam lobbies/identity; pas daarna SDR en distributie.

---

## 4. Drie-maandenroadmap

De planning gebruikt gates: een fase gaat alleen door als meetbare criteria slagen.

### Fase 0 — Productdefinitie en meetlat (week 1)

**Deliverables**

- `docs/vision.md`: één pagina met fantasy, spelerbelofte en niet-doelen.
- `docs/glossary.md`: exacte definities van micro-voxel, chunk, regio, shard, MMO.
- doelhardware, FPS-, RAM-, opslag-, latency- en spelerdoelen;
- eerste ADR’s voor taal, licensing, voxelvorm en servermodel;
- git-repository, CI, formatter/linter, test- en benchmarkharnas.

**Gate**

- Alle open vragen uit sectie 8 hebben voorlopige antwoorden.
- We kunnen in één alinea uitleggen wat na 12 weken speelbaar is.

### Fase 1 — Data- en meshing-spikes (week 2–3)

**Deliverables**

- coördinaten- en chunkbibliotheek met propertytests;
- uniforme/palette/dense opslagbenchmark;
- naive, culled en greedy mesher met gouden fixtures;
- deterministische worldgen met seed;
- benchmarkrapport voor meerdere chunkmaten en dichtheden.

**Gate**

- round-trip serialization is byte-stabiel of canoniek;
- random edits veroorzaken geen scheuren aan chunkgrenzen;
- gekozen opslag blijft binnen afgesproken RAM-budget;
- remeshing blokkeert de hoofdthread niet.

### Fase 2 — Twee clientshellspikes (week 4)

**Deliverables**

- dezelfde corewereld zichtbaar in Godot en Bevy/wgpu;
- camera, input, materiaalpalette, chunk upload en simpele collision;
- vergelijkingsmatrix: ontwikkeltijd, buildtijd, FPS, frametime, debugging, headless/serverintegratie.

**Gate**

- één clientpad wordt gekozen en het andere gearchiveerd;
- beslissing wordt vastgelegd als ADR, niet op gevoel.

### Fase 3 — Lokale speelbare wereld (week 5–6)

**Deliverables**

- asynchrone chunkgeneration/meshing/uploadpipeline;
- plaats/verwijder-tool met revisies;
- eenvoudige spelercontroller;
- save/load van seed + edits;
- debugoverlay voor chunks, jobqueues, triangles en geheugen.

**Gate**

- 30 minuten rondlopen en bewerken zonder crash of groeiend geheugen;
- save/reload behoudt alle edits;
- benchmarkscenario voldoet aan voorlopige FPS/frametime-doelen.

### Fase 4 — Authoritative multiplayer (week 7–9)

**Deliverables**

- headless dedicated server;
- protocolhandshake met versiecontrole;
- 2–8 clients, beweging, voxel-edits en chunkinterest;
- prediction/reconciliation voor spelerbeweging;
- delta’s + baseline-resync voor chunks;
- simulatie van latency, jitter, packet loss en reconnects.

**Gate**

- server accepteert geen ongeldige/out-of-range voxel-edit;
- clients convergeren na packet loss en reconnect;
- soaktest met bots blijft stabiel;
- server kan zonder GPU draaien.

### Fase 5 — Persistence en minimale RPG-lus (week 10–11)

**Deliverables**

- server-side speleridentiteit;
- persistent inventory met enkele materialen;
- mine → collect → place/craft als minimale lus;
- snapshots, editlog, compaction en recoverytest;
- simpele permissions/rate limiting.

**Gate**

- hard-kill/restart verliest hoogstens expliciet gedefinieerde, zeer korte onbevestigde data;
- replay en live world hash zijn gelijk;
- duplicatiepogingen via dubbele berichten/reconnect falen.

### Fase 6 — Packaging, Steam-spike en evaluatie (week 12)

**Deliverables**

- reproduceerbare Windows client- en headless-serverbuild;
- LAN/direct-IP handleiding;
- technische Steamworks-spike (geen publicatie vereist);
- performance- en kostenrapport;
- go/no-go-roadmap voor 6–12 maanden.

**Gate**

- een andere tester kan client en server starten met alleen de README;
- alle acceptance tests en soaktests draaien reproduceerbaar;
- volgende investering is onderbouwd met metingen.

---

## 5. Hermes/OpenRouter-workflow binnen €40

### 5.1 Budgetverdeling

Rekenkader voor 13 weken:

- totaal: €40;
- gemiddelde bovengrens: circa €3,08 per week;
- reserve: €10 voor uitzonderlijke debugging/architectuur aan het einde;
- actief betaald budget: €30, circa €2,31 per week;
- gratis modellen zijn standaard; betaald gebruik is een expliciete escalatie.

Maak een aparte OpenRouter-key voor dit project met een harde creditlimiet. Zet geen sleutel in de repository. Controleer usage periodiek via OpenRouter’s key-endpoint of dashboard. Een gratis variant eindigt meestal op `:free`; de `openrouter/free` router kiest automatisch een geschikt gratis model, maar is niet deterministisch.

OpenRouter documenteert momenteel voor free variants 20 requests/minuut en, wanneer historisch minstens USD 10 credits zijn gekocht, 1000 requests/dag; zonder die aankoop is dit 50/dag. Beschikbaarheid verandert, dus pin geen projectplan aan één gratis modelnaam.

### 5.2 Rollen en modelrouting

| Werksoort | Standaard | Escalatie |
|---|---|---|
| Bestanden zoeken/lezen, tests draaien, formatteren | tools/scripts, geen LLM waar mogelijk | geen |
| Kleine implementatie, docs, testgeneratie | capabel gratis tool-calling/codingmodel | goedkope betaalde coder na 2 mislukte pogingen |
| Onderzoekssamenvatting | gratis model + primaire bronnen | betaald alleen bij conflicterende bronnen |
| Architectuur/ADR | twee gratis onafhankelijke voorstellen | één sterke betaalde reviewer |
| Bugfix | gratis debugger met minimale context | betaald na reproduceerbare testcase en root-cause dossier |
| Security/protocol/data-migratie review | gratis eerste pass | sterke betaalde finale review verplicht vóór release |

De actuele model-API liet tijdens dit onderzoek meerdere gratis tool-callingopties zien, waaronder `openrouter/free`, `qwen/qwen3-coder:free`, `openai/gpt-oss-20b:free`, `nvidia/nemotron-3-super-120b-a12b:free`, `qwen/qwen3-next-80b-a3b-instruct:free` en het momenteel ingestelde `tencent/hy3:free`. Deze lijst is vluchtig; laat een script capabilities en prijs opnieuw ophalen.

### 5.3 Hermes-organisatie

1. **Hoofdsessie = product owner/architect.** Houdt scope, ADR’s, risico’s en acceptatiecriteria bij.
2. **`delegate_task` = kort onderzoek/review.** Maximaal 2–3 parallelle, onafhankelijke taken; geen agents laten dupliceren.
3. **Kanban = duurzame implementatietaken.** Pas activeren zodra repository en profielen bestaan; taken overleven sessies en hebben audittrail.
4. **Profielen op rol:** bijvoorbeeld `architect`, `voxel`, `network`, `reviewer`; elk met minimaal benodigde skills/tools en liefst gratis model.
5. **Worktrees voor parallel codewerk.** Geen twee agents tegelijk dezelfde branch/bestanden laten wijzigen.
6. **Cron alleen voor goedkope deterministische checks.** Bijvoorbeeld status/usage of nightly tests via script-only `no_agent`; geen dagelijkse LLM-samenvattingen tenzij nodig.
7. **Skills voor herhaalbare projectprocedures.** Pas na bewezen workflows; niet elk taakresultaat als memory opslaan.
8. **`AGENTS.md` compact houden.** Grote kennis in `docs/`; agents krijgen alleen relevante files en tests.

### 5.4 Kostenbesparende regels

- Eerst een failing test/reproducer, dan pas een model om code vragen.
- Geef agents een exact bestand, interface en acceptatiecriterium; niet de hele repo.
- Gebruik search/read/terminal voor feiten en builds; geen LLM voor rekenen of logfiltering.
- Eén implementer + één reviewer, niet drie implementers voor routinewerk.
- Maximaal twee gratis pogingen; daarna diagnose verscherpen, niet eindeloos regenereren.
- Betaalde call alleen met een “escalatiepakket”: probleem, minimale reproducer, logs, relevante code/diff, al geprobeerde oplossingen en gewenste output.
- Nieuwe sessie per afgebakende taak om contextkosten te beperken; beslissingen duurzaam in ADR’s.
- Pin stabiele modelkeuzes gedurende één taak om prompt-cacheverlies en gedragsvariatie te beperken.
- Run lokaal `cargo fmt`, `cargo clippy`, tests, benchmarks en protocol fuzz/propertytests.
- Door mensen goed te keuren: scopewijzigingen, externe publicatie, uitgaven, secrets, deployments en destructieve gitacties.

### 5.5 Wekelijkse kwaliteitsgate

Iedere week eindigt met:

1. tests groen;
2. benchmarkdelta tegenover vorige baseline;
3. één onafhankelijke code-review;
4. ADR voor iedere blijvende architectuurwijziging;
5. budget/usage check;
6. demo of reproduceerbaar artifact;
7. stop/go-besluit voor de volgende week.

---

## 6. Test- en validatiestrategie

### Unit/propertytests

- wereld ↔ chunk/lokale coördinaten, vooral negatieve grenswaarden;
- palette-overgangen en bitpacking;
- serialization round trips en corrupte input;
- edit-idempotentie en revisievolgorde;
- meshing op lege, volle, checkerboard- en grensoverschrijdende chunks.

### Integratietests

- client/server protocolversies;
- baseline + ontbrekende delta + resync;
- twee gelijktijdige edits op dezelfde voxel;
- reconnect tijdens chunktransfer;
- snapshot + editlog recovery.

### Performance

- vaste seeds en camerapaden;
- p50/p95/p99 frametime, niet alleen gemiddelde FPS;
- meshingtijd per chunk en per dirty volume;
- bytes per voxel/chunk, triangles, uploads/frame;
- server tickduur, bandbreedte per client en dirty-chunkqueue;
- soaktest op geheugen- en handleleaks.

### Security

- fuzz/parsinglimieten voor netwerkberichten en savefiles;
- server valideert alle acties;
- rate limits en bounded queues;
- nooit clientgestuurde bestandspaden of arbitrary code;
- dependencies en licenties periodiek controleren.

---

## 7. Grootste risico’s en mitigaties

| Risico | Impact | Mitigatie |
|---|---:|---|
| Scope-explosie door “MMO + RPG + engine” | Kritiek | Vertical slice en expliciete niet-doelen; gates per fase |
| Uniforme micro-voxels verbruiken extreem veel RAM/disk/netwerk | Kritiek | Hiërarchische sparse data; procedurele basis + edits; benchmarks vóór renderer |
| Premature SVO/raytracing/Transvoxel-complexiteit | Hoog | Begin met blocky culled/greedy meshing; optimaliseer vanuit profilerdata |
| Custom alles bouwen | Kritiek | Eigen differentiërende kern, hergebruik commodity libraries/platforms |
| Free-model instabiliteit/kwaliteit | Hoog | Capability discovery, tests, fallback, betaalde reserve voor gates |
| AI produceert plausibele maar foute concurrency/networkcode | Kritiek | TDD, propertytests, fuzzing, sanitizers, onafhankelijke review |
| Serverhostingkosten | Hoog | Lokaal/community-hosted prototype; cloud pas meten na serverprofiling |
| Steam verkeerd begrepen als hostingprovider | Middel | Steam los zien van compute-hosting; gefaseerde integratie |
| Vendor/API churn | Middel | Protocol- en adapterlagen; versies pinnen; ADR’s en lockfiles |
| Geen content/designrichting | Hoog | Eerst kernfantasy en één minimale RPG-lus definiëren |

---

## 8. Brainstormvragen voor de volgende sessie

### Spel en doelgroep

1. Wat moet een speler in de eerste vijf minuten doen dat uniek voelt?
2. Is de kernfantasy bouwen, graven, ontdekken, vechten, economie of samenwerken?
3. First-person, third-person of beide?
4. Blocky uitstraling, smooth terrein of een hybride?
5. Wat maakt dit een RPG en niet alleen een voxel-sandbox?

### Micro-voxels

6. Hoe groot is één voxel in de spelwereld—1 m, 10 cm, 1 cm?
7. Moet ieder object uit dezelfde voxels bestaan, of alleen terrein/gebouwen?
8. Moeten edits onbeperkt zijn of kosten/gereedschap/claims vereisen?
9. Zijn er slopes/organische vormen nodig in de eerste demo?
10. Hoeveel verschillende materialen zijn in de eerste slice nodig?

### Multiplayer/MMO

11. Is het eerste doel 2–8 co-op, 32 spelers per zone of meteen een persistent shared world-concept?
12. Mag de community servers hosten?
13. Eén wereld voor iedereen of meerdere shards/servers?
14. Hoe belangrijk zijn offline persistence en economie in prototype 1?
15. Welke acties moeten strikt server-authoritative zijn?

### Techniek en eigendom

16. Betekent “custom engine” dat Godot/Bevy verboden is, of dat de voxel- en netwerkcore eigen moet zijn?
17. Welke hardware heeft de gebruiker en wat is de minimale doelhardware?
18. Alleen Windows/Steam of later ook Linux/Steam Deck?
19. Is Rust acceptabel, of heeft C++/C#/GDScript voorkeur?
20. Mag de eerste demo visueel eenvoudig zijn zolang de techniek bewezen is?

### Proces

21. Hoeveel uur per week kan de gebruiker reviewen, testen en beslissingen nemen?
22. Mag Hermes zelfstandig lokale branches/commits maken, maar nooit pushen/publiceren zonder toestemming?
23. Welke acties vereisen altijd expliciete goedkeuring?
24. Willen we wekelijkse demo’s en budgetrapporten?
25. Welk resultaat zou na drie maanden als een duidelijke mislukking of juist succes voelen?

---

## 9. Aanbevolen eerstvolgende stap

Nog niet aan de engine beginnen. Eerst samen de vragen 1, 6, 11, 16, 17 en 21 beantwoorden. Daarna maakt Hermes:

1. een éénpagina-productvisie;
2. een meetbaar 12-weken-MVP-contract;
3. drie eerste ADR’s (stack, voxelrepresentatie, multiplayerdoel);
4. een kleine technische spikeplanning voor week 1–3;
5. pas daarna repository, CI en code.

---

## 10. Aanvulling uit drie onafhankelijke onderzoeksreviews

Drie parallelle Hermes-onderzoekers hebben na het opstellen van dit plan hun resultaten opgeleverd. Hun bevindingen versterken de hoofdrichting, met één belangrijke architectuurvariant die bewust nog niet definitief wordt gekozen.

### 10.1 Gedeelde conclusies

Alle reviews adviseren:

- geen volledige MMO als eerste mijlpaal;
- een procedurele basiswereld met alleen sparse wijzigingen persistent opgeslagen;
- integer wereldcoördinaten en een lokale/floating render-origin;
- server-authoritative acties en chunkgebaseerd interest management;
- voxeldata/edits repliceren, niet gegenereerde meshes;
- gewone UDP/SteamNetworkingSockets eerst, SDR pas later;
- Steam zien als distributie-, identity-, discovery- en transportlaag, niet als gratis compute-hosting;
- gratis modellen voor routinewerk en betaalde modellen alleen via expliciete gates;
- compiler, tests, fuzzing, benchmarks en profiler als autoriteit—niet modelconsensus.

### 10.2 Concreet micro-voxeluitgangspunt voor de eerste benchmark

Een onderzoeker stelde **12,5 cm** als LOD0-cel voor. Dat geeft een spelerhoogte van ongeveer 14–16 cellen en is veel realistischer dan 1–5 cm. Bij 12,5 cm bevat een dense kubus van 32 meter al 16.777.216 voxels en kost die bij vier bytes per voxel circa 64 MiB, exclusief meshes, colliders en overhead. Daarom blijft de aanbevolen representatie hiërarchisch/sparse.

Te benchmarken startconfiguratie:

- sparse bricks van `8³` samples;
- meshblocks van `32³` cellen plus een sample-halo;
- afzonderlijke begrippen voor brick, meshblock en persistence-region;
- uniforme bricks zonder payload;
- procedurele basis + gematerialiseerde, gecomprimeerde editbricks;
- revision-ID op mesh-, collider-, save- en netwerktaken zodat verouderde jobresultaten worden verworpen.

Deze waarden zijn hypothesen, geen definitieve specificatie.

### 10.3 Blocky versus smooth: expliciete fork

De hoofdroadmap beveelt blocky/palette voxels met culled/greedy meshing aan als laagste-risico-MVP. Een onafhankelijke review adviseert juist smooth density/SDF-terrein met CPU Marching Cubes en later Transvoxel. Beide zijn technisch verdedigbaar:

| Richting | Voordeel | Risico |
|---|---|---|
| Blocky + greedy meshing | Eenvoudiger data, edits, collisions, materiaalgrenzen en netwerktests | Minder organische uitstraling |
| Smooth SDF + Marching Cubes/Transvoxel | Grotten, bogen en sculpting voelen natuurlijker | Veel complexere seams, materiaalblending, collisions en determinisme |

**Beslisregel:** vóór productie maken we een kleine, begrensde spike van beide representaties met dezelfde edit-, geheugen- en frametimebenchmarks. De gewenste visuele identiteit weegt mee, maar de keuze wordt met meetdata vastgelegd in een ADR.

### 10.4 Derde client-/rendereroptie

Naast Godot + GDExtension en Bevy/wgpu stelde een review **C++20 + Vulkan 1.3 + SDL3 + Jolt + CMake/vcpkg** voor. Dat biedt maximale engine-eigendom, maar vergroot de infrastructuur- en debugginglast aanzienlijk. Deze route wordt alleen als derde benchmarkkandidaat toegelaten wanneer “custom” voor de gebruiker expliciet betekent dat ook renderer en platformlaag zelf gebouwd moeten worden. Anders blijft hij buiten het twaalfweken-MVP.

### 10.5 Aanvullende networking- en persistence-eisen

- Betrouwbaar/ordered voor inventory, voxeltransacties, spawns en queststate; unreliable/sequenced voor frequente transforms.
- Per client een bytebudget en prioriteitenqueue; grote chunktransfers mogen beweging/combat niet blokkeren.
- Iedere persistente transactie krijgt een idempotency-ID.
- Bij meerdere servers is precies één eigenaar per chunk/entity nodig, met lease plus epoch/fencing token.
- SQLite/WAL is geschikt voor de slice, maar niet als gedeelde MMO-database over meerdere worldservers.
- Back-up telt pas als betrouwbaar nadat restore automatisch is getest.
- Steam Game Bans/VAC vervangen geen eigen servervalidatie en cheatdetectie.

### 10.6 Aangescherpte OpenRouter-regels

- Gratis modelbeschikbaarheid en vervaldata worden via `https://openrouter.ai/api/v1/models` ontdekt; geen permanente afhankelijkheid van één gratis model.
- `openrouter/free` is geschikt voor losse research/triage, maar minder voor reproduceerbare patches omdat selectie varieert.
- Maximaal twee gratis pogingen per fout; daarna de reproducer/diagnose verbeteren of expliciet escaleren.
- Betaald gebruik vereist: failing test of concrete reviewvraag, geselecteerde bestanden, acceptatiecriteria en outputlimiet.
- Voorgestelde cumulatieve waarschuwingen in USD: `$10`, `$22`, `$32`, paid stop rond `$36`, met `$4` eindreserve. De precieze euro/dollar- en transactiekosten moeten bij aankoop opnieuw worden gecontroleerd.
- No-agent cronjobs zijn geschikt voor balanscontrole, modelcatalogus, tests en benchmarkvergelijking zonder LLM-kosten.

---

## 11. Vastgelegde productantwoorden van de gebruiker

### Inspiratie en visuele/technische richting

De belangrijkste referenties zijn:

- **Lay of the Land** — kleine destructible/buildable voxels en simulatie;
- **Voxtopolis** — ray-traced voxelwereld en grote visuele dichtheid;
- **John Lin** — hoge detailgraad, LOD/compressie, voxeldata als centrale ontwerpkwestie;
- **Tantan** — Rust/wgpu/Bevy-experimenten, destructie en transparante ontwikkelvideo’s;
- researchpapers, social media, GitHub en zoveel mogelijk open-sourceimplementaties.

Dit wijst niet op een eenvoudige Minecraft-kloon, maar op een gedetailleerde, dynamische voxelwereld waarin bouwen, vernietigen, simulatie en schaal samengaan. John Lins waarschuwing wordt als ontwerpprincipe overgenomen: rendering alleen is niet genoeg; dezelfde data moet ook bruikbaar zijn voor physics, networking, persistence, AI en bewerking.

### Wereldschaal

De beoogde uiteindelijke wereldoppervlakte is circa **150 km²**. Als die wereld ongeveer vierkant is, is dat circa **12,247 × 12,247 km**.

Een volledige dense wereld is onmogelijk praktisch:

- bij voxels van 50 cm: 600 miljoen voxels per enkele horizontale laag;
- bij 12,5 cm: 9,6 miljard per laag;
- bij 10 cm: 15 miljard per laag;
- bij 1 cm: 1,5 biljoen per laag.

Daarom betekent 150 km² nadrukkelijk **adresruimte en procedurele wereldomvang**, niet dat alles tegelijk in RAM, op disk of op de GPU staat. De architectuur moet vanaf het begin gebruiken:

- deterministische procedurele generatie;
- region/chunk/brick-streaming;
- sparse opslag;
- meerdere detailniveaus;
- alleen wijzigingen en belangrijke objecten persistent opslaan;
- lokale/floating origin;
- bounded resident working set rond actieve spelers.

De eerste vertical slice blijft klein, maar gebruikt dezelfde coördinaten en interfaces zodat opschaling geen wereldformaatrewrite vereist.

### Micro-voxelformaat

De exacte minimumresolutie is nog niet definitief gekozen. Op basis van de inspiratie is een hybride model waarschijnlijker dan één uniforme voxelmaat:

- grovere voxels/impliciete data voor verre en ongewijzigde wereld;
- circa 10–12,5 cm als eerste nabij-terreinbenchmark;
- eventueel lokale verdere onderverdeling voor bouwen, vernietiging of objectdetail;
- normale meshes/shaders/props wanneer voxels geen gameplayvoordeel bieden.

We benchmarken daarom minimaal blocky/palette en smooth/SDF, plus adaptieve lokale detaillering. “Zo klein mogelijk” is geen criterium; bruikbare data, editlatency, geheugen, physics en netwerkbaarheid zijn dat wel.

### Multiplayer

We beginnen klein:

1. singleplayer/headless simulatie;
2. 2–8 spelers op één authoritative server;
3. daarna 16–32 per instance/zone als metingen dat toelaten;
4. pas veel later persistent shared-world- en MMO-R&D.

### Beschikbare ontwikkelhardware

Persoonlijke ontwikkel-/testmachine van de gebruiker:

- NVIDIA RTX 4080 Super;
- 32 GB RAM;
- Intel Core Ultra 7 265K, opgegeven rond 3,9 GHz;
- Windows 10.

Dit is krachtige ontwikkelhardware, maar mag niet onze minimumdoelhardware worden. We moeten twee profielen hanteren:

- **ontwikkelprofiel:** deze pc, voor zware profiling en snelle iteratie;
- **minimumdoelprofiel:** later expliciet kiezen, waarschijnlijk een veel zwakkere doorsnee gaming-pc.

De engine krijgt configureerbare RAM/VRAM-, view-distance-, LOD- en jobbudgetten. Tests moeten kunstmatig lagere budgetten kunnen afdwingen.

### Menselijke betrokkenheid en autonomie

De gebruiker heeft geen programmeer- of engine-ervaring en wil minimale betrokkenheid. Hermes draagt daarom verantwoordelijkheid voor:

- onderzoek, architectuur, planning en taakdecompositie;
- implementatie, tests, benchmarks en documentatie;
- begrijpelijke samenvattingen zonder vakjargon waar mogelijk;
- standaardkeuzes maken wanneer risico en impact laag zijn;
- alleen betekenisvolle productkeuzes of risicovolle acties voorleggen;
- builds zo verpakken dat testen neerkomt op starten, spelen en eenvoudige observaties rapporteren.

De gebruiker hoeft geen code te beoordelen. Menselijke gates blijven wel gelden voor uitgaven, publicatie, externe accounts, licentie-/productrichting, destructieve acties en grote scopekeuzes.

---

## 12. Bronnen

### Voxel/rendering

- Transvoxel-overzicht, paper en tabellen: https://transvoxel.org/
- Greedy meshing en trade-offs: https://0fps.net/2012/06/30/meshing-in-a-minecraft-game/
- Meshing, materialen en normals: https://0fps.net/2012/07/07/meshing-minecraft-part-2/
- NVIDIA, Efficient Sparse Voxel Octrees: https://research.nvidia.com/sites/default/files/pubs/2010-02_Efficient-Sparse-Voxel/laine2010tr1_paper.pdf
- wgpu-documentatie: https://docs.rs/wgpu/latest/wgpu/
- Bevy-documentatie: https://docs.rs/bevy/latest/bevy/
- John Lin, *The Perfect Voxel Engine*: https://voxely.net/blog/the-perfect-voxel-engine/
- John Lin GitHub en open onderzoeksrepositories: https://github.com/Lin20
- Tantan, eerste Rust/wgpu voxel-engine (MIT): https://github.com/TanTanDev/first_voxel_engine
- Cubiquity, actieve experimentele micro-voxel-engine (CC0): https://github.com/DavidWilliams81/cubiquity
- Curated open-source voxelprojecten: https://github.com/DrSensor/awesome-opensource-voxel
- Godot GDExtension: https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/what_is_gdextension.html
- Godot compute shaders: https://docs.godotengine.org/en/stable/tutorials/shaders/compute_shaders.html

### Multiplayer/server/Steam

- Godot high-level multiplayerconcepten: https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html
- Godot dedicated-serverexport: https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_dedicated_servers.html
- Steam Game Servers: https://partner.steamgames.com/doc/features/multiplayer/game_servers
- Steam Networking: https://partner.steamgames.com/doc/features/multiplayer/networking
- Steam Datagram Relay: https://partner.steamgames.com/doc/features/multiplayer/steamdatagramrelay
- Steam Direct fee: https://partner.steamgames.com/doc/gettingstarted/appfee

### Hermes/OpenRouter

- Hermes-configuratie: https://hermes-agent.nousresearch.com/docs/user-guide/configuration
- Hermes Kanban: https://hermes-agent.nousresearch.com/docs/user-guide/features/kanban
- Hermes fallback providers: https://hermes-agent.nousresearch.com/docs/user-guide/features/fallback-providers
- OpenRouter limieten: https://openrouter.ai/docs/api/reference/limits
- OpenRouter free router: https://openrouter.ai/docs/guides/routing/routers/free-router
- OpenRouter free variants: https://openrouter.ai/docs/guides/routing/model-variants/free
- OpenRouter modelcatalogus/API: https://openrouter.ai/models?pricing=free en https://openrouter.ai/api/v1/models

> Broninformatie, modelbeschikbaarheid, prijzen en rate limits zijn tijdsgevoelig en moeten vóór configuratie of betaling opnieuw worden gecontroleerd.
