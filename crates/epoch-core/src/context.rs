// M1.4 초기 정치 맥락 — 문화·종교·관계·약속·정보 도메인 모델

use crate::error::CoreError;
use crate::political::PoliticalWorld;
use serde::{Deserialize, Serialize};

/// ContextWorld 스키마 버전 (M1.4 initial political context layer).
pub const CONTEXT_WORLD_SCHEMA_VERSION: u32 = 1;
/// 고정 문화 수.
pub const CULTURE_COUNT: usize = 3;
/// 고정 종교 수.
pub const RELIGION_COUNT: usize = 2;
/// 가문 관계 수 (intra 18 + cross 6).
pub const RELATION_COUNT: usize = 24;
/// 충돌 약속 수 (realm당 2 × 6).
pub const PROMISE_COUNT: usize = 12;
/// 정보 항목 수 (public 6 + private confirmed 6 + private unverified 6).
pub const INFORMATION_COUNT: usize = 18;
/// 국가당 가문 관계 수 (intra).
pub const INTRA_REALM_RELATION_COUNT: usize = 18;
/// 국가 간 가문 관계 수 (cross ruling ring).
pub const CROSS_REALM_RELATION_COUNT: usize = 6;
/// 공개 확정 정보 수.
pub const PUBLIC_CONFIRMED_INFORMATION_COUNT: usize = 6;
/// 비공개 확정 정보 수.
pub const PRIVATE_CONFIRMED_INFORMATION_COUNT: usize = 6;
/// 비공개 미검증 정보 수.
pub const PRIVATE_UNVERIFIED_INFORMATION_COUNT: usize = 6;

/// 고정 문화 ID.
pub const CULTURE_AMBER: &str = "culture-amber";
pub const CULTURE_RIVER: &str = "culture-river";
pub const CULTURE_STONE: &str = "culture-stone";

/// 고정 종교 ID.
pub const FAITH_SOLAR: &str = "faith-solar";
pub const FAITH_ANCESTRAL: &str = "faith-ancestral";

/// 문화 identity (단순 고정 fixture).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Culture {
    pub id: String,
    pub name: String,
}

/// 종교 identity (단순 고정 fixture).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Religion {
    pub id: String,
    pub name: String,
}

/// 국가 majority 문화·종교 프로필.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmIdentity {
    pub realm_id: String,
    pub majority_culture_id: String,
    pub majority_religion_id: String,
}

/// 가문 문화·종교 프로필.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseIdentity {
    pub house_id: String,
    pub culture_id: String,
    pub religion_id: String,
}

/// 인물 문화·종교 프로필 (House 상속).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonIdentity {
    pub person_id: String,
    pub culture_id: String,
    pub religion_id: String,
}

/// 가문 관계 유형 (점수 아님).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseRelationKind {
    Cooperative,
    Rival,
    Competitive,
}

/// undirected 가문 관계 (`house_a_id < house_b_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseRelation {
    pub house_a_id: String,
    pub house_b_id: String,
    pub kind: HouseRelationKind,
}

/// 충돌 가능한 희소 보상 약속.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promise {
    pub id: String,
    pub realm_id: String,
    pub promisor_person_id: String,
    pub promisee_person_id: String,
    pub reward_key: String,
    pub known_by_person_ids: Vec<String>,
}

/// 정보 공개 범위.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationScope {
    Public,
    Private,
}

/// 정보 신뢰도.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationConfidence {
    Confirmed,
    Unverified,
}

/// 정보 주제.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationTopic {
    ReligiousMinority,
    PromiseConflict,
}

/// 공개 범위·신뢰도를 가진 정보 항목.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InformationItem {
    pub id: String,
    pub realm_id: String,
    pub topic: InformationTopic,
    pub scope: InformationScope,
    pub confidence: InformationConfidence,
    pub subject_ids: Vec<String>,
    pub known_by_person_ids: Vec<String>,
}

/// M1.4 초기 정치 맥락 집합.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialPoliticalContext {
    pub cultures: Vec<Culture>,
    pub religions: Vec<Religion>,
    pub realm_identities: Vec<RealmIdentity>,
    pub house_identities: Vec<HouseIdentity>,
    pub person_identities: Vec<PersonIdentity>,
    pub relations: Vec<HouseRelation>,
    pub promises: Vec<Promise>,
    pub information: Vec<InformationItem>,
}

/// M1.3 정치 세계 + M1.4 초기 정치 맥락.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub political: PoliticalWorld,
    pub context: InitialPoliticalContext,
}

impl ContextWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
