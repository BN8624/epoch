// 생성 세계 관찰 화면 — RightsWorld와 선택적 SuccessionWorld를 정적 사이트로 내보낸다

use epoch_core::{RightsWorld, SuccessionWorld, generate_rights_world, generate_succession_world};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// export가 관리하는 정적 UI 파일. 이 목록만 복사·덮어쓴다.
const STATIC_UI_FILES: &[&str] = &["index.html", "styles.css", "app.js", "view-model.js"];
const RIGHTS_WORLD_FILE: &str = "rights-world.json";
const SUCCESSION_WORLD_FILE: &str = "succession-world.json";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExportReport {
    seed: u64,
    rights_bytes: usize,
    files: usize,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "help" | "-h" | "--help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "export" => cmd_export(&args),
        "export-succession" => cmd_export_succession(&args),
        other => {
            eprintln!("error: unknown command '{other}'");
            print_usage_stderr();
            ExitCode::from(2)
        }
    }
}

fn print_usage() {
    println!(
        "\
epoch-app — EPOCH generated-world observer

Usage:
  cargo run -p epoch-app -- help
  cargo run -p epoch-app -- export <seed> <output-directory>
  cargo run -p epoch-app -- export-succession <seed> <realm-id> <output-directory>

Commands:
  help               Show this help
  export             Generate a RightsWorld and write a static observer site
  export-succession  Generate a SuccessionWorld overlay and write a static observer site
"
    );
}

fn print_usage_stderr() {
    eprintln!(
        "\
Usage:
  cargo run -p epoch-app -- help
  cargo run -p epoch-app -- export <seed> <output-directory>
  cargo run -p epoch-app -- export-succession <seed> <realm-id> <output-directory>
"
    );
}

