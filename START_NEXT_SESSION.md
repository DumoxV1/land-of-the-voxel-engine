# Volgende Hermes-sessie: tokenzuinig naar engine-start

## Voor je een nieuwe sessie start

1. Kies in de Hermes Desktop-modelkiezer **`openrouter/free`** of een actueel gratis tool-callingmodel met `:free`.
2. Start binnen het project **Land of the Voxel Engine** een nieuwe chat (`/new` kan ook).
3. Plak uitsluitend de korte startprompt hieronder. Plak niet het lange plan of deze hele conversatie; Hermes leest `AGENTS.md`, `.hermes/PROJECT_STATE.md` en het canonieke plan uit de projectmap.

## Korte startprompt

```text
Ga autonoom verder met Land of the Voxel Engine. Lees eerst AGENTS.md en .hermes/PROJECT_STATE.md, daarna alleen de relevante delen van het canonieke plan. Controleer het Kanban-board land-of-the-voxel-engine. Rond eerst de actieve researchreview/plansynthese en alle blocking findings af. Start pas daarna de eerste goedgekeurde voxel-core tracer bullet volgens strict TDD. Gebruik standaard gratis OpenRouter-modellen, bewaak het budget en voer na iedere derde stap de verplichte terugstapcontrole uit. Rapporteer alleen blockers, gates en echte testresultaten in het Nederlands.
```

## Waarom dit tokens bespaart

- Nieuwe sessie zonder lange chathistorie.
- Compacte `AGENTS.md` wordt automatisch geladen.
- Projectstatus en besluiten staan in bestanden/Kanban, niet in de prompt.
- Eén afgebakende tracer bullet per sessie.
- `session_search` alleen gebruiken wanneer een besluit niet in de projectdocs staat.
- Geen brede subagent-swarms; één implementer plus één reviewer.

## Canonieke verificatie tijdens de voorbereidingsfase

```bash
python -m unittest discover -s tests -p 'test_*.py' -v
python scripts/plan_alignment_check.py
python scripts/openrouter_guard.py
hermes kanban --board land-of-the-voxel-engine list
hermes gateway status
hermes cron status
```

## Engine-startgate

Productiecode mag pas beginnen wanneer:

- researchreview klaar is;
- blocking findings zijn verwerkt;
- architectuursynthese klaar is;
- het canonieke plan en PROJECT_STATE zijn bijgewerkt;
- een exact implementatieplan voor de eerste `voxel-core` tracer bullet bestaat;
- tests eerst worden geschreven en aantoonbaar rood gaan.

De eerste engine-sessie bouwt dus niet meteen renderer/MMO/gameplay, maar begint met de kleinste bewezen kern: integer wereld-/chunk-/lokale coördinaten plus propertytests en benchmarkharnas.
