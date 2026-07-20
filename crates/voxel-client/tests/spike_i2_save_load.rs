//! I2 (save/load) integration test — TDD: geschreven vóór `App::save_edits`/`load_edits`
//! bestaan (RED). Test dat een edit in de client na save + reload behouden blijft.

use std::path::PathBuf;
use voxel_core::coords::WorldVoxel;
use voxel_core::palette::MaterialId;
use voxel_client::App;

fn save_path() -> PathBuf {
    std::env::temp_dir().join("voxel_client_i2_test.bin")
}

#[test]
fn client_edit_survives_save_and_reload() {
    let path = save_path();
    let _ = std::fs::remove_file(&path); // schone start

    // Sessie 1: doe een edit en sla op.
    let mut app = App::default();
    let target = WorldVoxel::new(40, 200, 40);
    app.apply_edit(target, MaterialId::from(3));
    app.save_edits(&path);
    assert!(path.exists(), "save file moet bestaan na save_edits");

    // Sessie 2: verse client laadt de save → edit moet terug zijn.
    let mut app2 = App::default();
    app2.load_edits(&path);
    let restored = app2
        .world_mut()
        .get_or_generate(voxel_core::coords::ChunkCoord::from_world(target))
        .get(voxel_core::coords::LocalVoxel::from_world(target));
    assert_eq!(
        restored,
        MaterialId::from(3),
        "edit moet na reload behouden blijven"
    );

    let _ = std::fs::remove_file(&path);
}
