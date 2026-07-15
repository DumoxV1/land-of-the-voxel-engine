# State-of-the-Art Voxel Engine Research Program

## Opdracht

Analyseer alle opgegeven losse YouTube-video's en kanalen, minimaal 30.000 woorden per video- of kanaaldossier. Breid elk dossier uit met primaire bronnen: GitHub-repositories, papers, forums, open-source engines, technische blogs en reproduceerbare benchmarks. Vergelijk de bevindingen uiteindelijk met de huidige Land of the Voxel Engine-codebase en adviseer per subsysteem: **behouden**, **gericht aanpassen**, **gefaseerd vervangen**, of **niet overnemen**.

## Harde regels

1. Alle autonome onderzoekers gebruiken `openrouter/free`; geen betaalde modellen of automatische fallback.
2. Een dossier telt pas als voltooid bij minimaal 30.000 Nederlandse woorden exclusief bronlijst en transcript.
3. YouTube-claims zijn hypothesen totdat een primaire bron of reproduceerbaar experiment ze ondersteunt.
4. Elk dossier bevat tegenbewijs, beperkingen, hardware/context, licentie/IP-status en implicaties voor onze Rust/wgpu-codebase.
5. Geen codewijzigingen tijdens dit onderzoek; alleen researchbestanden onder deze map.
6. Elke run schrijft één hoofdstuk van circa 5.000–7.000 woorden en werkt zijn lane-manifest atomair bij. Zo vermijden we output-/contextafkapping.
7. Directe bronnen (video, kanaal, repo, paper) gaan vóór blogs en modelkennis.

## Dossierstructuur (minimaal 6 hoofdstukken)

1. **Bron en claims** — transcript, tijdcodes, auteurscontext, concrete technische claims.
2. **Architectuur en datastructuren** — representatie, chunking, DAG/octree/grid, meshing/raycasting, threading, GPU-pad.
3. **Algoritmen en implementatiedetails** — pseudocode, memory layout, update-/editpad, LOD, streaming.
4. **Bewijs en tegenbewijs** — benchmarks, hardware, reproduceerbaarheid, failure modes, communitykritiek.
5. **Vergelijking met onze codebase** — exacte crates/bestanden/invarianten en migratie-impact.
6. **Besluit en experimenten** — retain/update/replace/reject, risico, ADR-kandidaat, meetbare tracer bullets.

## Lanes

- **Lane A — Adaptive grids & ecosystem:** adaptive voxel grids-video, papers/repositories en minimaal 10 aanvullende primaire bronnen; tevens Lay of the Land, VoxTopia/Voxtopolis, Tantan, John Lin en Tooly1998 als cross-reference.
- **Lane B — Videos/channels set A:** eerste helft van de opgegeven video's en kanalen.
- **Lane C — Videos/channels set B:** tweede helft van de opgegeven video's en kanalen.

## Eindproducten

- `lane-a/`, `lane-b/`, `lane-c/`: individuele 30k-dossiers plus bronnenbestanden.
- `reviews/`: onafhankelijke claim- en bronreviews.
- `CODEBASE_GAP_ANALYSIS.md`: subsystem-by-subsystem vergelijking met onze actuele code.
- `RETAIN_UPDATE_REPLACE_MATRIX.md`: beslismatrix met kosten, risico, verwachte winst en experiment.
- `ADAPTIVE_VOXEL_GRIDS_VERDICT.md`: apart technisch en productmatig oordeel.
- `FINAL_SYNTHESIS.md`: geprioriteerd langetermijnadvies; geen rewrite zonder bewijs.

## Gepauzeerde ontwikkelstatus

De client-crate-extractie was begonnen maar niet afgemaakt. Huidige on-gecommitte wijzigingen kunnen omvatten: workspace-member `voxel-client`, `crates/voxel-client/Cargo.toml`, en een gekopieerde/aangepaste `src/lib.rs`. Deze code niet wijzigen tijdens research. Na het onderzoeksprogramma wordt de extractie hervat vanaf de git-status en opnieuw geverifieerd.

## Acceptatie

Een aanbeveling wordt pas `accepted` wanneer:
- primaire bronnen live gecontroleerd zijn;
- relevante licenties bekend zijn;
- claims reproduceerbaar of expliciet onzeker zijn;
- de vergelijking concrete codepaden noemt;
- een kleine benchmark/tracer bullet is gespecificeerd;
- een onafhankelijke reviewer geen blocker vindt.
Remember: compiler/tests/benchmarks en primaire bronnen gaan vóór modelconsensus.
