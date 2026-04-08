use std::process::Command;
use std::sync::atomic::{AtomicU16, Ordering};

static PORT_COUNTER: AtomicU16 = AtomicU16::new(19_000);

fn alloc_port_pair() -> (u16, u16) {
    let a = PORT_COUNTER.fetch_add(2, Ordering::Relaxed);
    (a, a + 1)
}

fn game_binary() -> String {
    let test_exe = std::env::current_exe().expect("current_exe");
    let target_dir = test_exe
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find target/debug");
    let name = if cfg!(windows) {
        "rengine-fps-mp.exe"
    } else {
        "rengine-fps-mp"
    };
    target_dir.join(name).to_string_lossy().into_owned()
}

fn check_sync_result(label: &str, stdout: &str) {
    let trimmed = stdout.trim();
    if trimmed.starts_with("OK ") {
    } else if trimmed == "DESYNC" {
        panic!("{label}: GGRS detected a desync!\nstdout: {stdout}");
    } else {
        panic!("{label}: unexpected output.\nstdout: {stdout}");
    }
}

#[test]
fn online_two_player_sync() {
    let (port_a, port_b) = alloc_port_pair();
    let bin = game_binary();
    let frames = "300";

    let p0 = Command::new(&bin)
        .args([
            "--online",
            "--headless",
            "--demo",
            "--port",
            &port_a.to_string(),
            "--remote",
            &format!("127.0.0.1:{port_b}"),
            "--player",
            "0",
            "--frames",
            frames,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn P0");

    let p1 = Command::new(&bin)
        .args([
            "--online",
            "--headless",
            "--demo",
            "--port",
            &port_b.to_string(),
            "--remote",
            &format!("127.0.0.1:{port_a}"),
            "--player",
            "1",
            "--frames",
            frames,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn P1");

    let out0 = p0.wait_with_output().expect("P0 wait");
    let out1 = p1.wait_with_output().expect("P1 wait");

    let stdout0 = String::from_utf8_lossy(&out0.stdout);
    let stdout1 = String::from_utf8_lossy(&out1.stdout);

    assert!(
        out0.status.success(),
        "P0 exited with {}\nstderr: {}",
        out0.status,
        String::from_utf8_lossy(&out0.stderr)
    );
    assert!(
        out1.status.success(),
        "P1 exited with {}\nstderr: {}",
        out1.status,
        String::from_utf8_lossy(&out1.stderr)
    );

    check_sync_result("P0", &stdout0);
    check_sync_result("P1", &stdout1);
}

#[test]
#[ignore]
fn online_visual() {
    let (port_a, port_b) = alloc_port_pair();
    let bin = game_binary();

    let p0 = Command::new(&bin)
        .args([
            "--online",
            "--demo",
            "--port",
            &port_a.to_string(),
            "--remote",
            &format!("127.0.0.1:{port_b}"),
            "--player",
            "0",
        ])
        .spawn()
        .expect("failed to spawn P0");

    let p1 = Command::new(&bin)
        .args([
            "--online",
            "--demo",
            "--port",
            &port_b.to_string(),
            "--remote",
            &format!("127.0.0.1:{port_a}"),
            "--player",
            "1",
        ])
        .spawn()
        .expect("failed to spawn P1");

    let out0 = p0.wait_with_output().expect("P0 wait");
    let out1 = p1.wait_with_output().expect("P1 wait");

    assert!(out0.status.success(), "P0 exited with {}", out0.status);
    assert!(out1.status.success(), "P1 exited with {}", out1.status);
}
