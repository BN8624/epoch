// 명령 봉투, 검증, 실행

use crate::error::CoreError;
use crate::event::{ContributionClass, Event, InfluenceLink, RandomDraw, StateChange, reason};
use crate::model::{InformationVisibility, SupportStatus, WorldState};
use crate::scheduler::{Phase, ScheduledKey, Scheduler};
use serde::{Deserialize, Serialize};

/// 고정 플레이어 행동 코드.
pub const EXPOSE_DUPLICATE_PROMISE: &str = "EXPOSE_DUPLICATE_PROMISE";

/// 최소 명령 타입.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    RecordPlayerAction {
        action_code: String,
    },
    SetPlayerStance {
        player_id: String,
        stance: String,
    },
    SetHouseSupport {
        house_id: String,
        support_status: SupportStatus,
        supported_candidate: Option<String>,
    },
    RevealInformation {
        info_id: String,
        visibility: InformationVisibility,
    },
    ResolveInformation {
        info_id: String,
        chance_basis_points: u32,
    },
}

/// 명령 메타데이터 봉투.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub command_id: String,
    pub scheduled_key: ScheduledKey,
    pub issued_by: String,
    pub caused_by_event: Option<u64>,
    pub command: Command,
}

/// 실행 중 추가 스케줄이 필요한 후속 명령.
struct FollowUp {
    phase: Phase,
    priority: i32,
    actor_id: String,
    time: u64,
    issued_by: String,
    caused_by_event: Option<u64>,
    command: Command,
    command_id_suffix: String,
}

/// 단일 명령을 검증 후 적용한다. 오류 시 세계와 스케줄러를 모두 복원한다.
pub fn execute_command(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    envelope: &CommandEnvelope,
) -> Result<(), CoreError> {
    let world_snapshot = world.clone();
    let scheduler_snapshot = scheduler.clone();
    match apply_command(world, scheduler, envelope) {
        Ok(()) => Ok(()),
        Err(e) => {
            *world = world_snapshot;
            *scheduler = scheduler_snapshot;
            Err(e)
        }
    }
}

fn apply_command(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    envelope: &CommandEnvelope,
) -> Result<(), CoreError> {
    world.world_time = envelope.scheduled_key.time;

    match &envelope.command {
        Command::RecordPlayerAction { action_code } => {
            apply_record_player_action(world, scheduler, envelope, action_code)
        }
        Command::SetPlayerStance { player_id, stance } => {
            apply_set_player_stance(world, envelope, player_id, stance)
        }
        Command::SetHouseSupport {
            house_id,
            support_status,
            supported_candidate,
        } => apply_set_house_support(
            world,
            envelope,
            house_id,
            *support_status,
            supported_candidate.as_deref(),
        ),
        Command::RevealInformation {
            info_id,
            visibility,
        } => apply_reveal_information(world, scheduler, envelope, info_id, *visibility),
        Command::ResolveInformation {
            info_id,
            chance_basis_points,
        } => apply_resolve_information(world, envelope, info_id, *chance_basis_points),
    }
}

fn apply_record_player_action(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    envelope: &CommandEnvelope,
    action_code: &str,
) -> Result<(), CoreError> {
    if action_code != EXPOSE_DUPLICATE_PROMISE {
        return Err(CoreError::InvalidActionCode {
            action_code: action_code.to_string(),
        });
    }

    let event_id = world.allocate_event_id()?;
    let actor = world.player.id.clone();
    let event = Event::player_action(
        event_id,
        world.world_time,
        &actor,
        action_code,
        "렌 아르덴이 다리안의 중복 직위 약속을 공개했습니다.",
    );
    world.events.push(event);

    let t = world.world_time;
    let follow_ups = [
        FollowUp {
            phase: Phase::StateChangeEventRecording,
            priority: 0,
            actor_id: actor.clone(),
            time: t,
            issued_by: envelope.issued_by.clone(),
            caused_by_event: Some(event_id),
            command: Command::SetPlayerStance {
                player_id: actor.clone(),
                stance: "seria_information_cooperation".to_string(),
            },
            command_id_suffix: "set-stance".to_string(),
        },
        FollowUp {
            phase: Phase::StateChangeEventRecording,
            priority: 1,
            actor_id: actor.clone(),
            time: t,
            issued_by: envelope.issued_by.clone(),
            caused_by_event: Some(event_id),
            command: Command::SetHouseSupport {
                house_id: "house-soren".to_string(),
                support_status: SupportStatus::Undecided,
                supported_candidate: None,
            },
            command_id_suffix: "set-house-soren".to_string(),
        },
        FollowUp {
            phase: Phase::StateChangeEventRecording,
            priority: 2,
            actor_id: actor.clone(),
            time: t,
            issued_by: envelope.issued_by.clone(),
            caused_by_event: Some(event_id),
            command: Command::RevealInformation {
                info_id: "info-darian-duplicate-promise".to_string(),
                visibility: InformationVisibility::Unverified,
            },
            command_id_suffix: "reveal-info".to_string(),
        },
    ];

    for fu in follow_ups {
        schedule_follow_up(world, scheduler, fu)?;
    }
    Ok(())
}

