// 사건 기록, 상태 변화, 인과·영향 링크

use serde::{Deserialize, Serialize};

/// 구조화된 상태 변화 (before/after).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateChange {
    pub entity_id: String,
    pub field: String,
    pub before: String,
    pub after: String,
}

/// RNG 추첨 기록.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomDraw {
    pub stream: String,
    pub draw_index: u64,
    pub raw_value: u64,
    pub chance_basis_points: u32,
    pub success: bool,
}

/// 영향 기여 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionClass {
    Direct,
    Mediated,
}

/// 인과 영향 링크 (direct / mediated만).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InfluenceLink {
    pub source_event: u64,
    pub path_length: u64,
    pub contribution_class: ContributionClass,
    pub top_contributors: Vec<u64>,
}

/// 최소 사건 기록.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: u64,
    pub world_time: u64,
    pub event_type: String,
    pub actors: Vec<String>,
    pub targets: Vec<String>,
    pub reason_codes: Vec<String>,
    pub random_draws: Vec<RandomDraw>,
    pub state_changes: Vec<StateChange>,
    /// 직접 원인 사건. 없으면 None.
    pub caused_by: Option<u64>,
    pub influence_links: Vec<InfluenceLink>,
    pub summary: String,
}

impl Event {
    pub fn player_action(
        event_id: u64,
        world_time: u64,
        actor: &str,
        _action_code: &str,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            event_id,
            world_time,
            event_type: "player_action_recorded".to_string(),
            actors: vec![actor.to_string()],
            targets: vec![],
            reason_codes: vec!["PLAYER_EXPOSED_DUPLICATE_PROMISE".to_string()],
            random_draws: vec![],
            state_changes: vec![],
            caused_by: None,
            influence_links: vec![],
            summary: summary.into(),
        }
    }

    pub fn with_direct_cause(mut self, cause_event_id: u64) -> Self {
        self.caused_by = Some(cause_event_id);
        self.influence_links = vec![InfluenceLink {
            source_event: cause_event_id,
            path_length: 1,
            contribution_class: ContributionClass::Direct,
            top_contributors: vec![cause_event_id],
        }];
        self
    }

    pub fn with_mediated_cause(
        mut self,
        caused_by: u64,
        root_source: u64,
        intermediate: u64,
    ) -> Self {
        self.caused_by = Some(caused_by);
        self.influence_links = vec![InfluenceLink {
            source_event: root_source,
            path_length: 2,
            contribution_class: ContributionClass::Mediated,
            top_contributors: vec![root_source, intermediate],
        }];
        self
    }
}

/// reason code 상수.
pub mod reason {
    pub const PLAYER_EXPOSED_DUPLICATE_PROMISE: &str = "PLAYER_EXPOSED_DUPLICATE_PROMISE";
    pub const HOUSE_RECONSIDERED_DUPLICATE_PROMISE: &str = "HOUSE_RECONSIDERED_DUPLICATE_PROMISE";
    pub const INFORMATION_DISCLOSED: &str = "INFORMATION_DISCLOSED";
    pub const INFORMATION_VERIFICATION_DRAW: &str = "INFORMATION_VERIFICATION_DRAW";
}
