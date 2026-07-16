# Branch & release strategy

## Branches
- `master` — de actieve development-branch. Hierop wordt direct gepusht voor kleine fixes
  en (na autonome goedkeuring) voor features. Dit is de enige "source of truth" branch.
- Voor niet-triviale work (nieuwe subsystemen, API-breaks, refactors > 100 regels) maak je
  een topic-branch en een PR naar `master`:
  - `feat/<korte-naam>` — nieuwe functionaliteit
  - `fix/<korte-naam>` — bugfix
  - `chore/<korte-naam>` — onderhoud, docs, tooling
- `origin/main` is een achterlopende legacy-branch (niet meer gebruikt). Niet naar pushen.

## Commits
- Prefix met type: `feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `chore:`, `test:`.
- Nederlandse samenvatting in de eerste regel (< 72 chars), details daaronder.
- Elke commit moet groen zijn (tests + build) tenzij expliciet anders aangegeven.

## Releases / tags
- Bij mijlpalen (vertical slice, speelbare demo, API-stabilisatie): `git tag -a vX.Y.Z -m "..."`
  en `git push origin --tags`.
- Semver: X = major API-break, Y = feature, Z = fix.

## Issues & labels
- `bug` — iets werkt niet zoals bedoeld
- `enhancement` — nieuwe functionaliteit
- `perf` — performance
- `visual` — rendering / shaders / assets
- `worldgen` — terrain / biome / cave generatie
- `profiling` — Tracy / metrics / observability
- `docs` — documentatie
- `chore` — tooling / build / onderhoud
- `blocked` — wacht op externe input (gebruiker, asset, API-key)