fn apply_set_player_stance(
    world: &mut WorldState,
    envelope: &CommandEnvelope,
    player_id: &str,
    stance: &str,
) -> Result<(), CoreError> {
    if !world.player_id_matches(player_id) {
        return Err(CoreError::UnknownPlayer {
            id: player_id.to_string(),
        });
    }

    let before = world.player.stance.clone();
    if before == stance {
        return Ok(());
    }
    world.player.stance = stance.to_string();

    let event_id = world.allocate_event_id()?;
    let cause = envelope.caused_by_event;
    let mut event = Event {
        event_id,
        world_time: world.world_time,
        event_type: "player_stance_changed".to_string(),
        actors: vec![player_id.to_string()],
        targets: vec![],
        reason_codes: vec![reason::PLAYER_EXPOSED_DUPLICATE_PROMISE.to_string()],
        random_draws: vec![],
        state_changes: vec![StateChange {
            entity_id: player_id.to_string(),
            field: "stance".to_string(),
            before,
            after: stance.to_string(),
        }],
        caused_by: cause,
        influence_links: vec![],
        summary: "렌 아르덴의 입장이 세리아 측 정보 협력으로 바뀌었습니다.".to_string(),
    };
    if let Some(c) = cause {
        event = event.with_direct_cause(c);
    }
    world.events.push(event);
    Ok(())
}

fn apply_set_house_support(
    world: &mut WorldState,
    envelope: &CommandEnvelope,
    house_id: &str,
    support_status: SupportStatus,
    supported_candidate: Option<&str>,
) -> Result<(), CoreError> {
    let idx = world
        .house_index(house_id)
        .ok_or_else(|| CoreError::UnknownHouse {
            id: house_id.to_string(),
        })?;

    if support_status == SupportStatus::Declared {
        let cand = supported_candidate.ok_or_else(|| CoreError::MissingCandidateForSupport {
            house_id: house_id.to_string(),
        })?;
        if !world.candidate_exists(cand) {
            return Err(CoreError::UnknownCandidate {
                id: cand.to_string(),
            });
        }
    }

    if let Some(cand) = supported_candidate
        && !world.candidate_exists(cand)
    {
        return Err(CoreError::UnknownCandidate {
            id: cand.to_string(),
        });
    }

    let house = &world.houses[idx];
    let before_status = support_status_str(house.support_status).to_string();
    let before_cand = house
        .supported_candidate
        .clone()
        .unwrap_or_else(|| "none".to_string());

    let after_status = support_status_str(support_status);
    let after_cand = supported_candidate.unwrap_or("none");

    world.houses[idx].support_status = support_status;
    world.houses[idx].supported_candidate = supported_candidate.map(|s| s.to_string());

    let event_id = world.allocate_event_id()?;
    let cause = envelope.caused_by_event;
    let mut event = Event {
        event_id,
        world_time: world.world_time,
        event_type: "house_support_changed".to_string(),
        actors: vec![house_id.to_string()],
        targets: supported_candidate
            .map(|s| vec![s.to_string()])
            .unwrap_or_default(),
        reason_codes: vec![reason::HOUSE_RECONSIDERED_DUPLICATE_PROMISE.to_string()],
        random_draws: vec![],
        state_changes: vec![
            StateChange {
                entity_id: house_id.to_string(),
                field: "support_status".to_string(),
                before: before_status,
                after: after_status.to_string(),
            },
            StateChange {
                entity_id: house_id.to_string(),
                field: "supported_candidate".to_string(),
                before: before_cand,
                after: after_cand.to_string(),
            },
        ],
        caused_by: cause,
        influence_links: vec![],
        summary: "소렌 가문이 다리안에 대한 공개 지지를 철회했습니다.".to_string(),
    };
    if let Some(c) = cause {
        event = event.with_direct_cause(c);
    }
    world.events.push(event);
    Ok(())
}

