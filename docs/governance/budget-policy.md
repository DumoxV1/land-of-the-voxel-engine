# LLM-budgetbeleid

## Harde grens
Totaal beschikbaar OpenRouter-budget: **€40 voor drie maanden**. Geen auto-top-up. Aankoop- en wisselkoerskosten tellen mee.

## Default
- `€0` voor scripts, builds, tests, lokale analyse en no-agent cron.
- Gratis OpenRouter-modellen voor research, triage, docs en eerste review.
- Betaalde modellen zijn standaard verboden voor autonome taken.

## Betaalde escalatiegate
Alle voorwaarden zijn vereist:
1. concrete failing test, benchmarkregressie of belangrijke reviewvraag;
2. maximaal twee gratis pogingen;
3. geselecteerde relevante bestanden en beperkte context;
4. duidelijke acceptatiecriteria en maximale output;
5. resterend budget boven de reserve;
6. vooraf vastgelegde reden in Kanban/comment of alignment-log.

## Budgetpotten
- Research/routine: gratis.
- Goedkope betaalde herstelroute: maximaal €12.
- Zware architectuur/security/releasegates: maximaal €8.
- Nood- en eindreserve: minimaal €10.
- Wisselkoers/transactiebuffer: resterende ruimte.

## Waarschuwingen
- €10 totaal gebruikt: review.
- €22 totaal gebruikt: alleen integratie/blockers betaald.
- €30 totaal gebruikt: paid standaard dicht.
- €36 totaal gebruikt: absolute paid stop behalve expliciete menselijke toestemming.

## Monitoring
`scripts/openrouter_guard.py` controleert sleutelusage en gratis modellen zonder inferencecall. Secrets worden uitsluitend uit environment gelezen en nooit gelogd.

Waarschuwingsdrempels worden toegepast op **project-key spend** (`key limit - limit_remaining`), niet op accountbrede historische `usage`. Als de key geen limiet rapporteert, is de status `MONITOR` en mag daaruit geen projectsom worden afgeleid.
