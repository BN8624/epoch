// M1.3 정치 활동 계층 — Active 24 / Supporting 120 도메인 모델

use crate::error::CoreError;
use crate::population::DynasticWorld;
use serde::{Deserialize, Serialize};

/// 정치 세계 스키마 버전 (M1.3 political-activity layer).
pub const POLITICAL_WORLD_SCHEMA_VERSION: u32 = 1;
/// 적극적 정치 행위자 수.
pub const ACTIVE_ACTOR_COUNT: usize = 24;
/// 보조 인물 수.
pub const SUPPORTING_PERSON_COUNT: usize = 120;
/// 국가당 Active 수.
pub const ACTIVE_PER_REALM: usize = 4;
/// 국가당 Supporting 수.
pub const SUPPORTING_PER_REALM: usize = 20;
/// Active 중 Ruler 수.
pub const RULER_ACTIVE_COUNT: usize = 6;
/// Active 중 non-ruling HouseHead 수.
pub const HOUSE_HEAD_ACTIVE_COUNT: usize = 12;
/// Active 중 RulingHouseCurrent 수.
pub const RULING_HOUSE_CURRENT_ACTIVE_COUNT: usize = 6;

/// Active 인물의 주 역할 (우선순위: Ruler > HouseHead > RulingHouseCurrent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveRole {
    Ruler,
    HouseHead,
    RulingHouseCurrent,
}

/// Active로 선정된 이유 (한 인물이 여러 이유를 가질 수 있음).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationReason {
    Ruler,
    HouseHead,
    RulingHouseCurrent,
}

/// 적극적 정치 행위자 한 명.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveActor {
    pub person_id: String,
    pub realm_id: String,
    pub primary_role: ActiveRole,
    pub activation_reasons: Vec<ActivationReason>,
}

/// Active / Supporting 분리 명부.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoliticalRoster {
    pub active_actors: Vec<ActiveActor>,
    pub supporting_person_ids: Vec<String>,
}

/// M1.2 왕조 세계 + M1.3 정치 활동 계층.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoliticalWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub dynastic: DynasticWorld,
    pub roster: PoliticalRoster,
}

impl PoliticalWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

/// activation_reasons 고정 정렬 순서 (Ruler → HouseHead → RulingHouseCurrent).
pub fn activation_reason_order(reason: ActivationReason) -> u8 {
    match reason {
        ActivationReason::Ruler => 0,
        ActivationReason::HouseHead => 1,
        ActivationReason::RulingHouseCurrent => 2,
    }
}

/// primary_role 우선순위 (낮을수록 우선).
pub fn primary_role_priority(role: ActiveRole) -> u8 {
    match role {
        ActiveRole::Ruler => 0,
        ActiveRole::HouseHead => 1,
        ActiveRole::RulingHouseCurrent => 2,
    }
}
