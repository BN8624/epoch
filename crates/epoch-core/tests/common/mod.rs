// 통합 테스트 공용 epoch-lab 실행 헬퍼와 고정 exact 회귀 계약

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};

/// 현재 소스에서 빌드된 바이너리만 검사하도록 항상 cargo run을 사용한다.
pub fn run_epoch_lab(args: &[&str]) -> Output {
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

/// 주문서가 상수로 고정한 M0~M2.1 CLI 출력.
///
/// 부분 문자열 검사는 `claims=120`이 `claims=12`를 통과시키고 `bytes=` 뒤에 숫자가
/// 없어도 통과하므로 사용하지 않는다. 여기 항목은 전부 stdout 전체와 비교한다.
/// 현재 작업 중인 계층의 출력은 해당 계층 테스트 파일에서 따로 검사한다.
pub const CLI_EXACT_REGRESSION: &[([&str; 2], &str)] = &[
    (
        ["world-check", "1"],
        "WORLD_OK seed=1 realms=6 territories=36 rulers=6 template=vertical bytes=6234",
    ),
    (
        ["world-check", "2"],
        "WORLD_OK seed=2 realms=6 territories=36 rulers=6 template=blocks_2x3 bytes=6233",
    ),
    (
        ["population-check", "1"],
        "POPULATION_OK seed=1 houses=18 persons=144 elder=36 current=54 young=54 rulers=6 bytes=34960",
    ),
    (
        ["population-check", "2"],
        "POPULATION_OK seed=2 houses=18 persons=144 elder=36 current=54 young=54 rulers=6 bytes=34959",
    ),
    (
        ["actors-check", "1"],
        "ACTORS_OK seed=1 active=24 supporting=120 rulers=6 house_heads=12 ruling_house_current=6 realms=6 bytes=39466",
    ),
    (
        ["actors-check", "2"],
        "ACTORS_OK seed=2 active=24 supporting=120 rulers=6 house_heads=12 ruling_house_current=6 realms=6 bytes=39465",
    ),
    (
        ["context-check", "1"],
        "CONTEXT_OK seed=1 cultures=3 religions=2 realm_profiles=6 house_profiles=18 person_profiles=144 relations=24 promises=12 information=18 bytes=61898",
    ),
    (
        ["context-check", "2"],
        "CONTEXT_OK seed=2 cultures=3 religions=2 realm_profiles=6 house_profiles=18 person_profiles=144 relations=24 promises=12 information=18 bytes=61897",
    ),
    (
        ["rights-check", "1"],
        "RIGHTS_OK seed=1 realms=6 claims=12 direct=6 restored=6 strong=6 contested=6 evidence=6 bytes=66222",
    ),
    (
        ["rights-check", "2"],
        "RIGHTS_OK seed=2 realms=6 claims=12 direct=6 restored=6 strong=6 contested=6 evidence=6 bytes=66221",
    ),
    (
        ["family-check", "1"],
        "FAMILY_OK seed=1 marriages=12 parentages=12 interfaith=6 intercultural=6 dual_parent_children=12 bytes=69415",
    ),
    (
        ["family-check", "2"],
        "FAMILY_OK seed=2 marriages=12 parentages=12 interfaith=6 intercultural=6 dual_parent_children=12 bytes=69414",
    ),
    (
        ["replay-check", "1"],
        "DETERMINISM_OK seed=1 events=5 bytes=7353",
    ),
    (
        ["replay-check", "2"],
        "DETERMINISM_OK seed=2 events=5 bytes=7392",
    ),
    (
        ["save-check", "1"],
        "SAVE_LOAD_OK seed=1 checkpoint_bytes=2167 events=5",
    ),
    (
        ["save-check", "2"],
        "SAVE_LOAD_OK seed=2 checkpoint_bytes=2167 events=5",
    ),
];

/// 고정된 exact 회귀 표에서 한 명령의 기대 출력을 찾는다.
pub fn expected_cli_output(args: [&str; 2]) -> &'static str {
    CLI_EXACT_REGRESSION
        .iter()
        .find(|(candidate, _)| *candidate == args)
        .map(|(_, expected)| *expected)
        .unwrap_or_else(|| panic!("no fixed expectation for {args:?}"))
}

/// epoch-lab 명령을 실행하고 stdout 전체를 기대 문자열과 비교한다.
pub fn assert_cli_exact(args: &[&str], expected: &str) {
    let output = run_epoch_lab(args);
    assert!(
        output.status.success(),
        "{args:?} stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        expected,
        "{args:?}"
    );
}

/// 고정된 M0~M2.1 exact 회귀 전체를 검사한다.
///
/// 마일스톤마다 이 배터리를 새 파일에 다시 작성하지 않는다. 최상위 계층 테스트
/// 하나에서만 호출해 같은 CLI를 여러 번 실행하지 않도록 한다.
pub fn assert_cli_exact_regression() {
    for (args, expected) in CLI_EXACT_REGRESSION {
        assert_cli_exact(args, expected);
    }
}

/// JSON 덤프 명령의 stdout을 역직렬화해 기대 구조와 비교한다.
///
/// 키 이름 부분 문자열만 확인하면 깨진 JSON도 통과하므로 실제로 파싱한다.
pub fn assert_cli_json_eq<T>(args: &[&str], expected: &T)
where
    T: serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let output = run_epoch_lab(args);
    assert!(
        output.status.success(),
        "{args:?} stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout utf-8");
    let parsed: T = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("{args:?} stdout must be valid JSON: {e}"));
    assert_eq!(&parsed, expected, "{args:?}");
}