fn cmd_export(args: &[String]) -> ExitCode {
    let (seed, output) = match parse_export_args(args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match export_rights_world(seed, &output) {
        Ok(report) => {
            println!(
                "APP_EXPORT_OK seed={} rights_bytes={} files={}",
                report.seed, report.rights_bytes, report.files
            );
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(1)
        }
    }
}

fn cmd_export_succession(args: &[String]) -> ExitCode {
    let (seed, realm_id, output) = match parse_export_succession_args(args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match export_succession_world(seed, &realm_id, &output) {
        Ok(report) => {
            println!(
                "APP_SUCCESSION_EXPORT_OK seed={} realm={} rights_bytes={} succession_bytes={} files={}",
                report.seed,
                report.realm_id,
                report.rights_bytes,
                report.succession_bytes,
                report.files
            );
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(1)
        }
    }
}

fn parse_export_succession_args(args: &[String]) -> Result<(u64, String, PathBuf), String> {
    match args {
        [] => Err("missing seed, realm id, and output directory".to_string()),
        [_] => Err("missing realm id and output directory".to_string()),
        [_, _] => Err("missing output directory".to_string()),
        [seed, realm, output] => {
            let seed = seed
                .parse::<u64>()
                .map_err(|_| format!("invalid seed '{seed}': expected unsigned 64-bit integer"))?;
            if realm.is_empty() {
                return Err("realm id must not be empty".to_string());
            }
            Ok((seed, realm.clone(), PathBuf::from(output)))
        }
        _ => Err(
            "too many arguments: expected export-succession <seed> <realm-id> <output-directory>"
                .to_string(),
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuccessionExportReport {
    seed: u64,
    realm_id: String,
    rights_bytes: usize,
    succession_bytes: usize,
    files: usize,
}

fn export_succession_world(
    seed: u64,
    realm_id: &str,
    output_dir: &Path,
) -> Result<SuccessionExportReport, String> {
    let succession = generate_succession_world(seed, realm_id)
        .map_err(|e| format!("succession world generation failed: {e}"))?;
    let rights = &succession.pre_succession_world.family_world.rights_world;
    let rights_compact = rights
        .to_compact_json_bytes()
        .map_err(|e| format!("failed to serialize RightsWorld: {e}"))?;
    let succession_compact = succession
        .to_compact_json_bytes()
        .map_err(|e| format!("failed to serialize SuccessionWorld: {e}"))?;

    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "cannot create output directory '{}': {e}",
            output_dir.display()
        )
    })?;

    let src_dir = web_dir();
    for name in STATIC_UI_FILES {
        let src = src_dir.join(name);
        let bytes =
            fs::read(&src).map_err(|e| format!("cannot read static UI file '{name}': {e}"))?;
        write_managed_file(output_dir, name, &bytes)?;
    }

    write_managed_file(output_dir, RIGHTS_WORLD_FILE, &rights_compact)?;
    write_managed_file(output_dir, SUCCESSION_WORLD_FILE, &succession_compact)?;

    let exported_rights_path = output_dir.join(RIGHTS_WORLD_FILE);
    let read_rights = fs::read(&exported_rights_path).map_err(|e| {
        format!(
            "failed to re-read exported '{}': {e}",
            exported_rights_path.display()
        )
    })?;
    if read_rights != rights_compact {
        return Err("exported rights-world.json bytes differ from generated compact JSON".into());
    }
    let parsed_rights: RightsWorld = serde_json::from_slice(&read_rights)
        .map_err(|e| format!("exported rights-world.json is not valid RightsWorld JSON: {e}"))?;
    if parsed_rights != *rights {
        return Err("exported rights-world.json does not match generated RightsWorld".into());
    }

    let exported_succession_path = output_dir.join(SUCCESSION_WORLD_FILE);
    let read_succession = fs::read(&exported_succession_path).map_err(|e| {
        format!(
            "failed to re-read exported '{}': {e}",
            exported_succession_path.display()
        )
    })?;
    if read_succession != succession_compact {
        return Err(
            "exported succession-world.json bytes differ from generated compact JSON".into(),
        );
    }
    let parsed_succession: SuccessionWorld =
        serde_json::from_slice(&read_succession).map_err(|e| {
            format!("exported succession-world.json is not valid SuccessionWorld JSON: {e}")
        })?;
    if parsed_succession != succession {
        return Err(
            "exported succession-world.json does not match generated SuccessionWorld".into(),
        );
    }

    Ok(SuccessionExportReport {
        seed,
        realm_id: realm_id.to_string(),
        rights_bytes: rights_compact.len(),
        succession_bytes: succession_compact.len(),
        files: STATIC_UI_FILES.len() + 2,
    })
}

fn parse_export_args(args: &[String]) -> Result<(u64, PathBuf), String> {
    match args {
        [] => Err("missing seed and output directory".to_string()),
        [_] => Err("missing output directory".to_string()),
        [seed, output] => {
            let seed = seed
                .parse::<u64>()
                .map_err(|_| format!("invalid seed '{seed}': expected unsigned 64-bit integer"))?;
            Ok((seed, PathBuf::from(output)))
        }
        _ => Err("too many arguments: expected export <seed> <output-directory>".to_string()),
    }
}

fn web_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web")
}

fn export_rights_world(seed: u64, output_dir: &Path) -> Result<ExportReport, String> {
    let world =
        generate_rights_world(seed).map_err(|e| format!("rights world generation failed: {e}"))?;
    let compact = world
        .to_compact_json_bytes()
        .map_err(|e| format!("failed to serialize RightsWorld: {e}"))?;

    fs::create_dir_all(output_dir).map_err(|e| {
        format!(
            "cannot create output directory '{}': {e}",
            output_dir.display()
        )
    })?;

    let src_dir = web_dir();
    for name in STATIC_UI_FILES {
        let src = src_dir.join(name);
        let bytes =
            fs::read(&src).map_err(|e| format!("cannot read static UI file '{name}': {e}"))?;
        write_managed_file(output_dir, name, &bytes)?;
    }

    write_managed_file(output_dir, RIGHTS_WORLD_FILE, &compact)?;

    let exported_path = output_dir.join(RIGHTS_WORLD_FILE);
    let read_back = fs::read(&exported_path).map_err(|e| {
        format!(
            "failed to re-read exported '{}': {e}",
            exported_path.display()
        )
    })?;
    if read_back != compact {
        return Err("exported rights-world.json bytes differ from generated compact JSON".into());
    }
    let parsed: RightsWorld = serde_json::from_slice(&read_back)
        .map_err(|e| format!("exported rights-world.json is not valid RightsWorld JSON: {e}"))?;
    if parsed != world {
        return Err("exported rights-world.json does not match generated RightsWorld".into());
    }

    Ok(ExportReport {
        seed,
        rights_bytes: compact.len(),
        files: STATIC_UI_FILES.len() + 1,
    })
}

fn write_managed_file(output_dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    let dest = output_dir.join(name);
    fs::write(&dest, bytes).map_err(|e| format!("cannot write '{}': {e}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use epoch_core::{generate_rights_world, generate_succession_world};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = env::temp_dir().join(format!(
                "epoch-app-{}-{}-{nanos}",
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

    fn assert_static_site(dir: &Path) {
        for name in STATIC_UI_FILES {
            let path = dir.join(name);
            assert!(path.is_file(), "missing {}", path.display());
            assert!(
                fs::metadata(&path).expect("meta").len() > 0,
                "empty {}",
                path.display()
            );
        }
        assert!(dir.join(RIGHTS_WORLD_FILE).is_file());
    }

    #[test]
    fn seed1_export_matches_core_and_keeps_bytes() {
        let tmp = TempDir::new("seed1");
        let report = export_rights_world(1, &tmp.0).expect("export seed 1");
        assert_eq!(report.seed, 1);
        assert_eq!(report.rights_bytes, 66222);
        assert_eq!(report.files, 5);
        assert_static_site(&tmp.0);

        let core = generate_rights_world(1).expect("core");
        let compact = core.to_compact_json_bytes().expect("compact");
        assert_eq!(compact.len(), 66222);
        let exported = fs::read(tmp.0.join(RIGHTS_WORLD_FILE)).expect("read json");
        assert_eq!(exported, compact);
        let parsed: RightsWorld = serde_json::from_slice(&exported).expect("parse");
        assert_eq!(parsed, core);
    }

    #[test]
    fn seed2_export_matches_core_and_keeps_bytes() {
        let tmp = TempDir::new("seed2");
        let report = export_rights_world(2, &tmp.0).expect("export seed 2");
        assert_eq!(report.seed, 2);
        assert_eq!(report.rights_bytes, 66221);
        assert_eq!(report.files, 5);
        assert_static_site(&tmp.0);

        let core = generate_rights_world(2).expect("core");
        let compact = core.to_compact_json_bytes().expect("compact");
        assert_eq!(compact.len(), 66221);
        let exported = fs::read(tmp.0.join(RIGHTS_WORLD_FILE)).expect("read json");
        assert_eq!(exported, compact);
        let parsed: RightsWorld = serde_json::from_slice(&exported).expect("parse");
        assert_eq!(parsed, core);
    }

    #[test]
    fn seed1_succession_export_matches_core_and_keeps_bytes() {
        let tmp = TempDir::new("succession-seed1");
        let report = export_succession_world(1, "realm-01", &tmp.0).expect("export succession");
        assert_eq!(report.seed, 1);
        assert_eq!(report.realm_id, "realm-01");
        assert_eq!(report.rights_bytes, 66222);
        assert_eq!(report.succession_bytes, 71915);
        assert_eq!(report.files, 6);
        assert_static_site(&tmp.0);
        assert!(tmp.0.join(SUCCESSION_WORLD_FILE).is_file());

        let core = generate_succession_world(1, "realm-01").expect("core");
        let compact = core.to_compact_json_bytes().expect("compact");
        assert_eq!(compact.len(), 71915);
        let exported = fs::read(tmp.0.join(SUCCESSION_WORLD_FILE)).expect("read json");
        assert_eq!(exported, compact);
        let parsed: SuccessionWorld = serde_json::from_slice(&exported).expect("parse");
        assert_eq!(parsed, core);

        let rights = generate_rights_world(1).expect("rights");
        let rights_compact = rights.to_compact_json_bytes().expect("rights compact");
        assert_eq!(
            fs::read(tmp.0.join(RIGHTS_WORLD_FILE)).expect("read rights"),
            rights_compact
        );
    }

    #[test]
    fn unwritable_output_path_fails_closed() {
        let tmp = TempDir::new("blocked");
        let blocker = tmp.0.join("not-a-directory");
        fs::write(&blocker, b"x").expect("blocker file");
        let err = export_rights_world(1, &blocker).expect_err("must fail");
        assert!(
            err.contains("cannot create output directory") || err.contains("cannot write"),
            "unexpected error: {err}"
        );
    }
}
