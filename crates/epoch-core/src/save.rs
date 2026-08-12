// 결정론적 RuntimeState 저장·로드

use crate::error::CoreError;
use crate::model::{SupportStatus, WorldState};
use crate::runtime::RuntimeState;
use crate::scheduler::{Scheduler, SchedulerSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 저장 포맷 schema 버전.
pub const SAVE_SCHEMA_VERSION: u32 = 1;

/// schema v1 save envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveEnvelope {
    pub schema_version: u32,
    pub world: WorldState,
    pub scheduler: SchedulerSnapshot,
}

/// RuntimeState를 compact JSON bytes로 직렬화한다.
pub fn save_runtime_to_bytes(state: &RuntimeState) -> Result<Vec<u8>, CoreError> {
    let envelope = SaveEnvelope {
        schema_version: SAVE_SCHEMA_VERSION,
        world: state.world.clone(),
        scheduler: state.scheduler.to_snapshot(),
    };
    serde_json::to_vec(&envelope).map_err(|e| CoreError::Serialization(e.to_string()))
}

/// compact JSON bytes에서 RuntimeState를 복원한다. 불변식 위반 시 부분 상태를 반환하지 않는다.
pub fn load_runtime_from_bytes(bytes: &[u8]) -> Result<RuntimeState, CoreError> {
    let envelope: SaveEnvelope = serde_json::from_slice(bytes).map_err(|e| {
        // 전체 JSON을 오류 메시지에 넣지 않는다.
        CoreError::SaveDecode(summarize_decode_error(&e))
    })?;

    if envelope.schema_version != SAVE_SCHEMA_VERSION {
        return Err(CoreError::UnsupportedSaveSchema {
            version: envelope.schema_version,
        });
    }

    validate_loaded_state(&envelope.world, &envelope.scheduler)?;

    let scheduler = Scheduler::from_snapshot(envelope.scheduler)?;
    Ok(RuntimeState {
        world: envelope.world,
        scheduler,
    })
}

fn summarize_decode_error(err: &serde_json::Error) -> String {
    // 라인/열만 남기고 본문 조각은 넣지 않는다.
    format!(
        "malformed JSON at line {} column {}",
        err.line(),
        err.column()
    )
}

/// 로드 직후 최소 불변식 검사. 실패 시 스케줄러를 만들지 않는다.
fn validate_loaded_state(
    world: &WorldState,
    scheduler: &SchedulerSnapshot,
) -> Result<(), CoreError> {
    validate_houses(world)?;
    validate_events(world)?;
    validate_sequence_alignment(world, scheduler)?;
    validate_pending_caused_by(world, scheduler)?;
    // 스케줄러 스냅샷 자체 검증 (from_snapshot에서도 수행하나, 여기서 먼저 fail closed).
    let _ = Scheduler::from_snapshot(scheduler.clone())?;
    Ok(())
}

