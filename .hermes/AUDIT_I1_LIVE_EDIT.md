# AUDIT I1 — Live Voxel Edit

Onafhankelijke code-review van `raycast_voxel` (voxel-edit/src/lib.rs:151-230),
`edit_at_look` + koppeling (voxel-client/src/lib.rs:916-953) en de `edited`-guard
(voxel-client/src/lib.rs:671-678). Geen bron gewijzigd.

## 1. `raycast_voxel` — DDA
**OK.** Amanatides-Woo correct: `t_max` init (183-193), laagste-as stap (207-213),
`normal = -step[a]` (218-221) is de binnenkomende face. Div-by-zero geguard via
`if step[a]!=0` (183-193). Lege lucht → `None` (229). Start-in-solid →
`(start, 0)` (199-202), acceptabel.
**WAARSCHUWING.** Een schot in lege lucht genereert tot `max_steps`≈1600 chunks
(`material_at` mist→`generate_chunk`, 70). ~51 MB + hitch per klik in de hemel.
Fix: early-out als `material_at`源 chunk gecached/lucht is, of limiteer stappen.
**WAARSCHUWING.** Start-in-solid geeft `normal=(0,0,0)`; bij place wordt
`target=hit` (blok in jezelf). Zelden, maar plaats-dan op `hit+normal` faalt.

## 2. Client-koppeling `edit_at_look` (916-953)
**OK.** Eye→voxel (919-923), dir (925-927), `max_dist=200/VS` (928), place=hit+normal
(934-935), remove=hit (937), re-mesh via `take_dirty` (941-952).
**BUG (WAARSCHUWING→blokkerend).** Re-mesh doet alleen de dirty chunk van het
bewerkte voxel. Bij edit op een chunk-grens (elke 4 m) wordt de buur-chunk niet
her-mesht: verwijderen toont een gat/ontbrekende face aan de grens (zichtbaar).
Fix: her-mesh ook de 6 neighbour-chunks van hit én target.
**WAARSCHUWING.** Origin `as i64` trunkt naar nul (919-923); bij negatieve
eye-coord is de start-voxel 1 off (raycast floort opnieuw). Fix:
`(eye_m[i]/VS).floor() as i64`.

## 3. `edited`-guard (671-678)
**OK.** `if !self.edited.contains(&coord)` blokkeert `World::insert` voor bewerkte
chunks — correcter dan de oude `!dirty_chunks()` (die na `take_dirty` leegliep).
**WAARSCHUWING (latent).** `WorkerMsg::Mesh` (679-681) is ongeguard; bij LRU-evict
+ herplanning overschrijft de worldgen-mesh de edit (wereld behoudt edit, mesh niet).
Cap 200k/12 GB maakt dit in praktijk onbereikbaar.

## 4. Regressie inputs
**OK.** `MouseButton::Left` zet enkel `dragging` (499-504); edit alleen op
Right/Middle press (505-512). WASD/F/Space onaangetast. Geen regressie.

## 5. Forward-vector
**OK.** `edit_at_look` dir `[cp*cy, sy, sp*cy]` (925-927) identiek aan fly-fwd
(update_camera 584-586). Consistent.

## VERDICT: NEEDS FIX
Blokkerend: grens-edit re-mesh incompleet (zichtbare artifact, veelvoorkomend) +
negatieve-coord origin-truncatie. Overige punten OK/laag-risico.
