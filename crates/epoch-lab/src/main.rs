// EPOCH lab CLI — demo 및 결정론 재생 검사

use epoch_core::run_demo;
use std::env;
use std::process::ExitCode;

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

Commands:
  help           Show this help
  demo           Run fixed succession demo and print pretty JSON
  replay-check   Run demo twice and verify byte-identical compact JSON
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
