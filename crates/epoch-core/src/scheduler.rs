// 결정론적 스케줄 키와 단계 순서

use crate::error::CoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 동일 시각 실행 단계. 선언 순서가 실행 순서다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Expiry,
    RightsRecalculation,
    ActiveActorUpdate,
    InformationUpdate,
    SupportPlanReevaluation,
    ActionExecution,
    StateChangeEventRecording,
    UiSummary,
}

/// 스케줄 정렬 키: (time, phase, priority, actor_id, sequence)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledKey {
    pub time: u64,
    pub phase: Phase,
    pub priority: i32,
    pub actor_id: String,
    pub sequence: u64,
}

impl PartialOrd for ScheduledKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| self.phase.cmp(&other.phase))
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| self.actor_id.cmp(&other.actor_id))
            .then_with(|| self.sequence.cmp(&other.sequence))
    }
}

/// Vec 정렬 기반 최소 스케줄러. 단조 증가 sequence를 소유한다.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    queue: Vec<crate::command::CommandEnvelope>,
    /// 다음에 부여할 sequence.
    next_sequence: u64,
    /// 등록에 사용된 sequence 집합 (pop 후에도 재사용 금지).
    used_sequences: BTreeSet<u64>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            next_sequence: 0,
            used_sequences: BTreeSet::new(),
        }
    }

    /// 세계 상태의 next_command_sequence와 맞춰 시작한다.
    pub fn with_next_sequence(next_sequence: u64) -> Self {
        Self {
            queue: Vec::new(),
            next_sequence,
            used_sequences: BTreeSet::new(),
        }
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    /// 단조 증가 sequence를 부여해 등록한다. 충돌·오버플로 시 큐를 변경하지 않는다.
    pub fn register(
        &mut self,
        mut envelope: crate::command::CommandEnvelope,
    ) -> Result<u64, CoreError> {
        let sequence = self.next_sequence;
        if self.used_sequences.contains(&sequence)
            || self
                .queue
                .iter()
                .any(|e| e.scheduled_key.sequence == sequence)
        {
            return Err(CoreError::CommandSequenceCollision { sequence });
        }
        let next = sequence
            .checked_add(1)
            .ok_or(CoreError::CommandSequenceOverflow)?;

        envelope.scheduled_key.sequence = sequence;
        self.used_sequences.insert(sequence);
        self.next_sequence = next;
        self.queue.push(envelope);
        Ok(sequence)
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// 안정 키 오름차순으로 다음 명령을 꺼낸다.
    pub fn pop_next(&mut self) -> Option<crate::command::CommandEnvelope> {
        if self.queue.is_empty() {
            return None;
        }
        self.queue
            .sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        Some(self.queue.remove(0))
    }

    pub fn pending(&self) -> &[crate::command::CommandEnvelope] {
        &self.queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Command, CommandEnvelope};

    fn envelope_template(
        time: u64,
        phase: Phase,
        priority: i32,
        actor_id: &str,
        command_id: &str,
    ) -> CommandEnvelope {
        CommandEnvelope {
            command_id: command_id.to_string(),
            scheduled_key: ScheduledKey {
                time,
                phase,
                priority,
                actor_id: actor_id.to_string(),
                sequence: 0,
            },
            issued_by: "test".to_string(),
            caused_by_event: None,
            command: Command::SetPlayerStance {
                player_id: "player-ren-arden".to_string(),
                stance: "test".to_string(),
            },
        }
    }

    fn envelope_with_sequence(
        time: u64,
        phase: Phase,
        priority: i32,
        actor_id: &str,
        sequence: u64,
        command_id: &str,
    ) -> CommandEnvelope {
        let mut e = envelope_template(time, phase, priority, actor_id, command_id);
        e.scheduled_key.sequence = sequence;
        e
    }

    #[test]
    fn phase_order_matches_spec() {
        let phases = [
            Phase::Expiry,
            Phase::RightsRecalculation,
            Phase::ActiveActorUpdate,
            Phase::InformationUpdate,
            Phase::SupportPlanReevaluation,
            Phase::ActionExecution,
            Phase::StateChangeEventRecording,
            Phase::UiSummary,
        ];
        for window in phases.windows(2) {
            assert!(window[0] < window[1]);
        }
    }

    #[test]
    fn stable_key_sort_same_time() {
        let mut items = [
            envelope_with_sequence(100, Phase::UiSummary, 0, "a", 0, "c3"),
            envelope_with_sequence(100, Phase::ActionExecution, 0, "a", 0, "c1"),
            envelope_with_sequence(100, Phase::StateChangeEventRecording, 0, "a", 0, "c2"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "c1");
        assert_eq!(items[1].command_id, "c2");
        assert_eq!(items[2].command_id, "c3");
    }

    #[test]
    fn actor_id_and_priority_sort() {
        let mut items = [
            envelope_with_sequence(50, Phase::ActionExecution, 10, "b", 0, "p10-b"),
            envelope_with_sequence(50, Phase::ActionExecution, 0, "b", 0, "p0-b"),
            envelope_with_sequence(50, Phase::ActionExecution, 0, "a", 0, "p0-a"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "p0-a");
        assert_eq!(items[1].command_id, "p0-b");
        assert_eq!(items[2].command_id, "p10-b");
    }

    #[test]
    fn sequence_is_final_tie_break() {
        let mut items = [
            envelope_with_sequence(1, Phase::ActionExecution, 0, "x", 2, "s2"),
            envelope_with_sequence(1, Phase::ActionExecution, 0, "x", 0, "s0"),
            envelope_with_sequence(1, Phase::ActionExecution, 0, "x", 1, "s1"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "s0");
        assert_eq!(items[1].command_id, "s1");
        assert_eq!(items[2].command_id, "s2");
    }

    #[test]
    fn scheduler_assigns_monotonic_sequences() {
        let mut sched = Scheduler::new();
        let s0 = sched
            .register(envelope_template(
                10,
                Phase::ActionExecution,
                0,
                "a",
                "later",
            ))
            .unwrap();
        let s1 = sched
            .register(envelope_template(
                5,
                Phase::ActionExecution,
                0,
                "a",
                "earlier",
            ))
            .unwrap();
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(sched.pending()[0].scheduled_key.sequence, 0);
        assert_eq!(sched.pending()[1].scheduled_key.sequence, 1);
        let first = sched.pop_next().unwrap();
        assert_eq!(first.command_id, "earlier");
        assert_eq!(first.scheduled_key.sequence, 1);
        let second = sched.pop_next().unwrap();
        assert_eq!(second.command_id, "later");
        assert_eq!(second.scheduled_key.sequence, 0);
        assert!(sched.is_empty());
    }

    #[test]
    fn register_rejects_reused_sequence_counter() {
        let mut sched = Scheduler::new();
        sched
            .register(envelope_template(1, Phase::ActionExecution, 0, "a", "a"))
            .unwrap();
        // 강제로 next_sequence를 이미 사용된 값으로 되돌려 충돌 경로를 검증한다.
        sched.next_sequence = 0;
        let err = sched
            .register(envelope_template(1, Phase::ActionExecution, 0, "a", "b"))
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::CommandSequenceCollision { sequence: 0 }
        ));
        assert_eq!(sched.len(), 1);
    }
}
