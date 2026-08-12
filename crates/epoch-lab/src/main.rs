// EPOCH lab CLI — demo 및 결정론 재생·저장 검사

use epoch_core::{
    create_demo_checkpoint, generate_world, load_runtime_from_bytes, run_demo, run_demo_to_runtime,
    save_runtime_to_bytes, validate_world,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{self, ExitCode};

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
        "demo" => cmd_demo(&args),
        "replay-check" => cmd_replay_check(&args),
        "save-check" => cmd_save_check(&args),
        "world" => cmd_world(&args),
        "world-check" => cmd_world_check(&args),
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
epoch-lab — EPOCH deterministic core lab CLI

Usage:
  cargo run -p epoch-lab -- help
  cargo run -p epoch-lab -- demo <seed>
  cargo run -p epoch-lab -- replay-check <seed>
  cargo run -p epoch-lab -- save-check <seed>
  cargo run -p epoch-lab -- world <seed>
  cargo run -p epoch-lab -- world-check <seed>

Commands:
  help           Show this help
  demo           Run fixed succession demo and print pretty JSON
  replay-check   Run demo twice and verify byte-identical compact JSON
  save-check     Checkpoint mid-run, save/load via temp file, resume vs baseline
  world          Generate world skeleton and print pretty JSON
  world-check    Generate twice, verify determinism and world invariants
"
    );
}

fn print_usage_stderr() {
    eprintln!(
        "\
Usage:
  cargo run -p epoch-lab -- help
  cargo run -p epoch-lab -- demo <seed>
  cargo run -p epoch-lab -- replay-check <seed>
  cargo run -p epoch-lab -- save-check <seed>
  cargo run -p epoch-lab -- world <seed>
  cargo run -p epoch-lab -- world-check <seed>
"
    );
}

fn parse_seed(args: &[String]) -> Result<u64, String> {
    match args {
        [] => Err("missing seed argument".to_string()),
        [s] => s
            .parse::<u64>()
            .map_err(|_| format!("invalid seed '{s}': expected unsigned 64-bit integer")),
        _ => Err("too many arguments: expected a single seed".to_string()),
    }
}

fn cmd_world(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_world_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: second generate failed: {e}");
            return ExitCode::from(1);
        }
    };

    if a != b {
        eprintln!("error: structure inequality for seed={seed}");
        return ExitCode::from(1);
    }

    let bytes_a = match a.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize first world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second world: {e}");
            return ExitCode::from(1);
        }
    };
    if bytes_a != bytes_b {
        eprintln!(
            "error: compact JSON bytes differ for seed={seed} len_a={} len_b={}",
            bytes_a.len(),
            bytes_b.len()
        );
        return ExitCode::from(1);
    }

    if let Err(e) = validate_world(&a) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }

    println!(
        "WORLD_OK seed={seed} realms={} territories={} rulers={} template={} bytes={}",
        a.realms.len(),
        a.territories.len(),
        a.rulers.len(),
        a.generation.template_id,
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_demo(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match run_demo(seed) {
        Ok(result) => match result.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize demo result: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: demo failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_replay_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match run_demo(seed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: first run failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match run_demo(seed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: second run failed: {e}");
            return ExitCode::from(1);
        }
    };

    let bytes_a = match a.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize first result: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second result: {e}");
            return ExitCode::from(1);
        }
    };

    if bytes_a == bytes_b {
        let events = a.events.len();
        let nbytes = bytes_a.len();
        println!("DETERMINISM_OK seed={seed} events={events} bytes={nbytes}");
        ExitCode::SUCCESS
    } else {
        let mismatch = first_mismatch(&bytes_a, &bytes_b);
        eprintln!(
            "DETERMINISM_FAIL seed={seed} len_a={} len_b={} first_mismatch={mismatch}",
            bytes_a.len(),
            bytes_b.len()
        );
        ExitCode::from(1)
    }
}

