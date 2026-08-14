// EPOCH 결정론 코어 — 명령·스케줄·사건 재생 계약

pub mod command;
pub mod context;
pub mod contextgen;
pub mod demo;
pub mod error;
pub mod event;
pub mod model;
pub mod political;
pub mod politicalgen;
pub mod population;
pub mod populationgen;
pub mod rights;
pub mod rightsgen;
pub mod rng;
pub mod runtime;
pub mod save;
pub mod scheduler;
pub mod world;
pub mod worldgen;

pub use command::{
    Command, CommandEnvelope, EXPOSE_DUPLICATE_PROMISE, SubmitSpec, execute_command, submit_command,
};
pub use context::{
    CONTEXT_WORLD_SCHEMA_VERSION, CULTURE_COUNT, ContextWorld, Culture, HouseIdentity,
    HouseRelation, HouseRelationKind, INFORMATION_COUNT, InformationConfidence, InformationItem,
    InformationScope, InformationTopic, InitialPoliticalContext, PROMISE_COUNT, PersonIdentity,
    Promise, RELATION_COUNT, RELIGION_COUNT, RealmIdentity, Religion,
};
pub use contextgen::{derive_initial_context, generate_context_world, validate_initial_context};
pub use demo::{
    DemoResult, create_demo_checkpoint, create_demo_runtime, run_demo, run_demo_to_runtime,
    run_demo_via_checkpoint,
};
pub use error::CoreError;
pub use event::{ContributionClass, Event, InfluenceLink, RandomDraw, StateChange};
pub use model::{
    Candidate, House, Information, InformationVisibility, Player, SupportStatus, WorldState,
};
pub use political::{
    ACTIVE_ACTOR_COUNT, ActivationReason, ActiveActor, ActiveRole, POLITICAL_WORLD_SCHEMA_VERSION,
    PoliticalRoster, PoliticalWorld, SUPPORTING_PERSON_COUNT,
};
pub use politicalgen::{
    derive_political_roster, generate_political_world, validate_political_roster,
};
pub use population::{
    DYNASTIC_WORLD_SCHEMA_VERSION, DynasticWorld, GenerationBand, HOUSE_COUNT, PERSON_COUNT,
    PopulationGenerationMeta, PopulationSkeleton, RulerPersonLink,
};
// population::House is available as epoch_core::population::House (avoids clash with model::House)
pub use populationgen::{generate_dynastic_world, generate_population, validate_population};
pub use rights::{
    ClaimBasis, ClaimStanding, InitialRights, REALM_RIGHTS_COUNT, RIGHT_EVIDENCE_COUNT,
    RIGHTS_WORLD_SCHEMA_VERSION, RealmRights, RightEvidenceKind, RightEvidenceRecord, RightsWorld,
    SUCCESSION_CLAIM_COUNT, SuccessionClaim,
};
pub use rightsgen::{derive_initial_rights, generate_rights_world, validate_initial_rights};
pub use rng::DeterministicRng;
pub use runtime::RuntimeState;
pub use save::{SAVE_SCHEMA_VERSION, SaveEnvelope, load_runtime_from_bytes, save_runtime_to_bytes};
pub use scheduler::{Phase, ScheduledKey, Scheduler, SchedulerSnapshot};
pub use world::{
    GenerationMeta, Realm, Ruler, Territory, WORLD_HEIGHT, WORLD_REALM_COUNT, WORLD_RULER_COUNT,
    WORLD_SCHEMA_VERSION, WORLD_TERRITORY_COUNT, WORLD_WIDTH, WorldSkeleton,
};
pub use worldgen::{generate_world, validate_world};
