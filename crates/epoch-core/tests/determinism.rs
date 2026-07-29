// 결정론·도메인 결과 통합 테스트

use epoch_core::{
    ContributionClass, DeterministicRng, InformationVisibility, SupportStatus, run_demo,
};
use std::path::PathBuf;
use std::process::Command;

/// 현재 소스에서 빌드된 바이너리만 검사하도록 항상 cargo run을 사용한다.
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
fn same_seed_same_demo_result() {
    let a = run_demo(1).expect("demo a");
    let b = run_demo(1).expect("demo b");
    assert_eq!(a, b);
}

#[test]
fn same_seed_identical_compact_json_bytes() {
    let a = run_demo(1).expect("demo a");
    let b = run_demo(1).expect("demo b");
    let ba = a.to_compact_json_bytes().expect("json a");
    let bb = b.to_compact_json_bytes().expect("json b");
    assert_eq!(ba, bb);
}

#[test]
fn different_seeds_different_rng_draws() {
    let a = run_demo(1).expect("seed1");
    let b = run_demo(2).expect("seed2");
    let draw_a = a
        .events
        .iter()
        .find(|e| e.event_type == "information_resolved")
        .unwrap()
        .random_draws[0]
        .raw_value;
    let draw_b = b
        .events
        .iter()
        .find(|e| e.event_type == "information_resolved")
        .unwrap()
        .random_draws[0]
        .raw_value;
    assert_ne!(draw_a, draw_b);

    let mut r1 = DeterministicRng::new(1);
    let mut r2 = DeterministicRng::new(2);
    assert_ne!(r1.next_u64(), r2.next_u64());
}

#[test]
fn same_command_list_same_event_order() {
    let a = run_demo(7).expect("a");
    let b = run_demo(7).expect("b");
    let types_a: Vec<_> = a.events.iter().map(|e| e.event_type.as_str()).collect();
    let types_b: Vec<_> = b.events.iter().map(|e| e.event_type.as_str()).collect();
    assert_eq!(types_a, types_b);
    assert_eq!(
        types_a,
        vec![
            "player_action_recorded",
            "player_stance_changed",
            "house_support_changed",
            "information_revealed",
            "information_resolved",
        ]
    );
}

#[test]
fn darian_support_drops_from_two_to_one() {
    let result = run_demo(1).expect("demo");
    assert_eq!(
        result
            .initial_state
            .declared_support_count("candidate-darian"),
        2
    );
    assert_eq!(
        result
            .final_state
            .declared_support_count("candidate-darian"),
        1
    );
    let soren = result
        .final_state
        .houses
        .iter()
        .find(|h| h.id == "house-soren")
        .unwrap();
    assert_eq!(soren.support_status, SupportStatus::Undecided);
}

#[test]
fn player_stance_changes() {
    let result = run_demo(1).expect("demo");
    assert_eq!(result.initial_state.player.stance, "house_darian_support");
    assert_eq!(
        result.final_state.player.stance,
        "seria_information_cooperation"
    );
}

#[test]
fn information_reveal_and_resolve() {
    let result = run_demo(1).expect("demo");
    assert_eq!(
        result.initial_state.information[0].visibility,
        InformationVisibility::Private
    );
    let final_vis = result.final_state.information[0].visibility;
    assert!(
        final_vis == InformationVisibility::Unverified
            || final_vis == InformationVisibility::PublicFact
    );
    let resolve = result
        .events
        .iter()
        .find(|e| e.event_type == "information_resolved")
        .unwrap();
    assert_eq!(resolve.random_draws.len(), 1);
    if resolve.random_draws[0].success {
        assert_eq!(final_vis, InformationVisibility::PublicFact);
    } else {
        assert_eq!(final_vis, InformationVisibility::Unverified);
    }
}

#[test]
fn all_caused_by_reference_prior_events() {
    let result = run_demo(1).expect("demo");
    let ids: std::collections::BTreeSet<u64> = result.events.iter().map(|e| e.event_id).collect();
    for e in &result.events {
        if let Some(cause) = e.caused_by {
            assert!(ids.contains(&cause), "missing cause {}", cause);
            assert!(cause < e.event_id, "cause must be prior event");
        }
        for link in &e.influence_links {
            assert!(ids.contains(&link.source_event));
            for c in &link.top_contributors {
                assert!(ids.contains(c));
            }
        }
    }
}

#[test]
fn influence_path_length_matches_chain() {
    let result = run_demo(1).expect("demo");
    let action = result
        .events
        .iter()
        .find(|e| e.event_type == "player_action_recorded")
        .expect("action");
    assert!(action.caused_by.is_none());
    assert!(action.influence_links.is_empty());

    // 즉시 상태 변경: caused_by=행동, direct, path_length 1, 단일 기여자=행동
    for et in [
        "player_stance_changed",
        "house_support_changed",
        "information_revealed",
    ] {
        let e = result
            .events
            .iter()
            .find(|ev| ev.event_type == et)
            .unwrap_or_else(|| panic!("missing {et}"));
        assert_eq!(e.caused_by, Some(action.event_id), "{et} caused_by");
        assert_eq!(e.influence_links.len(), 1, "{et} link count");
        let link = &e.influence_links[0];
        assert_eq!(link.source_event, action.event_id, "{et} source_event");
        assert_eq!(link.path_length, 1, "{et} path_length");
        assert_eq!(
            link.contribution_class,
            ContributionClass::Direct,
            "{et} class"
        );
        assert_eq!(
            link.top_contributors,
            vec![action.event_id],
            "{et} contributors"
        );
    }

    // 정보 확인: caused_by=공개, source_event=행동, mediated, path_length 2, 기여자=[행동, 공개]
    let revealed = result
        .events
        .iter()
        .find(|e| e.event_type == "information_revealed")
        .expect("revealed");
    let resolved = result
        .events
        .iter()
        .find(|e| e.event_type == "information_resolved")
        .expect("resolved");
    assert_eq!(resolved.caused_by, Some(revealed.event_id));
    assert_eq!(resolved.influence_links.len(), 1);
    let link = &resolved.influence_links[0];
    assert_eq!(link.source_event, action.event_id);
    assert_eq!(link.path_length, 2);
    assert_eq!(link.contribution_class, ContributionClass::Mediated);
    assert_eq!(
        link.top_contributors,
        vec![action.event_id, revealed.event_id]
    );
}

#[test]
fn initial_state_preserved_in_result() {
    let result = run_demo(3).expect("demo");
    let expected = epoch_core::WorldState::new_initial(3);
    assert_eq!(result.initial_state, expected);
    assert_ne!(result.final_state.player.stance, expected.player.stance);
}

#[test]
fn demo_completes_without_panic() {
    let _ = run_demo(1).expect("ok");
}

#[test]
fn cli_replay_check_1_prints_determinism_ok() {
    let output = run_epoch_lab(&["replay-check", "1"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("DETERMINISM_OK"), "stdout: {stdout}");
    assert!(stdout.contains("seed=1"));
}

#[test]
fn cli_invalid_args_nonzero_exit() {
    let output = run_epoch_lab(&["demo", "not-a-number"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid seed") || stderr.contains("error"),
        "stderr: {stderr}"
    );
}