fn apply_reveal_information(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    envelope: &CommandEnvelope,
    info_id: &str,
    visibility: InformationVisibility,
) -> Result<(), CoreError> {
    let idx = world
        .information_index(info_id)
        .ok_or_else(|| CoreError::UnknownInformation {
            id: info_id.to_string(),
        })?;

    let current = world.information[idx].visibility;
    validate_visibility_transition(current, visibility)?;

    let before = visibility_str(current).to_string();
    world.information[idx].visibility = visibility;
    let after = visibility_str(visibility).to_string();

    let event_id = world.allocate_event_id()?;
    let cause = envelope.caused_by_event;
    let mut event = Event {
        event_id,
        world_time: world.world_time,
        event_type: "information_revealed".to_string(),
        actors: vec![world.player.id.clone()],
        targets: vec![info_id.to_string()],
        reason_codes: vec![reason::INFORMATION_DISCLOSED.to_string()],
        random_draws: vec![],
        state_changes: vec![StateChange {
            entity_id: info_id.to_string(),
            field: "visibility".to_string(),
            before,
            after,
        }],
        caused_by: cause,
        influence_links: vec![],
        summary: "다리안의 중복 직위 약속 정보가 미확인 상태로 공개되었습니다.".to_string(),
    };
    if let Some(c) = cause {
        event = event.with_direct_cause(c);
    }
    world.events.push(event);

    // 다음 시각에 확인 명령 스케줄
    schedule_follow_up(
        world,
        scheduler,
        FollowUp {
            phase: Phase::InformationUpdate,
            priority: 0,
            actor_id: world.player.id.clone(),
            time: world.world_time.saturating_add(1),
            issued_by: envelope.issued_by.clone(),
            caused_by_event: Some(event_id),
            command: Command::ResolveInformation {
                info_id: info_id.to_string(),
                chance_basis_points: 5000,
            },
            command_id_suffix: format!("resolve-{info_id}"),
        },
    )?;

    Ok(())
}

fn apply_resolve_information(
    world: &mut WorldState,
    envelope: &CommandEnvelope,
    info_id: &str,
    chance_basis_points: u32,
) -> Result<(), CoreError> {
    if chance_basis_points > 10_000 {
        return Err(CoreError::InvalidChanceBasisPoints {
            value: chance_basis_points,
        });
    }

    let idx = world
        .information_index(info_id)
        .ok_or_else(|| CoreError::UnknownInformation {
            id: info_id.to_string(),
        })?;

    let current = world.information[idx].visibility;
    let draw_index = world.rng.draws() + 1;
    let (raw_value, success) = world.rng.roll_basis_points(chance_basis_points);

    let before = visibility_str(current).to_string();
    let new_vis = if success {
        InformationVisibility::PublicFact
    } else if current == InformationVisibility::Private {
        InformationVisibility::Unverified
    } else {
        current
    };
    world.information[idx].visibility = new_vis;
    let after = visibility_str(new_vis).to_string();

    let event_id = world.allocate_event_id()?;
    let revealed_event = envelope.caused_by_event;

    // 매개 인과: root = 공개 사건의 caused_by(플레이어 행동)
    let root_source = revealed_event.and_then(|rid| {
        world
            .events
            .iter()
            .find(|e| e.event_id == rid)
            .and_then(|e| e.caused_by)
    });

    let mut event = Event {
        event_id,
        world_time: world.world_time,
        event_type: "information_resolved".to_string(),
        actors: vec![world.player.id.clone()],
        targets: vec![info_id.to_string()],
        reason_codes: vec![reason::INFORMATION_VERIFICATION_DRAW.to_string()],
        random_draws: vec![RandomDraw {
            stream: "main".to_string(),
            draw_index,
            raw_value,
            chance_basis_points,
            success,
        }],
        state_changes: vec![StateChange {
            entity_id: info_id.to_string(),
            field: "visibility".to_string(),
            before,
            after,
        }],
        caused_by: revealed_event,
        influence_links: vec![],
        summary: if success {
            "다리안의 중복 직위 약속이 공개 사실로 확정되었습니다.".to_string()
        } else {
            "다리안의 중복 직위 약속 확인에 실패해 미확인 상태가 유지되었습니다.".to_string()
        },
    };

    if let (Some(caused_by), Some(root)) = (revealed_event, root_source) {
        event = event.with_mediated_cause(caused_by, root, caused_by);
    } else if let Some(caused_by) = revealed_event {
        event.caused_by = Some(caused_by);
        event.influence_links = vec![InfluenceLink {
            source_event: caused_by,
            path_length: 1,
            contribution_class: ContributionClass::Direct,
            top_contributors: vec![caused_by],
        }];
    }

    world.events.push(event);
    Ok(())
}

