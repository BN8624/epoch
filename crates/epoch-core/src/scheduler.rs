// 결정론적 스케줄 키와 단계 순서

use serde::{Deserialize, Serialize};

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

/// Vec 정렬 기반 최소 스케줄러.
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    queue: Vec<crate::command::CommandEnvelope>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    pub fn register(&mut self, envelope: crate::command::CommandEnvelope) {
        self.queue.push(envelope);
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

    fn envelope(
        time: u64,
        phase: Phase,
        priority: i32,
        actor_id: &str,
        sequence: u64,
        command_id: &str,
    ) -> CommandEnvelope {
        CommandEnvelope {
            command_id: command_id.to_string(),
            scheduled_key: ScheduledKey {
                time,
                phase,
                priority,
                actor_id: actor_id.to_string(),
                sequence,
            },
            issued_by: "test".to_string(),
            caused_by_event: None,
            command: Command::SetPlayerStance {
                player_id: "player-ren-arden".to_string(),
                stance: "test".to_string(),
            },
        }
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
            envelope(100, Phase::UiSummary, 0, "a", 0, "c3"),
            envelope(100, Phase::ActionExecution, 0, "a", 0, "c1"),
            envelope(100, Phase::StateChangeEventRecording, 0, "a", 0, "c2"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "c1");
        assert_eq!(items[1].command_id, "c2");
        assert_eq!(items[2].command_id, "c3");
    }

    #[test]
    fn actor_id_and_priority_sort() {
        let mut items = [
            envelope(50, Phase::ActionExecution, 10, "b", 0, "p10-b"),
            envelope(50, Phase::ActionExecution, 0, "b", 0, "p0-b"),
            envelope(50, Phase::ActionExecution, 0, "a", 0, "p0-a"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "p0-a");
        assert_eq!(items[1].command_id, "p0-b");
        assert_eq!(items[2].command_id, "p10-b");
    }

    #[test]
    fn sequence_is_final_tie_break() {
        let mut items = [
            envelope(1, Phase::ActionExecution, 0, "x", 2, "s2"),
            envelope(1, Phase::ActionExecution, 0, "x", 0, "s0"),
            envelope(1, Phase::ActionExecution, 0, "x", 1, "s1"),
        ];
        items.sort_by(|a, b| a.scheduled_key.cmp(&b.scheduled_key));
        assert_eq!(items[0].command_id, "s0");
        assert_eq!(items[1].command_id, "s1");
        assert_eq!(items[2].command_id, "s2");
    }

    #[test]
    fn scheduler_pops_in_key_order() {
        let mut sched = Scheduler::new();
        sched.register(envelope(10, Phase::ActionExecution, 0, "a", 1, "later"));
        sched.register(envelope(5, Phase::ActionExecution, 0, "a", 0, "earlier"));
        let first = sched.pop_next().unwrap();
        assert_eq!(first.command_id, "earlier");
        let second = sched.pop_next().unwrap();
        assert_eq!(second.command_id, "later");
        assert!(sched.is_empty());
    }
}
