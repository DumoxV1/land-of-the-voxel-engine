# AUDIT_I1_FIX — HER-AUDIT bevestiging I1 live voxel edit

Doel: verifiëren of de 2 blokkerende punten uit AUDIT_I1_LIVE_EDIT.md opgelost zijn.
Bronnen: `crates/voxel-client/src/lib.rs` (edit_at_look), `crates/voxel-edit/src/lib.rs`.

## Aspect 1 — Grens-edit re-mesh incompleet (BUG)
- **OK.** `edit_at_look` bouwt `with_neighbours` (lib.rs:946) en loopt de dx/dy/dz-buren (lib.rs:947–962). De check `dx.abs()+dy.abs()+dz.abs()!=1` her-mesht exact de 6 face-neighbours van elke dirty chunk. Gat bij chunk-grens is gedicht.

## Aspect 2 — Negatieve-coord origin-truncatie (BUG)
- **OK.** `origin` gebruikt nu `(eye_m[i]/VOXEL_SIZE_M).floor() as i64` (lib.rs:920–924, met uitleg-comment). Negatieve coords worden correct naar beneden afgerond, niet naar 0 getrunkt.

## Verdict
**SAFE TO SHIP.** Beide blokkerende punten opgelost.

## Niet-blokkerend (bekend/geaccepteerd)
- Hemel-schot genereert chunks: geaccepteerd (geen edit-fout).
- Start-in-solid geeft normal=(0,0,0): geaccepteerd, niet-blokkerend.