fn validate_visibility_transition(
    current: InformationVisibility,
    next: InformationVisibility,
) -> Result<(), CoreError> {
    if current == InformationVisibility::PublicFact && next == InformationVisibility::Private {
        return Err(CoreError::VisibilityRegression {
            from: visibility_str(current).to_string(),
            to: visibility_str(next).to_string(),
        });
    }
    Ok(())
}

fn visibility_str(v: InformationVisibility) -> &'static str {
    match v {
        InformationVisibility::Private => "private",
        InformationVisibility::Unverified => "unverified",
        InformationVisibility::PublicFact => "public_fact",
    }
}

fn support_status_str(s: SupportStatus) -> &'static str {
    match s {
        SupportStatus::Declared => "declared",
        SupportStatus::Undecided => "undecided",
    }
}

/// Scheduler가 sequence를 부여하고, 성공 시에만 세계 카운터를 동기화한다.
fn schedule_follow_up(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    fu: FollowUp,
) -> Result<(), CoreError> {
    let sequence = scheduler.next_sequence();
    let envelope = CommandEnvelope {
        command_id: format!("cmd-{sequence}-{}", fu.command_id_suffix),
        scheduled_key: ScheduledKey {
            time: fu.time,
            phase: fu.phase,
            priority: fu.priority,
            actor_id: fu.actor_id,
            sequence: 0,
        },
        issued_by: fu.issued_by,
        caused_by_event: fu.caused_by_event,
        command: fu.command,
    };
    scheduler.register(envelope)?;
    world.next_command_sequence = scheduler.next_sequence();
    Ok(())
}

/// 외부 제출용 스케줄 인자.
pub struct SubmitSpec {
    pub time: u64,
    pub phase: Phase,
    pub priority: i32,
    pub actor_id: String,
    pub issued_by: String,
    pub caused_by_event: Option<u64>,
    pub command: Command,
    pub command_id: String,
}

