// M1.1 세계 골격 CLI·결정론 통합 테스트

use epoch_core::{generate_world, validate_world};
use std::path::PathBuf;
use std::process::Command;

fn run_epoch_lab(args: &[&str]) -> std::process::Output {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // crates
    root.pop(); // workspace root
    Command::new("cargo")
        .current_dir(&root)
        .args(["run", "-q", "-p", "epoch-lab", "--"])
        .args(args)
        .output()
        .expect("cargo run -p epoch-lab")
}

#[test]
fn cli_world_1_succeeds() {
    let output = run_epoch_lab(&["world", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"schema_version\""), "stdout: {stdout}");
    assert!(stdout.contains("\"territories\""), "stdout: {stdout}");
    assert!(stdout.contains("\"realms\""), "stdout: {stdout}");
    assert!(stdout.contains("\"rulers\""), "stdout: {stdout}");
}

#[test]
fn cli_world_check_1_prints_world_ok() {
    let output = run_epoch_lab(&["world-check", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("WORLD_OK"), "stdout: {stdout}");
    assert!(stdout.contains("seed=1"), "stdout: {stdout}");
    assert!(stdout.contains("realms=6"), "stdout: {stdout}");
    assert!(stdout.contains("territories=36"), "stdout: {stdout}");
    assert!(stdout.contains("rulers=6"), "stdout: {stdout}");
}

#[test]
fn cli_world_check_2_prints_world_ok() {
    let output = run_epoch_lab(&["world-check", "2"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("WORLD_OK"), "stdout: {stdout}");
    assert!(stdout.contains("seed=2"), "stdout: {stdout}");
}

#[test]
fn generate_world_api_seed_1_and_2() {
    let w1 = generate_world(1).expect("seed1");
    let w2 = generate_world(2).expect("seed2");
    validate_world(&w1).expect("v1");
    validate_world(&w2).expect("v2");
    assert_ne!(
        w1.to_compact_json_bytes().unwrap(),
        w2.to_compact_json_bytes().unwrap()
    );
}
