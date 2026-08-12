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
    pub fn step(&mut self) -> Result<Option<CommandEnvelope>, CoreError> {
        let Some(envelope) = self.scheduler.pop_next() else {
            return Ok(None);
        };
        execute_command(&mut self.world, &mut self.scheduler, &envelope)?;
        Ok(Some(envelope))
    }

    /// 큐가 빌 때까지 명령을 실행한다.
    pub fn run_until_idle(&mut self) -> Result<(), CoreError> {
        while self.step()?.is_some() {}
        Ok(())
    }
}
