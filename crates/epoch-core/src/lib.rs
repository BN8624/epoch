// EPOCH 결정론 코어 — 명령·스케줄·사건 재생 계약

pub mod command;
pub mod demo;
pub mod error;
pub mod event;
pub mod model;
pub mod rng;
pub mod runtime;
pub mod save;
pub mod scheduler;

pub use command::{
    Command, CommandEnvelope, EXPOSE_DUPLICATE_PROMISE, SubmitSpec, execute_command, submit_command,
};
pub use demo::{
    DemoResult, create_demo_checkpoint, create_demo_runtime, run_demo, run_demo_to_runtime,
    run_demo_via_checkpoint,
};
pub use error::CoreError;
pub use event::{ContributionClass, Event, InfluenceLink, RandomDraw, StateChange};
pub use model::{
    Candidate, House, Information, InformationVisibility, Player, SupportStatus, WorldState,
};
pub use rng::DeterministicRng;
pub use runtime::RuntimeState;
pub use save::{SAVE_SCHEMA_VERSION, SaveEnvelope, load_runtime_from_bytes, save_runtime_to_bytes};
pub use scheduler::{Phase, ScheduledKey, Scheduler, SchedulerSnapshot};
