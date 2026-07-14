//! S-11 audit-hardening tests (strict TDD — RED before fixes).
//!
//! Audit finding S-01s: tick order over players came from HashMap iteration order
//! (RandomState), which is not deterministic across processes. With multiple players
//! sharing a world, simulation must be reproducible: same seed + same inputs => same state.

use voxel_server::Server;

fn run(seed: u32) -> Vec<[f32; 3]> {
    let mut s = Server::new(seed);
    // Enough players that HashMap ordering would matter if used.
    for id in 0..8u32 {
        s.add_player(id, [10.0 + id as f32 * 3.0, 40.0, 10.0 + id as f32 * 2.0]);
    }
    for i in 0..240 {
        // Alternate inputs so movement interacts with terrain generation.
        for id in 0..8u32 {
            let input = if (i + id as u64) % 3 == 0 {
                voxel_player::Input::forward()
            } else {
                voxel_player::Input::none()
            };
            s.set_input(id, input);
        }
        s.tick(1.0 / 60.0);
    }
    (0..8u32).map(|id| s.player(id).unwrap().pos).collect()
}

/// Two identical runs in the same process must agree bit-for-bit for all 8 players.
/// (Cross-process determinism is covered by the B-06 replay benchmark later; this test
/// at minimum forces a stable iteration order.)
#[test]
fn eight_player_simulation_is_deterministic() {
    let a = run(1234);
    let b = run(1234);
    assert_eq!(a, b, "same seed + same inputs must give identical positions");
}
