# Research protocol

## Doel
Onderzoek moet architectuurkeuzes ondersteunen, niet alleen interessante links verzamelen.

## Standaardworkflow
1. Lees het canonieke plan en `PROJECT_STATE.md`.
2. Definieer de beslissing, hypothese of onzekerheid.
3. Besteed brede bronverkenning uit aan een expliciet gratis OpenRouter-model.
4. Eis primaire bronnen waar beschikbaar: papers, specificaties, officiële documentatie en originele repositories.
5. Verifieer kernclaims zelf tegen de bron.
6. Leg resultaat vast in `docs/research/<onderwerp>.md` met datum, bron-URL, licentie, risico, toepasbaarheid en open vragen.
7. Vertaal uitsluitend bevestigde bevindingen naar ADR, benchmark of planwijziging.

## Bronhiërarchie
1. Peer-reviewed paper of officiële specificatie.
2. Officiële docs/repository van het project.
3. Reproduceerbare open-sourceimplementatie.
4. Technische post van de oorspronkelijke auteur.
5. Communitycontent uitsluitend als startpunt, niet als eindbewijs.

## Verplichte velden per onderzoeksmemo
- Vraag en scope
- Samenvatting
- Claims met bronnen
- Licentie/IP-status
- Reproduceerbaarheid
- Relevantie voor onze north star
- Kosten/complexiteit
- Risico’s en tegenbewijs
- Aanbevolen experiment
- Beslisstatus: hypothese / kandidaat / aangenomen / verworpen

## Kostenregels
- Researchworker: gratis `:free` model of `openrouter/free`.
- Maximaal twee modelpogingen per vraag.
- Gebruik web/file/tools voor extractie; geen model voor mechanische verwerking.
- Betaalde escalatie alleen volgens `docs/governance/budget-policy.md`.

## Controle na iedere derde stap
Na stap 3, 6, 9, enzovoort wordt stap N-1 heropend. Controleer bronkwaliteit, artifact, planrichting en scope. Log de uitkomst in `docs/governance/alignment-log.md`.
