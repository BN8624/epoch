// EPOCH lab CLI — demo 및 결정론 재생·저장 검사

use epoch_core::{
    ActiveRole, ClaimBasis, ClaimStanding, GenerationBand, SuccessionPriority,
    create_demo_checkpoint, generate_claim_propagation_world, generate_context_world,
    generate_dynastic_world, generate_family_world, generate_political_world,
    generate_rights_world, generate_succession_world, generate_world, load_runtime_from_bytes,
    run_demo, run_demo_to_runtime, save_runtime_to_bytes, validate_initial_claim_propagation,
    validate_initial_context, validate_initial_family, validate_initial_rights,
    validate_political_roster, validate_population, validate_succession_transition, validate_world,
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
        "population" => cmd_population(&args),
        "population-check" => cmd_population_check(&args),
        "actors" => cmd_actors(&args),
        "actors-check" => cmd_actors_check(&args),
        "context" => cmd_context(&args),
        "context-check" => cmd_context_check(&args),
        "rights" => cmd_rights(&args),
        "rights-check" => cmd_rights_check(&args),
        "family" => cmd_family(&args),
        "family-check" => cmd_family_check(&args),
        "claim-propagation" => cmd_claim_propagation(&args),
        "claim-propagation-check" => cmd_claim_propagation_check(&args),
        "succession" => cmd_succession(&args),
        "succession-check" => cmd_succession_check(&args),
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
  cargo run -p epoch-lab -- population <seed>
  cargo run -p epoch-lab -- population-check <seed>
  cargo run -p epoch-lab -- actors <seed>
  cargo run -p epoch-lab -- actors-check <seed>
  cargo run -p epoch-lab -- context <seed>
  cargo run -p epoch-lab -- context-check <seed>
  cargo run -p epoch-lab -- rights <seed>
  cargo run -p epoch-lab -- rights-check <seed>
  cargo run -p epoch-lab -- family <seed>
  cargo run -p epoch-lab -- family-check <seed>
  cargo run -p epoch-lab -- claim-propagation <seed>
  cargo run -p epoch-lab -- claim-propagation-check <seed>
  cargo run -p epoch-lab -- succession <seed> <realm-id>
  cargo run -p epoch-lab -- succession-check <seed>

Commands:
  help              Show this help
  demo              Run fixed succession demo and print pretty JSON
  replay-check      Run demo twice and verify byte-identical compact JSON
  save-check        Checkpoint mid-run, save/load via temp file, resume vs baseline
  world             Generate world skeleton and print pretty JSON
  world-check       Generate twice, verify determinism and world invariants
  population        Generate dynastic world (world + population) and print pretty JSON
  population-check  Generate twice, verify determinism and population invariants
  actors            Generate political world and print pretty JSON
  actors-check      Generate twice, verify determinism and political roster invariants
  context           Generate context world and print pretty JSON
  context-check     Generate twice, verify determinism and context invariants
  rights            Generate rights world and print pretty JSON
  rights-check      Generate twice, verify determinism and rights invariants
  family            Generate family world and print pretty JSON
  family-check      Generate twice, verify determinism and family invariants
  claim-propagation Generate claim-propagation world and print pretty JSON
  claim-propagation-check  Generate twice, verify determinism and derived-claim invariants
  succession        Generate succession world for one realm and print pretty JSON
  succession-check  Independently verify all six realms for one seed
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
  cargo run -p epoch-lab -- population <seed>
  cargo run -p epoch-lab -- population-check <seed>
  cargo run -p epoch-lab -- actors <seed>
  cargo run -p epoch-lab -- actors-check <seed>
  cargo run -p epoch-lab -- context <seed>
  cargo run -p epoch-lab -- context-check <seed>
  cargo run -p epoch-lab -- rights <seed>
  cargo run -p epoch-lab -- rights-check <seed>
  cargo run -p epoch-lab -- family <seed>
  cargo run -p epoch-lab -- family-check <seed>
  cargo run -p epoch-lab -- claim-propagation <seed>
  cargo run -p epoch-lab -- claim-propagation-check <seed>
  cargo run -p epoch-lab -- succession <seed> <realm-id>
  cargo run -p epoch-lab -- succession-check <seed>
"
    );
}

