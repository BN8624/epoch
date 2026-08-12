// 세계와 스케줄러를 묶는 최소 실행 상태

use crate::command::{CommandEnvelope, execute_command};
use crate::error::CoreError;
use crate::model::WorldState;
use crate::scheduler::Scheduler;

/// 재개에 필요한 최소 실행 상태.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeState {
    pub world: WorldState,
    pub scheduler: Scheduler,
}

impl RuntimeState {
    pub fn new(world: WorldState, scheduler: Scheduler) -> Self {
        Self { world, scheduler }
    }

    /// pending 중 다음 명령 하나를 실행한다. 큐가 비면 None.
    /// 실행 실패 시 world·scheduler(pending 명령 포함)를 step 이전 상태로 복원한다.
    pub fn step(&mut self) -> Result<Option<CommandEnvelope>, CoreError> {
        if self.scheduler.is_empty() {
            return Ok(None);
        }
        // pop 이전 상태를 보존해 실패 시 pending 명령까지 원자적으로 복구한다.
        let world_backup = self.world.clone();
        let scheduler_backup = self.scheduler.clone();
        let envelope = self
            .scheduler
            .pop_next()
            .expect("scheduler non-empty checked above");
        match execute_command(&mut self.world, &mut self.scheduler, &envelope) {
            Ok(()) => Ok(Some(envelope)),
            Err(e) => {
                self.world = world_backup;
                self.scheduler = scheduler_backup;
                Err(e)
            }
        }
    }

    /// 큐가 빌 때까지 명령을 실행한다.
    pub fn run_until_idle(&mut self) -> Result<(), CoreError> {
        while self.step()?.is_some() {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Command, CommandEnvelope};
    use crate::scheduler::{Phase, ScheduledKey};

    #[test]
    fn step_restores_world_and_pending_on_command_failure() {
        let world = WorldState::new_initial(1);
        let mut scheduler = Scheduler::new();
        let envelope = CommandEnvelope {
            command_id: "fail-stance".into(),
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
        scheduler.register(envelope).expect("register");
        let mut runtime = RuntimeState::new(world, scheduler);
        let before = runtime.clone();

        let err = runtime.step().expect_err("unknown player must fail");
        assert!(matches!(err, CoreError::UnknownPlayer { .. }));
        assert_eq!(runtime, before);
        assert_eq!(runtime.scheduler.len(), 1);
        assert_eq!(runtime.scheduler.pending()[0].command_id, "fail-stance");
    }
}
