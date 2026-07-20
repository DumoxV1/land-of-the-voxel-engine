# AUDIT I2 (save/load) — voxel-client / voxel-persist

## 1. `save_edits` na elke edit + atomic write
**OK.** `edit_at_look` (lib.rs:976) → `save_edits` (lib.rs:1002) → `voxel_persist::save_world` (persist:38).
Atomic write is correct: tmp-file (`voxel_save.tmp`, persist:55) + `sync_all` (persist:58) + `rename` (persist:60). Crash mid-write cannot corrupt an existing save.
*Fix-aanbeveling:* geen. (Wel perf: heel log elke klik wegschrijven — zie 5.)

## 2. `load_edits` + edited-guard
**BUG (gedeeltelijk).** `load_edits` (lib.rs:1010) laadt world+log, zet `self.world`, markeert `edited` (1019) en replays via `edit_tool.place` (1024). Logisch correct.
MAAR: de edited-guard zit **alleen op Gen** (lib.rs:675 `if !self.edited.contains(&coord)`). De **Mesh**-branch (lib.rs:679) insert **onvoorwaardelijk** in `mesh_cache`. Na load is `mesh_cache` leeg → scheduler plant de bewerkte chunk → worker stuurt wereldgen-mesh → die overschrijft de client-edit visueel. Edit blijft in `world` (collision/raycast OK) maar wordt **verkeerd getekend**.
*Fix:* guard ook Mesh (`if !self.edited.contains(&coord)`) én mesh bewerkte chunks vanuit `self.world` na load (zoals `edit_at_look` doet, lib.rs:963-974).

## 3. `save_path()` = "voxel_save.bin" (relatief)
**WAARSCHUWING.** Relatief t.o.v. cwd → verschilt bij dubbelklik .bat vs `cargo run`. Voor MVP acceptabel (taak erkent dit).

## 4. Test `spike_i2_save_load.rs`
**OK (met gat).** Dekt save→reload round-trip van wereld-staat (assert material, regel 32). Vangt het Mesh-bug (2) NIET, want test leest `world.get`, niet de getekende mesh.
*Fix-aanbeveling:* voeg assert toe dat edit ook na worker-stream (mesh_cache) zichtbaar blijft, of test `edited`-guard.

## 5. Regressie I1 / startup
**OK op startup, WAARSCHUWING op perf.** `run()` laadt alleen `if save.exists()` (lib.rs:1046) — dus NIET per ongeluk altijd load. I1 breekt niet (edit_at_look functioneel intact; `seen`-guard in scheduler maskeert bug 2 in steady-state).
*Fix-aanbeveling:* `save_edits` per klik schrijft het volledige log + `sync_all`; bij veel edits merkbare hitch. Overweeg throttle/debounce of append-only write.

## VERDICT
**NEEDS FIX.** Bug 2 (Mesh onvoorwaardelijk ge-insert → bewerkte chunks renderen als onbewerkt na load) is een echte regressie voor I2's kerndoel (edits blijven zichtbaar na herstart). Overige aspecten OK/acceptabel.