fn parse_seed_and_realm(args: &[String]) -> Result<(u64, String), String> {
    match args {
        [] => Err("missing seed and realm id".to_string()),
        [_] => Err("missing realm id".to_string()),
        [seed, realm] => {
            let seed = seed
                .parse::<u64>()
                .map_err(|_| format!("invalid seed '{seed}': expected unsigned 64-bit integer"))?;
            if realm.is_empty() {
                return Err("realm id must not be empty".to_string());
            }
            Ok((seed, realm.clone()))
        }
        _ => Err("too many arguments: expected succession <seed> <realm-id>".to_string()),
    }
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

fn cmd_population(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_dynastic_world(seed) {
        Ok(dynastic) => match dynastic.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize dynastic world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: dynastic world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_population_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_dynastic_world(seed) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_dynastic_world(seed) {
        Ok(d) => d,
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
            eprintln!("error: serialize first dynastic world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second dynastic world: {e}");
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

    if let Err(e) = validate_world(&a.world) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_population(&a.world, &a.population) {
        eprintln!("error: population invariants failed: {e}");
        return ExitCode::from(1);
    }

    let mut elder = 0usize;
    let mut current = 0usize;
    let mut young = 0usize;
    for p in &a.population.persons {
        match p.generation {
            GenerationBand::Elder => elder += 1,
            GenerationBand::Current => current += 1,
            GenerationBand::Young => young += 1,
        }
    }

    println!(
        "POPULATION_OK seed={seed} houses={} persons={} elder={elder} current={current} young={young} rulers={} bytes={}",
        a.population.houses.len(),
        a.population.persons.len(),
        a.population.ruler_links.len(),
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_actors(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_political_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize political world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: political world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_actors_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_political_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_political_world(seed) {
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
            eprintln!("error: serialize first political world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second political world: {e}");
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

    if let Err(e) = validate_world(&a.dynastic.world) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_population(&a.dynastic.world, &a.dynastic.population) {
        eprintln!("error: population invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_political_roster(&a.dynastic, &a.roster) {
        eprintln!("error: political roster invariants failed: {e}");
        return ExitCode::from(1);
    }

    let mut rulers = 0usize;
    let mut house_heads = 0usize;
    let mut ruling_house_current = 0usize;
    for actor in &a.roster.active_actors {
        match actor.primary_role {
            ActiveRole::Ruler => rulers += 1,
            ActiveRole::HouseHead => house_heads += 1,
            ActiveRole::RulingHouseCurrent => ruling_house_current += 1,
        }
    }

    println!(
        "ACTORS_OK seed={seed} active={} supporting={} rulers={rulers} house_heads={house_heads} ruling_house_current={ruling_house_current} realms={} bytes={}",
        a.roster.active_actors.len(),
        a.roster.supporting_person_ids.len(),
        a.dynastic.world.realms.len(),
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_context(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_context_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize context world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: context world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_context_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_context_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_context_world(seed) {
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
            eprintln!("error: serialize first context world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second context world: {e}");
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

    if let Err(e) = validate_world(&a.political.dynastic.world) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_population(
        &a.political.dynastic.world,
        &a.political.dynastic.population,
    ) {
        eprintln!("error: population invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_political_roster(&a.political.dynastic, &a.political.roster) {
        eprintln!("error: political roster invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_context(&a.political, &a.context) {
        eprintln!("error: context invariants failed: {e}");
        return ExitCode::from(1);
    }

    println!(
        "CONTEXT_OK seed={seed} cultures={} religions={} realm_profiles={} house_profiles={} person_profiles={} relations={} promises={} information={} bytes={}",
        a.context.cultures.len(),
        a.context.religions.len(),
        a.context.realm_identities.len(),
        a.context.house_identities.len(),
        a.context.person_identities.len(),
        a.context.relations.len(),
        a.context.promises.len(),
        a.context.information.len(),
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_rights(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_rights_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize rights world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: rights world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_rights_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_rights_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_rights_world(seed) {
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
            eprintln!("error: serialize first rights world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second rights world: {e}");
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

    if let Err(e) = validate_world(&a.context_world.political.dynastic.world) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_population(
        &a.context_world.political.dynastic.world,
        &a.context_world.political.dynastic.population,
    ) {
        eprintln!("error: population invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_political_roster(
        &a.context_world.political.dynastic,
        &a.context_world.political.roster,
    ) {
        eprintln!("error: political roster invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_context(&a.context_world.political, &a.context_world.context) {
        eprintln!("error: context invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_rights(&a.context_world, &a.rights) {
        eprintln!("error: rights invariants failed: {e}");
        return ExitCode::from(1);
    }

    let mut direct = 0usize;
    let mut restored = 0usize;
    let mut strong = 0usize;
    let mut contested = 0usize;
    for claim in &a.rights.claims {
        match claim.basis {
            ClaimBasis::DirectDescent => direct += 1,
            ClaimBasis::RestoredLineRecord => restored += 1,
        }
        match claim.standing {
            ClaimStanding::Strong => strong += 1,
            ClaimStanding::Contested => contested += 1,
        }
    }

    println!(
        "RIGHTS_OK seed={seed} realms={} claims={} direct={direct} restored={restored} strong={strong} contested={contested} evidence={} bytes={}",
        a.rights.realms.len(),
        a.rights.claims.len(),
        a.rights.evidence_records.len(),
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_family(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_family_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize family world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: family world generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_family_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_family_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_family_world(seed) {
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
            eprintln!("error: serialize first family world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second family world: {e}");
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

    if let Err(e) = validate_world(&a.rights_world.context_world.political.dynastic.world) {
        eprintln!("error: world invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_population(
        &a.rights_world.context_world.political.dynastic.world,
        &a.rights_world.context_world.political.dynastic.population,
    ) {
        eprintln!("error: population invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_political_roster(
        &a.rights_world.context_world.political.dynastic,
        &a.rights_world.context_world.political.roster,
    ) {
        eprintln!("error: political roster invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_context(
        &a.rights_world.context_world.political,
        &a.rights_world.context_world.context,
    ) {
        eprintln!("error: context invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_rights(&a.rights_world.context_world, &a.rights_world.rights) {
        eprintln!("error: rights invariants failed: {e}");
        return ExitCode::from(1);
    }
    if let Err(e) = validate_initial_family(&a.rights_world, &a.family) {
        eprintln!("error: family invariants failed: {e}");
        return ExitCode::from(1);
    }

    let identities: std::collections::BTreeMap<&str, &epoch_core::PersonIdentity> = a
        .rights_world
        .context_world
        .context
        .person_identities
        .iter()
        .map(|p| (p.person_id.as_str(), p))
        .collect();
    let mut interfaith = 0usize;
    let mut intercultural = 0usize;
    for marriage in &a.family.marriages {
        let left_id = match marriage.spouse_person_ids.first() {
            Some(id) => id.as_str(),
            None => {
                eprintln!("error: marriage {} missing spouse 0", marriage.id);
                return ExitCode::from(1);
            }
        };
        let right_id = match marriage.spouse_person_ids.get(1) {
            Some(id) => id.as_str(),
            None => {
                eprintln!("error: marriage {} missing spouse 1", marriage.id);
                return ExitCode::from(1);
            }
        };
        let left = match identities.get(left_id) {
            Some(id) => *id,
            None => {
                eprintln!("error: missing identity {left_id}");
                return ExitCode::from(1);
            }
        };
        let right = match identities.get(right_id) {
            Some(id) => *id,
            None => {
                eprintln!("error: missing identity {right_id}");
                return ExitCode::from(1);
            }
        };
        if left.culture_id == right.culture_id && left.religion_id != right.religion_id {
            interfaith += 1;
        } else if left.culture_id != right.culture_id && left.religion_id == right.religion_id {
            intercultural += 1;
        }
    }
    let dual_parent_children = a.family.parentages.len();

    println!(
        "FAMILY_OK seed={seed} marriages={} parentages={} interfaith={interfaith} intercultural={intercultural} dual_parent_children={dual_parent_children} bytes={}",
        a.family.marriages.len(),
        a.family.parentages.len(),
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_claim_propagation(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_claim_propagation_world(seed) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize claim propagation world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: claim propagation generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_claim_propagation_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let a = match generate_claim_propagation_world(seed) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let b = match generate_claim_propagation_world(seed) {
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
            eprintln!("error: serialize first claim propagation world: {e}");
            return ExitCode::from(1);
        }
    };
    let bytes_b = match b.to_compact_json_bytes() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: serialize second claim propagation world: {e}");
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

    if let Err(e) = validate_initial_claim_propagation(&a.family_world, &a.propagation) {
        eprintln!("error: claim propagation invariants failed: {e}");
        return ExitCode::from(1);
    }

    let original = a.family_world.rights_world.rights.claims.len();
    let derived = a.propagation.derived_claims.len();
    let claim_by_id: std::collections::BTreeMap<&str, &epoch_core::SuccessionClaim> = a
        .family_world
        .rights_world
        .rights
        .claims
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();
    let supporting: std::collections::BTreeSet<&str> = a
        .family_world
        .rights_world
        .context_world
        .political
        .roster
        .supporting_person_ids
        .iter()
        .map(|s| s.as_str())
        .collect();
    let mut restored_sources = 0usize;
    let mut direct_sources = 0usize;
    let mut distance1 = 0usize;
    let mut derived_supporting = 0usize;
    for derived_claim in &a.propagation.derived_claims {
        let source = match claim_by_id.get(derived_claim.source_claim_id.as_str()) {
            Some(claim) => *claim,
            None => {
                eprintln!(
                    "error: unknown source claim {}",
                    derived_claim.source_claim_id
                );
                return ExitCode::from(1);
            }
        };
        match source.basis {
            ClaimBasis::RestoredLineRecord => restored_sources += 1,
            ClaimBasis::DirectDescent => direct_sources += 1,
        }
        if derived_claim.generation_distance == 1 {
            distance1 += 1;
        }
        if supporting.contains(derived_claim.claimant_person_id.as_str()) {
            derived_supporting += 1;
        }
    }

    println!(
        "CLAIM_PROPAGATION_OK seed={seed} original={original} derived={derived} restored_sources={restored_sources} direct_sources={direct_sources} distance1={distance1} derived_supporting={derived_supporting} bytes={}",
        bytes_a.len()
    );
    ExitCode::SUCCESS
}

fn cmd_succession(args: &[String]) -> ExitCode {
    let (seed, realm_id) = match parse_seed_and_realm(args) {
        Ok(v) => v,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    match generate_succession_world(seed, &realm_id) {
        Ok(world) => match world.to_pretty_json() {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: failed to serialize succession world: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("error: succession generation failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_succession_check(args: &[String]) -> ExitCode {
    let seed = match parse_seed(args) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("error: {msg}");
            print_usage_stderr();
            return ExitCode::from(2);
        }
    };

    let probe = match generate_succession_world(seed, "realm-01") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: first generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let realm_ids: Vec<String> = probe
        .pre_succession_world
        .family_world
        .rights_world
        .rights
        .realms
        .iter()
        .map(|r| r.realm_id.clone())
        .collect();
    if realm_ids.len() != 6 {
        eprintln!("error: expected 6 realms, got {}", realm_ids.len());
        return ExitCode::from(1);
    }

    let mut realms = 0usize;
    let mut candidates = 0usize;
    let mut direct_winners = 0usize;
    let mut restored_winners = 0usize;
    let mut derived_winners = 0usize;
    let mut vacancies = 0usize;

    for realm_id in &realm_ids {
        let a = match generate_succession_world(seed, realm_id) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("error: first generate failed for {realm_id}: {e}");
                return ExitCode::from(1);
            }
        };
        let b = match generate_succession_world(seed, realm_id) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("error: second generate failed for {realm_id}: {e}");
                return ExitCode::from(1);
            }
        };
        if a != b {
            eprintln!("error: structure inequality for seed={seed} realm={realm_id}");
            return ExitCode::from(1);
        }
        let bytes_a = match a.to_compact_json_bytes() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: serialize first succession world: {e}");
                return ExitCode::from(1);
            }
        };
        let bytes_b = match b.to_compact_json_bytes() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: serialize second succession world: {e}");
                return ExitCode::from(1);
            }
        };
        if bytes_a != bytes_b {
            eprintln!(
                "error: compact JSON bytes differ for seed={seed} realm={realm_id} len_a={} len_b={}",
                bytes_a.len(),
                bytes_b.len()
            );
            return ExitCode::from(1);
        }
        if let Err(e) = validate_succession_transition(&a.pre_succession_world, &a.transition) {
            eprintln!("error: succession invariants failed for {realm_id}: {e}");
            return ExitCode::from(1);
        }
        if a.seed != seed {
            eprintln!("error: seed {} != requested {seed}", a.seed);
            return ExitCode::from(1);
        }
        if a.transition.realm_id != *realm_id {
            eprintln!(
                "error: transition realm {} != {realm_id}",
                a.transition.realm_id
            );
            return ExitCode::from(1);
        }
        realms += 1;
        candidates += a.transition.candidates.len();
        if a.transition.vacancy.is_vacant {
            vacancies += 1;
        }
        let winner = match a
            .transition
            .candidates
            .iter()
            .find(|c| c.person_id == a.transition.presumptive_successor_person_id)
        {
            Some(c) => c,
            None => {
                eprintln!("error: successor is not a candidate for {realm_id}");
                return ExitCode::from(1);
            }
        };
        match winner.priority {
            SuccessionPriority::DirectStrongOriginal => direct_winners += 1,
            SuccessionPriority::RestoredContestedOriginal => restored_winners += 1,
            SuccessionPriority::RestoredContestedDerived => derived_winners += 1,
        }
    }

    let realm01 = match generate_succession_world(seed, "realm-01") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: realm-01 generate failed: {e}");
            return ExitCode::from(1);
        }
    };
    let realm01_bytes = match realm01.to_compact_json_bytes() {
        Ok(b) => b.len(),
        Err(e) => {
            eprintln!("error: serialize realm-01 succession world: {e}");
            return ExitCode::from(1);
        }
    };

    println!(
        "SUCCESSION_OK seed={seed} realms={realms} candidates={candidates} direct_winners={direct_winners} restored_winners={restored_winners} derived_winners={derived_winners} vacancies={vacancies} realm01_bytes={realm01_bytes}"
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
