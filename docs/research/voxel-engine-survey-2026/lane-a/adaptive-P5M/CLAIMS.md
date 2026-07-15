# CLAIMS — adaptive-P5M dossier (P5M_QiamXvw)

Status-legenda: `hypothesis` (niet geverifieerd) · `supported` (primaire bron ondersteunt) ·
`contested` (tegenbewijs) · `rejected`.

Bijgewerkt: 2026-07-15 (run 1, hoofdstuk 1). Onderzoeker: Lane A.

## Inhoudelijke claims over de video

| ID | Claim | Status | Bron | Reproduceerbaarheid | Notitie |
|---|---|---|---|---|---|
| C1 | De video toont een surface-adaptive voxel grid rond een mensfiguur | hypothesis | Titel "Adaptive Voxel Grid (Human)"; H1 in hoofdstuk 1 | Alleen via frame-inspectie of contact maker | Transcript disabled; frames niet gedecodeerd in deze run |
| C2 | De grid is opgebouwd via recursive subdivision in Blender Geometry Nodes | hypothesis | "(Geometrynodes)"-variant (Reddit zh6fin, titel geverifieerd); Cartesian Caramel tutorial `_2PkrmpMmQA` | Alleen via node-graaf/maker | Body Reddit-thread niet gescraped |
| C3 | Resolutie is fijn nabij het oppervlak en grof in de lege ruimte | hypothesis | Titel "adaptive" + SVO-analogie (Laine-Karras) | Alleen via frame-inspectie | Kern van "adaptive" in voxelcontext |
| C4 | Surface-adaptive voxelization is een bewezen patroon (SVO/ADF) | supported | Laine & Karras 2010 (I3D, NVIDIA), benchmarks 60,9/122 M stralen/s; poelzi repo BSD-3 | Ja (paper + repo live) | Concept bewezen, niet de 2010-hardware-cijfers |
| C5 | De short is een statische/artistieke demo, geen realtime engine | hypothesis | Shorts-vorm; geen engine/gameplay-context in metadata; Gumroad-gerichtheid | Gedeeltelijk via metadata | Laag risico |
| C6 | Adaptieve voxelgrids zijn ook voor realtime simulatie bruikbaar | supported (cross-ref) | Grant Kot "Voxel Physics Devlog #1" (yY8I-gWP0oY), MPM/FLIP, adaptive MPM | Ja (publieke devlog) | Ondersteunt Fase-5 LOD-richting |

## Licentie- / IP-claims

| ID | Claim | Status | Bron | Reproduceerbaarheid |
|---|---|---|---|---|
| L1 | Video valt onder standaard YouTube-licentie (maker houdt copyright) | supported | oEmbed/YouTube | Ja |
| L2 | Gumroad-projecten van de maker zijn commercieel (betaald) | supported | Beschrijving `gumroad.com/bbbn19` | Ja |
| L3 | SVO-referentie-implementatie (poelzi) is BSD-3-Clause | supported | poelzi/efficient-sparse-voxel-octrees LICENSE | Ja, live geverifieerd |
| L4 | De SVO-paper (Laine-Karras) geeft géén open-source-licentie zelf | supported | Paper header, geen license-clausule | Ja |
| L5 | Blender is GPLv3; Geometry-Nodes-setups zijn maker-IP, geen engine-broncode | supported | Algemene Blender-licentie + kanaalcontext | Ja |

## Openstaande verificaties

- Frame-inspectie van P5M_QiamXvw (vision-tool) om C1–C3, C5 te bevestigen/weerleggen.
- Body van Reddit-thread zh6fin lezen voor C2-onderbouwing.
- Volledige watch-page-beschrijving ophalen (OG-meta was afgekapt).