/// 외부에서 초기 명령을 스케줄에 등록한다.
/// sequence 충돌은 Scheduler가 상태 변경 전에 거부하며, 오류 시 세계·스케줄러를 변경하지 않는다.
pub fn submit_command(
    world: &mut WorldState,
    scheduler: &mut Scheduler,
    spec: SubmitSpec,
) -> Result<(), CoreError> {
    let envelope = CommandEnvelope {
        command_id: spec.command_id,
        scheduled_key: ScheduledKey {
            time: spec.time,
            phase: spec.phase,
            priority: spec.priority,
            actor_id: spec.actor_id,
            sequence: 0,
        },
        issued_by: spec.issued_by,
        caused_by_event: spec.caused_by_event,
        command: spec.command,
    };
    scheduler.register(envelope)?;
    world.next_command_sequence = scheduler.next_sequence();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::Scheduler;

    fn fresh() -> (WorldState, Scheduler) {
        (WorldState::new_initial(1), Scheduler::new())
    }

    #[test]
    fn rejects_unknown_house() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetHouseSupport {
                house_id: "house-nope".into(),
                support_status: SupportStatus::Undecided,
                supported_candidate: None,
            },
        };
        let before = world.clone();
        let err = execute_command(&mut world, &mut sched, &env).unwrap_err();
        assert!(matches!(err, CoreError::UnknownHouse { .. }));
        assert_eq!(world, before);
    }

    #[test]
    fn rejects_unknown_candidate() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetHouseSupport {
                house_id: "house-soren".into(),
                support_status: SupportStatus::Declared,
                supported_candidate: Some("candidate-ghost".into()),
            },
        };
        let before = world.clone();
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::UnknownCandidate { .. })
        ));
        assert_eq!(world, before);
    }

    #[test]
    fn rejects_unknown_information() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::RevealInformation {
                info_id: "info-missing".into(),
                visibility: InformationVisibility::Unverified,
            },
        };
        let before = world.clone();
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::UnknownInformation { .. })
        ));
        assert_eq!(world, before);
    }

    #[test]
    fn rejects_unknown_player() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetPlayerStance {
                player_id: "player-other".into(),
                stance: "x".into(),
            },
        };
        let before = world.clone();
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::UnknownPlayer { .. })
        ));
        assert_eq!(world, before);
    }

    #[test]
    fn rejects_declared_without_candidate() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetHouseSupport {
                house_id: "house-soren".into(),
                support_status: SupportStatus::Declared,
                supported_candidate: None,
            },
        };
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::MissingCandidateForSupport { .. })
        ));
    }

    #[test]
    fn rejects_invalid_basis_points() {
        let (mut world, mut sched) = fresh();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::ResolveInformation {
                info_id: "info-darian-duplicate-promise".into(),
                chance_basis_points: 10001,
            },
        };
        let before = world.clone();
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::InvalidChanceBasisPoints { .. })
        ));
        assert_eq!(world.rng.draws(), before.rng.draws());
        assert_eq!(world, before);
    }

    #[test]
    fn rejects_visibility_regression_to_private() {
        let (mut world, mut sched) = fresh();
        world.information[0].visibility = InformationVisibility::PublicFact;
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::RevealInformation {
                info_id: "info-darian-duplicate-promise".into(),
                visibility: InformationVisibility::Private,
            },
        };
        let before = world.clone();
        assert!(matches!(
            execute_command(&mut world, &mut sched, &env),
            Err(CoreError::VisibilityRegression { .. })
        ));
        assert_eq!(world, before);
    }

    #[test]
    fn error_does_not_mutate_state() {
        let (mut world, mut sched) = fresh();
        let before = world.clone();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetHouseSupport {
                house_id: "missing".into(),
                support_status: SupportStatus::Undecided,
                supported_candidate: None,
            },
        };
        let _ = execute_command(&mut world, &mut sched, &env);
        assert_eq!(world.events.len(), before.events.len());
        assert_eq!(world.next_event_id, before.next_event_id);
        assert_eq!(world.player, before.player);
    }

    #[test]
    fn event_id_collision_rejects_without_mutation() {
        let (mut world, mut sched) = fresh();
        world.events.push(Event::player_action(
            1,
            0,
            "player-ren-arden",
            EXPOSE_DUPLICATE_PROMISE,
            "pre-existing",
        ));
        // next_event_id is still 1 → collision
        let before = world.clone();
        let env = CommandEnvelope {
            command_id: "t".into(),
            scheduled_key: ScheduledKey {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "x".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::SetPlayerStance {
                player_id: "player-ren-arden".into(),
                stance: "new-stance".into(),
            },
        };
        let err = execute_command(&mut world, &mut sched, &env).unwrap_err();
        assert!(matches!(err, CoreError::EventIdCollision { event_id: 1 }));
        assert_eq!(world.next_event_id, before.next_event_id);
        assert_eq!(world.player.stance, before.player.stance);
        assert_eq!(world.events.len(), before.events.len());
    }

    #[test]
    fn execute_error_restores_scheduler() {
        let (mut world, mut sched) = fresh();
        // 후속 등록 중 충돌을 만들기 위해 sequence를 선점
        sched
            .register(CommandEnvelope {
                command_id: "blocker".into(),
                scheduled_key: ScheduledKey {
                    time: 999,
                    phase: Phase::UiSummary,
                    priority: 0,
                    actor_id: "z".into(),
                    sequence: 0,
                },
                issued_by: "test".into(),
                caused_by_event: None,
                command: Command::SetPlayerStance {
                    player_id: "player-ren-arden".into(),
                    stance: "x".into(),
                },
            })
            .unwrap();
        // next_sequence is 1; force it back so next register collides after first follow-up?
        // Better: force next_sequence to a used value after first successful register inside action.
        // Instead, pre-use sequences 1,2,3 so follow-ups fail on second/third...
        // Actually first follow-up gets sequence 1. Pre-insert used via register then set next back.
        // Simpler path: allocate event succeeds, first follow-up registers (seq=1), then we need failure.
        // Use overflow: set next_sequence to u64::MAX so first follow-up fails with overflow.
        sched = Scheduler::with_next_sequence(u64::MAX);
        let before_world = world.clone();
        let before_sched = sched.clone();
        let env = CommandEnvelope {
            command_id: "action".into(),
            scheduled_key: ScheduledKey {
                time: 100,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "player-ren-arden".into(),
                sequence: 0,
            },
            issued_by: "test".into(),
            caused_by_event: None,
            command: Command::RecordPlayerAction {
                action_code: EXPOSE_DUPLICATE_PROMISE.into(),
            },
        };
        let err = execute_command(&mut world, &mut sched, &env).unwrap_err();
        assert!(matches!(err, CoreError::CommandSequenceOverflow));
        assert_eq!(world, before_world);
        assert_eq!(sched.len(), before_sched.len());
        assert_eq!(sched.next_sequence(), before_sched.next_sequence());
    }

    #[test]
    fn submit_command_does_not_advance_world_on_collision() {
        let (mut world, mut sched) = fresh();
        submit_command(
            &mut world,
            &mut sched,
            SubmitSpec {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "a".into(),
                issued_by: "t".into(),
                caused_by_event: None,
                command: Command::SetPlayerStance {
                    player_id: "player-ren-arden".into(),
                    stance: "x".into(),
                },
                command_id: "first".into(),
            },
        )
        .unwrap();
        assert_eq!(world.next_command_sequence, 1);
        // Force collision on next submit
        // Scheduler next is 1; mark it used by rolling next back after registering a phantom
        // Direct: set next_sequence to 0 which is used
        let mut sched2 = Scheduler::new();
        sched2
            .register(CommandEnvelope {
                command_id: "x".into(),
                scheduled_key: ScheduledKey {
                    time: 1,
                    phase: Phase::ActionExecution,
                    priority: 0,
                    actor_id: "a".into(),
                    sequence: 0,
                },
                issued_by: "t".into(),
                caused_by_event: None,
                command: Command::SetPlayerStance {
                    player_id: "player-ren-arden".into(),
                    stance: "x".into(),
                },
            })
            .unwrap();
        // Manually create collision path by using a scheduler that will collide
        // Re-use the internal path: register already used next by cloning and corrupting
        // We'll test via with_next_sequence(0) after sequences are used - need access.
        // Use public API only: after one register, create new world state and try
        // to register with scheduler whose next is still 0 but used contains 0.
        // The test in scheduler covers collision. Here verify submit doesn't change world
        // when register fails — inject by overflowing:
        let mut world2 = WorldState::new_initial(1);
        let mut sched_overflow = Scheduler::with_next_sequence(u64::MAX);
        let before_seq = world2.next_command_sequence;
        let err = submit_command(
            &mut world2,
            &mut sched_overflow,
            SubmitSpec {
                time: 1,
                phase: Phase::ActionExecution,
                priority: 0,
                actor_id: "a".into(),
                issued_by: "t".into(),
                caused_by_event: None,
                command: Command::SetPlayerStance {
                    player_id: "player-ren-arden".into(),
                    stance: "x".into(),
                },
                command_id: "boom".into(),
            },
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::CommandSequenceOverflow));
        assert_eq!(world2.next_command_sequence, before_seq);
        assert!(sched_overflow.is_empty());
    }
}