fn validate_houses(world: &WorldState) -> Result<(), CoreError> {
    for house in &world.houses {
        match house.support_status {
            SupportStatus::Declared => {
                let cand = house.supported_candidate.as_ref().ok_or_else(|| {
                    CoreError::InvalidSaveInvariant(format!(
                        "declared house {} missing candidate",
                        house.id
                    ))
                })?;
                if !world.candidate_exists(cand) {
                    return Err(CoreError::InvalidSaveInvariant(format!(
                        "declared house {} references unknown candidate {}",
                        house.id, cand
                    )));
                }
            }
            SupportStatus::Undecided => {
                if let Some(cand) = &house.supported_candidate {
                    return Err(CoreError::InvalidSaveInvariant(format!(
                        "undecided house {} has candidate {}",
                        house.id, cand
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_events(world: &WorldState) -> Result<(), CoreError> {
    let mut seen = BTreeSet::new();
    let mut prev_id = 0u64;
    for event in &world.events {
        if !seen.insert(event.event_id) {
            return Err(CoreError::InvalidSaveInvariant(format!(
                "duplicate event_id {}",
                event.event_id
            )));
        }
        // 실행 순서와 모순되지 않도록 event_id는 기록 순서상 단조 증가해야 한다.
        if event.event_id <= prev_id {
            return Err(CoreError::InvalidSaveInvariant(format!(
                "event_id {} is not strictly increasing after {}",
                event.event_id, prev_id
            )));
        }
        prev_id = event.event_id;
    }

    // next_event_id가 기존 사건과 충돌하지 않음.
    if seen.contains(&world.next_event_id) {
        return Err(CoreError::InvalidSaveInvariant(format!(
            "next_event_id {} collides with existing event",
            world.next_event_id
        )));
    }

    // 기존 사건 ID 범위와 next_event_id 모순: 기록된 최대 ID 초과여야 한다.
    if let Some(&max_id) = seen.iter().next_back()
        && world.next_event_id <= max_id
    {
        return Err(CoreError::InvalidSaveInvariant(format!(
            "next_event_id {} must be greater than max event_id {max_id}",
            world.next_event_id
        )));
    }

    Ok(())
}

fn validate_sequence_alignment(
    world: &WorldState,
    scheduler: &SchedulerSnapshot,
) -> Result<(), CoreError> {
    if world.next_command_sequence != scheduler.next_sequence {
        return Err(CoreError::InvalidSaveInvariant(format!(
            "world.next_command_sequence {} != scheduler.next_sequence {}",
            world.next_command_sequence, scheduler.next_sequence
        )));
    }
    Ok(())
}

fn validate_pending_caused_by(
    world: &WorldState,
    scheduler: &SchedulerSnapshot,
) -> Result<(), CoreError> {
    let event_ids: BTreeSet<u64> = world.events.iter().map(|e| e.event_id).collect();
    for env in &scheduler.queue {
        if let Some(cause) = env.caused_by_event
            && !event_ids.contains(&cause)
        {
            return Err(CoreError::InvalidSaveInvariant(format!(
                "pending command {} caused_by_event {cause} does not exist",
                env.command_id
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Command, EXPOSE_DUPLICATE_PROMISE, SubmitSpec, submit_command};
    use crate::model::WorldState;
    use crate::runtime::RuntimeState;
    use crate::scheduler::{Phase, Scheduler};

    fn demo_checkpoint_runtime(seed: u64) -> RuntimeState {
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
        )
        .expect("submit");
        let mut rt = RuntimeState::new(world, scheduler);
        rt.step().expect("step").expect("had command");
        rt
    }

    #[test]
    fn checkpoint_has_pending_and_zero_rng_draws() {
        let rt = demo_checkpoint_runtime(1);
        assert_eq!(rt.world.world_time, 100);
        assert_eq!(rt.world.events.len(), 1);
        assert_eq!(rt.world.events[0].event_type, "player_action_recorded");
        assert_eq!(rt.scheduler.len(), 3);
        assert_eq!(rt.world.rng.draws(), 0);
        assert_eq!(rt.world.player.stance, "house_darian_support");
    }

    #[test]
    fn save_bytes_identical_for_same_state() {
        let rt = demo_checkpoint_runtime(1);
        let a = save_runtime_to_bytes(&rt).expect("save a");
        let b = save_runtime_to_bytes(&rt).expect("save b");
        assert_eq!(a, b);
    }

    #[test]
    fn save_load_save_bytes_identical() {
        let rt = demo_checkpoint_runtime(1);
        let a = save_runtime_to_bytes(&rt).expect("save a");
        let loaded = load_runtime_from_bytes(&a).expect("load");
        let c = save_runtime_to_bytes(&loaded).expect("save c");
        assert_eq!(a, c);
        assert_eq!(loaded, rt);
    }

    #[test]
    fn rng_preserved_across_save_load() {
        let rt = demo_checkpoint_runtime(1);
        // checkpoint 직후 draws == 0 유지 확인 후, draw 없이 저장.
        let seed = rt.world.rng.seed();
        let state = rt.world.rng.state();
        let draws = rt.world.rng.draws();
        let bytes = save_runtime_to_bytes(&rt).expect("save");
        let loaded = load_runtime_from_bytes(&bytes).expect("load");
        assert_eq!(loaded.world.rng.seed(), seed);
        assert_eq!(loaded.world.rng.state(), state);
        assert_eq!(loaded.world.rng.draws(), draws);
        // 저장·로드 자체가 RNG를 소비하지 않음 (원본도 동일).
        assert_eq!(rt.world.rng.draws(), draws);
        assert_eq!(rt.world.rng.state(), state);
    }

    #[test]
    fn scheduler_snapshot_restore_equal() {
        let rt = demo_checkpoint_runtime(1);
        let snap = rt.scheduler.to_snapshot();
        let restored = Scheduler::from_snapshot(snap.clone()).expect("restore");
        assert_eq!(restored, rt.scheduler);
        assert_eq!(restored.to_snapshot(), snap);
    }

    #[test]
    fn rejects_world_scheduler_sequence_mismatch() {
        let rt = demo_checkpoint_runtime(1);
        let mut envelope = SaveEnvelope {
            schema_version: SAVE_SCHEMA_VERSION,
            world: rt.world.clone(),
            scheduler: rt.scheduler.to_snapshot(),
        };
        envelope.world.next_command_sequence = envelope.scheduler.next_sequence + 1;
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CoreError::InvalidSaveInvariant(_)));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let rt = demo_checkpoint_runtime(1);
        let envelope = SaveEnvelope {
            schema_version: 99,
            world: rt.world.clone(),
            scheduler: rt.scheduler.to_snapshot(),
        };
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(
            err,
            CoreError::UnsupportedSaveSchema { version: 99 }
        ));
    }

    #[test]
    fn rejects_malformed_json() {
        let err = load_runtime_from_bytes(b"{not json").unwrap_err();
        assert!(matches!(err, CoreError::SaveDecode(_)));
        // 오류 문자열에 원본 조각이 과도하게 포함되지 않음.
        let msg = err.to_string();
        assert!(!msg.contains("{not json"));
    }

    #[test]
    fn rejects_broken_house_support() {
        let mut rt = demo_checkpoint_runtime(1);
        // Declared without candidate
        rt.world.houses[0].supported_candidate = None;
        let bytes = save_runtime_to_bytes(&rt).expect("save");
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CoreError::InvalidSaveInvariant(_)));
    }

    #[test]
    fn rejects_duplicate_event_id() {
        let mut rt = demo_checkpoint_runtime(1);
        let dup = rt.world.events[0].clone();
        rt.world.events.push(dup);
        let bytes = save_runtime_to_bytes(&rt).expect("save");
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CoreError::InvalidSaveInvariant(_)));
    }

    #[test]
    fn rejects_missing_caused_by_event() {
        let rt = demo_checkpoint_runtime(1);
        // pending의 caused_by_event를 존재하지 않는 ID로 변조
        let snap = rt.scheduler.to_snapshot();
        let mut queue = snap.queue;
        queue[0].caused_by_event = Some(999);
        let bad = SchedulerSnapshot {
            queue,
            next_sequence: snap.next_sequence,
            used_sequences: snap.used_sequences,
        };
        let envelope = SaveEnvelope {
            schema_version: SAVE_SCHEMA_VERSION,
            world: rt.world.clone(),
            scheduler: bad,
        };
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CoreError::InvalidSaveInvariant(_)));
    }

    #[test]
    fn rejects_pending_sequence_not_in_used() {
        let rt = demo_checkpoint_runtime(1);
        let mut snap = rt.scheduler.to_snapshot();
        // used에서 pending sequence 하나를 제거
        let pending_seq = snap.queue[0].scheduled_key.sequence;
        snap.used_sequences.retain(|&s| s != pending_seq);
        let envelope = SaveEnvelope {
            schema_version: SAVE_SCHEMA_VERSION,
            world: rt.world.clone(),
            scheduler: snap,
        };
        let bytes = serde_json::to_vec(&envelope).unwrap();
        let err = load_runtime_from_bytes(&bytes).unwrap_err();
        assert!(matches!(err, CoreError::InvalidSaveInvariant(_)));
    }
}
