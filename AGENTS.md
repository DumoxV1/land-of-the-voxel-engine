# Land of the Voxel Engine — agentregels

## Canonieke bron
Lees vóór iedere taak eerst:

1. `.hermes/plans/2026-07-14_181851-onderzoek-en-aanpak-voxel-engine.md`
2. `.hermes/PROJECT_STATE.md`
3. de relevante ADR’s en onderzoeksmemo’s onder `docs/`

Het plan is levend en canoniek. Werk het bij wanneer geverifieerd onderzoek, benchmarks of besluiten de koers veranderen. Noteer wijzigingen met datum en reden; stilzwijgende architectuurwijzigingen zijn verboden.

## Missie en kwaliteitslat
Bouw een technisch grensverleggende, filmische 3D micro-voxel openwereld-RPG-engine: qua ambitie de “GTA VI / Crimson Desert onder micro-voxel-engines”. Dit betekent rijke werelddichtheid, dynamiek en schaal — niet het kopiëren van beschermde assets, personages of vormgeving. Correctheid en speelbaarheid gaan vóór maximale dichtheid; “extreem” telt alleen als benchmarks het dragen.

## Verplichte werkwijze
- Werk in kleine, meetbare stappen met acceptance criteria.
- Canonieke voorbereidingscheck: `python -m unittest discover -s tests -p 'test_*.py' -v`, daarna plan- en budgetguard.
- Na iedere **derde voltooide uitvoeringsstap**: ga één stap terug, controleer artifact/test/bron opnieuw, vergelijk met het canonieke plan en corrigeer drift vóór stap 4.
- Geen implementatie zonder voorafgaand plan en relevante failing test/benchmark waar toepasselijk.
- Geen claim “klaar” zonder echte uitvoering en verificatie.
- Eén implementer en één onafhankelijke reviewer; modelconsensus is geen bewijs.
- Compiler, tests, fuzzing, profiler en benchmarks zijn leidend.
- Leg blijvende technische keuzes vast in `docs/architecture/adr/`.

## Kosten en modellen
- Onderzoek, triage, documentatie en eerste reviews worden standaard uitbesteed aan expliciet gepinde **gratis OpenRouter-modellen** (`:free` of `openrouter/free`).
- Betaalde modellen zijn alleen toegestaan na een expliciete escalatiegate met reproducer, relevante bestanden, acceptatiecriteria, outputlimiet en budgetruimte.
- Nooit auto-top-up inschakelen. Nooit API-sleutels committen of tonen.
- Gebruik lokale tools/scripts in plaats van LLM’s voor zoeken in bestanden, rekenen, builds, tests, logfiltering en monitoring.

## Scope- en veiligheidsgrenzen
- Eerste doel: bewezen vertical slice; geen premature seamless-MMO-infrastructuur.
- Server authority, determinisme, versieerbare data en sparse/procedurele wereld zijn harde architectuurprincipes.
- Geen push, publicatie, deployment, aankoop, externe accountactie of destructieve gitactie zonder menselijke goedkeuring.
- Vragen aan de gebruiker alleen wanneer de keuze materieel is. Formatteer ze als Kanban-kaart: **Titel, Waarom nu, Opties, Aanbeveling, Gevolg bij uitstel**.

## Taal
Projectcommunicatie en samenvattingen voor de gebruiker zijn in het Nederlands. Code, identifiers en technische API-documentatie mogen Engels zijn.
