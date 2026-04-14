use std::process::Command;
use std::time::{Duration, Instant};

fn game_binary() -> String {
    // Prefer Cargo's injected env var (set for integration tests when
    // the crate defines a [[bin]] target). Fall back to walking from
    // the test exe for older Cargo versions or unusual layouts.
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_rengine-feature-everything") {
        return path;
    }
    let test_exe = std::env::current_exe().expect("current_exe");
    let target_dir = test_exe
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find target/debug");
    let name = if cfg!(windows) {
        "rengine-feature-everything.exe"
    } else {
        "rengine-feature-everything"
    };
    target_dir.join(name).to_string_lossy().into_owned()
}

/// Run the kitchen-sink sample in headless demo mode for 600 frames,
/// then verify that all key features were successfully demonstrated
/// by checking the `[FEATURE OK]` log lines in stdout.
#[test]
fn headless_demo() {
    let bin = game_binary();
    let frames = "600";
    let timeout = Duration::from_secs(60);

    let mut child = Command::new(&bin)
        .args(["--headless", "--demo", "--frames", frames])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn kitchen-sink binary");

    let start = Instant::now();
    let output = loop {
        match child.try_wait() {
            Ok(Some(_status)) => break child.wait_with_output().expect("failed to collect output"),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let out = child.wait_with_output().unwrap_or_else(|_| {
                        panic!("headless demo timed out after {timeout:?} and failed to collect output");
                    });
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    panic!(
                        "headless demo timed out after {timeout:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
                    );
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => panic!("error waiting for child process: {e}"),
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "kitchen-sink exited with {}\nstderr: {stderr}\nstdout: {stdout}",
        output.status,
    );

    // Look for "OK <frame_count>" line anywhere in output (on_exit prints after it)
    let ok_line = stdout.lines().find(|l| l.starts_with("OK "));
    assert!(
        ok_line.is_some(),
        "Expected 'OK <n>' line in stdout.\nFull stdout:\n{stdout}"
    );

    // Verify key features were logged.
    //
    // NOTE: Camera2D features (shake, rotation, zoom) are *configured* in
    // update/fixed_update and the feature-log fires there. In headless mode
    // render() is never called, so the actual Camera2D methods are not
    // exercised. The visual_demo test (run with --ignored) covers rendering.
    let required_features = [
        "Engine::set_asset_root",
        "ActionMap",
        "EngineConfig",
        "Globals::set",
        "run_with_scenes",
        "load_resource",
        "PixelCanvas",
        "SpriteSheet",
        "Engine::create_texture",
        "TileMap",
        "TriggerSystem",
        "CollisionLayer",
        "Globals::contains",
        "fixed_update (fixed timestep)",
        "TimeState::fixed_dt",
        "TileMap::collide_rect",
        "aabb_overlap",
        "Animation::update",
        "Rect",
        "Camera2D::shake (via coin)",
        "Camera2D::rotation",
        "Camera2D::zoom",
        "SceneOp::Push (Pause)",
        "SceneOp::Pop (Unpause)",
        "Scene::on_resume",
        "Scene::on_pause",
        "Scene::on_enter",
        "ParticleEmitter::burst",
    ];

    let mut missing = Vec::new();
    for feature in &required_features {
        if !stdout.contains(feature) {
            missing.push(*feature);
        }
    }

    assert!(
        missing.is_empty(),
        "Missing feature verifications in stdout:\n{}\n\nFull stdout:\n{stdout}",
        missing.join("\n  "),
    );

    println!("All {} features verified!", required_features.len());
}

/// Visual demo — runs with a window, ignored in CI.
/// Run manually: cargo test -p rengine-feature-everything -- --ignored visual_demo
#[test]
#[ignore]
fn visual_demo() {
    let bin = game_binary();

    let child = Command::new(&bin)
        .args(["--demo", "--frames", "600"])
        .spawn()
        .expect("failed to spawn visual demo");

    let output = child.wait_with_output().expect("failed to wait");
    assert!(
        output.status.success(),
        "Visual demo exited with {}",
        output.status,
    );
}
