// EPOCH 결정론 코어 — 명령·스케줄·사건 재생 계약

pub mod command;
pub mod demo;
pub mod error;
pub mod event;
pub mod model;
pub mod rng;
pub mod scheduler;

pub use command::{
    Command, CommandEnvelope, EXPOSE_DUPLICATE_PROMISE, SubmitSpec, execute_command, submit_command,
};
pub use demo::{DemoResult, run_demo};
pub use error::CoreError;
pub use event::{ContributionClass, Event, InfluenceLink, RandomDraw, StateChange};
pub use model::{
    Candidate, House, Information, InformationVisibility, Player, SupportStatus, WorldState,
};
pub use rng::DeterministicRng;
pub use scheduler::{Phase, ScheduledKey, Scheduler};