fn cmd_save_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    // 1. uninterrupted baseline
    let baseline = match run_demo_to_runtime(seed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: baseline run failed: {e}");
            return ExitCode::from(1);
        }
    };

    // 2–3. checkpoint + save bytes
    let checkpoint = match create_demo_checkpoint(seed) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: checkpoint creation failed: {e}");
            return ExitCode::from(1);
        }
    };
    let checkpoint_bytes = match save_runtime_to_bytes(&checkpoint) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: save checkpoint failed: {e}");
            return ExitCode::from(1);
        }
    };

    // 4–5. 실제 임시 파일에 기록 후 재읽기
    let tmp_path = temp_save_path(seed);
    if let Err(e) = fs::write(&tmp_path, &checkpoint_bytes) {
        // 부분 기록·생성 파일이 남을 수 있으므로 최선 노력으로 정리한다.
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: write temp save failed: {e}");
        return ExitCode::from(1);
    }
    let file_bytes = match fs::read(&tmp_path) {
        Ok(b) => b,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: read temp save failed: {e}");
            return ExitCode::from(1);
        }
    };
    if file_bytes != checkpoint_bytes {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: temp file bytes differ from in-memory checkpoint");
        return ExitCode::from(1);
    }

    // 6. load
    let mut resumed = match load_runtime_from_bytes(&file_bytes) {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: load failed: {e}");
            return ExitCode::from(1);
        }
    };

    // 7. resume
    if let Err(e) = resumed.run_until_idle() {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: resume execution failed: {e}");
        return ExitCode::from(1);
    }

    // 8. baseline 비교 (전체 구조)
    if baseline.world != resumed.world {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: final WorldState mismatch after resume");
        return ExitCode::from(1);
    }
    if baseline.world.events != resumed.world.events {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: final events mismatch after resume");
        return ExitCode::from(1);
    }
    if baseline.world.rng != resumed.world.rng {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: final RNG mismatch after resume");
        return ExitCode::from(1);
    }
    if baseline.scheduler != resumed.scheduler {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: final scheduler state mismatch after resume");
        return ExitCode::from(1);
    }
    let world_json_a = match serde_json::to_vec(&baseline.world) {
        Ok(b) => b,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: serialize baseline world: {e}");
            return ExitCode::from(1);
        }
    };
    let world_json_b = match serde_json::to_vec(&resumed.world) {
        Ok(b) => b,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: serialize resumed world: {e}");
            return ExitCode::from(1);
        }
    };
    if world_json_a != world_json_b {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: final compact world JSON mismatch after resume");
        return ExitCode::from(1);
    }

    // 9. load 뒤 다시 저장한 bytes vs 원본 checkpoint
    let reloaded = match load_runtime_from_bytes(&checkpoint_bytes) {
        Ok(r) => r,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: reload checkpoint for round-trip failed: {e}");
            return ExitCode::from(1);
        }
    };
    let resaved = match save_runtime_to_bytes(&reloaded) {
        Ok(b) => b,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            eprintln!("error: re-save after load failed: {e}");
            return ExitCode::from(1);
        }
    };
    if resaved != checkpoint_bytes {
        let _ = fs::remove_file(&tmp_path);
        eprintln!("error: save → load → save bytes differ from original checkpoint");
        return ExitCode::from(1);
    }

    // 10. 임시 파일 삭제
    if let Err(e) = fs::remove_file(&tmp_path) {
        eprintln!("error: failed to remove temp save: {e}");
        return ExitCode::from(1);
    }

    let events = resumed.world.events.len();
    let nbytes = checkpoint_bytes.len();
    println!("SAVE_LOAD_OK seed={seed} checkpoint_bytes={nbytes} events={events}");
    ExitCode::SUCCESS
}

fn temp_save_path(seed: u64) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!("epoch-save-check-{}-{seed}.json", process::id()));
    path
}

fn first_mismatch(a: &[u8], b: &[u8]) -> String {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i] != b[i] {
            return format!("offset {i}: {} vs {}", a[i], b[i]);
        }
    }
    if a.len() != b.len() {
        format!("length {} vs {}", a.len(), b.len())
    } else {
        "none".to_string()
    }
}
