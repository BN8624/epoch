// epoch-app export CLI — 잘못된 입력과 성공 문자열을 검증한다

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_epoch-app"))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!(
            "epoch-app-cli-{}-{}-{nanos}",
            label,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp dir");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cli_export_seed1_prints_ok_line() {
    let tmp = TempDir::new("ok");
    let output = bin()
        .args(["export", "1", tmp.0.to_str().expect("utf8 path")])
        .output()
        .expect("run export");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "APP_EXPORT_OK seed=1 rights_bytes=66222 files=5"
    );
}

#[test]
fn cli_invalid_seed_fails() {
    let tmp = TempDir::new("bad-seed");
    let output = bin()
        .args(["export", "nope", tmp.0.to_str().expect("utf8 path")])
        .output()
        .expect("run export");
    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn cli_missing_args_fails() {
    let output = bin().args(["export"]).output().expect("run export");
    assert!(!output.status.success());
}
