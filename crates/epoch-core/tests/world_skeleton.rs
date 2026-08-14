// M1.1 세계 골격 CLI·결정론 통합 테스트

use epoch_core::{generate_world, validate_world};

mod common;

// world-check 1/2의 exact 회귀는 common::CLI_EXACT_REGRESSION이 담당한다.

#[test]
fn cli_world_1_succeeds() {
    common::assert_cli_json_eq(&["world", "1"], &generate_world(1).expect("world 1"));
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
