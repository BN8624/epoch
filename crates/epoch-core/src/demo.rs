// 고정 계승 분쟁 demo 실행

use crate::command::{
    Command, CommandEnvelope, EXPOSE_DUPLICATE_PROMISE, SubmitSpec, submit_command,
};
use crate::error::CoreError;
use crate::event::Event;
use crate::model::WorldState;
use crate::runtime::RuntimeState;
use crate::scheduler::{Phase, Scheduler};
use serde::{Deserialize, Serialize};

/// demo 실행 정규 결과.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemoResult {
    pub schema_version: u32,
    pub seed: u64,
    pub initial_state: WorldState,
    pub submitted_commands: Vec<CommandEnvelope>,
    pub final_state: WorldState,
    pub events: Vec<Event>,
}

impl DemoResult {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

/// 고정 demo 루트 명령이 등록된 RuntimeState를 만든다 (실행 전).
pub fn create_demo_runtime(seed: u64) -> Result<RuntimeState, CoreError> {
    let mut world = WorldState::new_initial(seed);
    let mut scheduler = Scheduler::new();
    let player_id = world.player.id.clone();
    submit_command(
        &mut world,
        &mut scheduler,
        SubmitSpec {
            time: 100,
            phase: Phase::ActionExecution,
            priority: 0,
            actor_id: player_id,
            issued_by: "demo".to_string(),
            caused_by_event: None,
            command: Command::RecordPlayerAction {
                action_code: EXPOSE_DUPLICATE_PROMISE.to_string(),
            },
            command_id: "cmd-demo-record-player-action".to_string(),
        },
    )?;
    Ok(RuntimeState::new(world, scheduler))
}

/// 시간 100, RecordPlayerAction 완료 직후 checkpoint.
/// pending 후속 명령 3개, RNG draws == 0, stance/가문/정보 변경 전.
pub fn create_demo_checkpoint(seed: u64) -> Result<RuntimeState, CoreError> {
    let mut runtime = create_demo_runtime(seed)?;
    let stepped = runtime.step()?.ok_or_else(|| {
        CoreError::InvalidSaveInvariant("demo checkpoint expected a root command".into())
    })?;
    debug_assert_eq!(
        stepped.command_id, "cmd-demo-record-player-action",
        "checkpoint must follow RecordPlayerAction"
    );
    Ok(runtime)
}

/// 고정 demo: 시간 100에서 중복 약속 공개 행동을 실행한다.
pub fn run_demo(seed: u64) -> Result<DemoResult, CoreError> {
    let initial_state = WorldState::new_initial(seed);
    let mut runtime = create_demo_runtime(seed)?;

    // 제출 시점 스냅샷 (실행 전 큐 내용)
    let submitted_commands: Vec<CommandEnvelope> = runtime.scheduler.pending().to_vec();

    runtime.run_until_idle()?;

    let events = runtime.world.events.clone();
    Ok(DemoResult {
        schema_version: 1,
        seed,
        initial_state,
        submitted_commands,
        final_state: runtime.world,
        events,
    })
}

/// uninterrupted 경로 최종 RuntimeState (큐 비움).
pub fn run_demo_to_runtime(seed: u64) -> Result<RuntimeState, CoreError> {
    let mut runtime = create_demo_runtime(seed)?;
    runtime.run_until_idle()?;
    Ok(runtime)
}

/// checkpoint 저장 → 로드 → 재개 경로의 최종 RuntimeState.
pub fn run_demo_via_checkpoint(seed: u64) -> Result<(RuntimeState, Vec<u8>), CoreError> {
    let checkpoint = create_demo_checkpoint(seed)?;
    let bytes = crate::save::save_runtime_to_bytes(&checkpoint)?;
    let mut resumed = crate::save::load_runtime_from_bytes(&bytes)?;
    resumed.run_until_idle()?;
    Ok((resumed, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ContributionClass;
    use crate::model::{InformationVisibility, SupportStatus};
    use crate::save::{load_runtime_from_bytes, save_runtime_to_bytes};

    #[test]
    fn demo_changes_stance_and_soren_support() {
        let result = run_demo(1).expect("demo");
        assert_eq!(
            result.final_state.player.stance,
            "seria_information_cooperation"
        );
        let soren = result
            .final_state
            .houses
            .iter()
            .find(|h| h.id == "house-soren")
            .unwrap();
        assert_eq!(soren.support_status, SupportStatus::Undecided);
        assert_eq!(soren.supported_candidate, None);
        assert_eq!(
            result
                .final_state
                .declared_support_count("candidate-darian"),
            1
        );
        assert_eq!(
            result
                .initial_state
                .declared_support_count("candidate-darian"),
            2
        );
    }

    #[test]
    fn demo_reveals_information_and_resolves() {
        let result = run_demo(1).expect("demo");
        let info = result
            .final_state
            .information
            .iter()
            .find(|i| i.id == "info-darian-duplicate-promise")
            .unwrap();
        assert!(
            info.visibility == InformationVisibility::Unverified
                || info.visibility == InformationVisibility::PublicFact
        );
        assert!(
            result
                .events
                .iter()
                .any(|e| e.event_type == "information_revealed")
        );
        assert!(
            result
                .events
                .iter()
                .any(|e| e.event_type == "information_resolved")
        );
    }

    #[test]
    fn event_ids_monotone() {
        let result = run_demo(1).expect("demo");
        let mut prev = 0u64;
        for e in &result.events {
            assert!(e.event_id > prev);
            prev = e.event_id;
        }
        assert_eq!(result.events[0].event_id, 1);
    }

    #[test]
    fn state_change_before_after_accurate() {
        let result = run_demo(1).expect("demo");
        let stance_ev = result
            .events
            .iter()
            .find(|e| e.event_type == "player_stance_changed")
            .unwrap();
        let sc = &stance_ev.state_changes[0];
        assert_eq!(sc.field, "stance");
        assert_eq!(sc.before, "house_darian_support");
        assert_eq!(sc.after, "seria_information_cooperation");
    }

    #[test]
    fn random_draw_recorded_on_resolve() {
        let result = run_demo(1).expect("demo");
        let resolve = result
            .events
            .iter()
            .find(|e| e.event_type == "information_resolved")
            .unwrap();
        assert_eq!(resolve.random_draws.len(), 1);
        let d = &resolve.random_draws[0];
        assert_eq!(d.chance_basis_points, 5000);
        assert_eq!(d.draw_index, 1);
        assert_eq!(result.final_state.rng.draws(), 1);
    }

    #[test]
    fn direct_and_mediated_influence_links() {
        let result = run_demo(1).expect("demo");
        let action = result
            .events
            .iter()
            .find(|e| e.event_type == "player_action_recorded")
            .unwrap();
        assert!(action.caused_by.is_none());
        assert!(action.influence_links.is_empty());

        let stance = result
            .events
            .iter()
            .find(|e| e.event_type == "player_stance_changed")
            .unwrap();
        assert_eq!(stance.caused_by, Some(action.event_id));
        assert_eq!(stance.influence_links[0].path_length, 1);
        assert_eq!(
            stance.influence_links[0].contribution_class,
            ContributionClass::Direct
        );

        let revealed = result
            .events
            .iter()
            .find(|e| e.event_type == "information_revealed")
            .unwrap();
        let resolved = result
            .events
            .iter()
            .find(|e| e.event_type == "information_resolved")
            .unwrap();
        assert_eq!(resolved.caused_by, Some(revealed.event_id));
        assert_eq!(resolved.influence_links[0].path_length, 2);
        assert_eq!(
            resolved.influence_links[0].contribution_class,
            ContributionClass::Mediated
        );
        assert_eq!(
            resolved.influence_links[0].top_contributors,
            vec![action.event_id, revealed.event_id]
        );
    }

    #[test]
    fn initial_state_not_mutated() {
        let result = run_demo(1).expect("demo");
        assert_eq!(result.initial_state.player.stance, "house_darian_support");
        assert_eq!(
            result
                .initial_state
                .declared_support_count("candidate-darian"),
            2
        );
        assert_eq!(
            result.initial_state.information[0].visibility,
            InformationVisibility::Private
        );
        assert!(result.initial_state.events.is_empty());
    }

    #[test]
    fn demo_does_not_panic() {
        for seed in [0u64, 1, 2, 42, 999] {
            run_demo(seed).expect("demo should succeed");
        }
    }

    #[test]
    fn uninterrupted_equals_checkpoint_resume() {
        let baseline = run_demo_to_runtime(1).expect("baseline");
        let (resumed, checkpoint_bytes) = run_demo_via_checkpoint(1).expect("resumed");

        assert_eq!(baseline.world, resumed.world);
        assert_eq!(baseline.world.events, resumed.world.events);
        assert_eq!(baseline.world.rng, resumed.world.rng);
        assert_eq!(baseline.scheduler, resumed.scheduler);
        assert_eq!(
            baseline.world.next_command_sequence,
            resumed.world.next_command_sequence
        );

        // save → load → save byte 동일
        let re_saved = save_runtime_to_bytes(
            &load_runtime_from_bytes(&checkpoint_bytes).expect("reload checkpoint"),
        )
        .expect("re-save");
        assert_eq!(checkpoint_bytes, re_saved);

        // 최종 compact 비교용: world 전체 직렬화
        let ba = serde_json::to_vec(&baseline.world).expect("json a");
        let bb = serde_json::to_vec(&resumed.world).expect("json b");
        assert_eq!(ba, bb);
    }

    #[test]
    fn checkpoint_pending_commands_match_after_load() {
        let checkpoint = create_demo_checkpoint(1).expect("checkpoint");
        assert_eq!(checkpoint.scheduler.len(), 3);
        assert_eq!(checkpoint.world.rng.draws(), 0);
        let bytes = save_runtime_to_bytes(&checkpoint).expect("save");
        let loaded = load_runtime_from_bytes(&bytes).expect("load");
        assert_eq!(loaded.scheduler.len(), 3);
        assert_eq!(
            loaded.scheduler.pending().to_vec(),
            checkpoint.scheduler.to_snapshot().queue
        );
        assert_eq!(
            loaded.scheduler.next_sequence(),
            checkpoint.scheduler.next_sequence()
        );
    }
}
