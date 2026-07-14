# ADR-0003: Multiplayer Target and Authority Model

- **Status:** Accepted (via free-model architect synthesis, reviewed against `review-initial-bundle.md`)
- **Datum:** 2026-07-14
- **Deciders:** voxelarchitect (synthese), voxelreviewer (review)

## Context
De eindvisie is een persistente shared world, maar het plan (§4) verbiedt een volwaardige MMO in de
eerste 12 weken. De gebruiker (PROJECT_STATE / plan §11) start klein: singleplayer/headless sim →
2–8 spelers op één authoritative server → later 16–32 per instance → pas veel later MMO-R&D.
Server-authority, determinisme en versieerbare data zijn harde architectuurprincipes (AGENTS.md).

## Beslissing
- **Architectuur:** server-authoritative fixed-tick (20–30 Hz), headless, **zonder GPU**.
- **Client:** stuurt intenties/input, nooit zijn "ware" positie of voxelresultaat.
- **Interest management:** chunk-granulair; server stuurt alleen relevante chunks/entiteiten/edits.
- **Replicatie:** betrouwbaar/ordered voor login, inventory, chunk-baseline en edits;
  unreliable/sequenced voor frequente transforms.
- **Edits:** per edit wereldpositie, oud/nieuw, actor, server-tick, monotone revisie; idempotent
  via revisie-ID; duplicaat/oude edits worden verworpen.
- **Eerste doel:** 2–8 spelers op één authoritative serverproces, één zone.
- **Steam:** distributie/identity/lobby/transport (SteamNetworkingSockets/SDR); **niet** als gratis
  compute-hosting. Productie-GNS vereist Steamworks-partnership (network-persistence Claim 2/7).
- **Validatie:** server weigert out-of-range/ongeldige edits; clients converteren na packet loss en
  reconnect; soak met bots blijft stabiel.

## Alternatieven
- **Volledige MMO / seamless sharding in fase 1:** verworpen (scope-explosie, risico R-8).
- **Client-authoritative:** verworpen (cheat-gevoelig, breekt determinisme/server als bron).
- **Directe SDR zonder lobby-laag:** uitgesteld; eerste slice gebruikt LAN/direct-IP/localhost.

## Bewijs / benchmarks
- Valve, "Source Multiplayer Networking": server = single source of truth.
- Mikolalysenko, "Networking Voxel Worlds" / Teardown multiplayer postmortem (80.lv): chunk-based
  interest.
- Gaffer on Games, "Snapshot Compression" (CC-BY-NC-SA — research only, niet in commerciële docs):
  delta-techniek; voxel-specifieke grootte nog te meten (Experiment 2, B-04-claim).
- Teardown bandbreedte ~50–80 kbit/s als bovengrens-referentie voor het ≤100 kbit/s/client-doel.

## Gevolgen
- `protocol`-crate met versiecontrole vanaf dag 1; fuzz-tests op parsinglimieten (S-08).
- `dedicated-server` headless binary; `game-sim` deterministic replay-hash voor recovery (S-09).
- Latency/jitter/packet-loss/reconnect gesimuleerd in S-07 soak.
- Netwerkcijfers (≤100 kbit/s, delta <200 B/edit) blijven **targets** tot S-08 meet ze (B-08).

## Herzieningstrigger
- Soak toont dat 2–8 spelers het RAM/CPU-budget op de dev-pc overschrijdt → vroegere sharding-spike.
- Gemeten bandbreedte structural boven 100 kbit/s → aanpassing van tickrate/edit-frequentie of
  compressie (nieuwe ADR + meting).
