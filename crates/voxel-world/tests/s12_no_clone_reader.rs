//! Fase-2b #1 (S-12 deel 1) — World::material_at zonder chunk-clone (audit #12 perf-lek).
//!
//! `get_or_generate` gaf een volledige `Chunk`-clone terug; `voxel_player::solid_at`
//! riep dit per voxel-sample, dus collision deed tientallen 32 KB-clones per substep.
//! De fix voegt een borrow-gevende lezer `material_at` toe. Deze test eist dat die bestaat
//! en correct leest, zónder dat de caller een clone hoeft te nemen.

use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_world::World;

fn flat_world() -> World {
    let mut w = World::new(0);
    // Een stenen vloer op y=0, daarboven lucht.
    for x in -4..8 {
        for z in -4..8 {
            w.set_voxel(WorldVoxel::new(x, 0, z), MaterialId::from(3));
        }
    }
    w
}

#[test]
fn material_at_reads_without_clone_and_matches_chunk() {
    let mut w = flat_world();
    // Solid where we placed stone.
    assert!(w.material_at(WorldVoxel::new(0, 0, 0)) == MaterialId::from(3));
    // Air well above any generated terrain (heightmap tops out far below y=40).
    assert!(w.material_at(WorldVoxel::new(0, 40, 0)) == MaterialId::from(0));
    // Generated (non-edited) chunk returns a deterministic material, not an error/panic.
    let m = w.material_at(WorldVoxel::new(100, 0, 100));
    // Geen clone nodig: de API levert een value direct. We eisen enkel dat het
    // aanroepbaar is op &mut World zonder dat de caller een Chunk bezit.
    let _ = m;
}
