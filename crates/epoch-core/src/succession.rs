// 통치자 사망 후 법적 우선 후계와 공석·3인 계승 주장

use crate::claim_propagation::ClaimPropagationWorld;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};

/// SuccessionWorld 스키마 버전 (M2.3 vacancy + presumptive successor).
pub const SUCCESSION_WORLD_SCHEMA_VERSION: u32 = 1;
/// 국가당 법적 후보 수.
pub const SUCCESSION_CANDIDATE_COUNT: usize = 3;

/// 계승 후보의 권리 출처. basis/standing은 복제하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionClaimOrigin {
    Original,
    Derived,
}

/// Phase 1 fixture 전용 최소 법적 우선순위. 숫자 점수가 아니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionPriority {
    DirectStrongOriginal,
    RestoredContestedOriginal,
    RestoredContestedDerived,
}

/// 통치자 사망. 이번 단계에서는 명시적 계산 입력이다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncumbentDeath {
    pub id: String,
    pub realm_id: String,
    pub person_id: String,
}

/// 사망 후 Realm 공석. 새 incumbent를 기록하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmVacancy {
    pub realm_id: String,
    pub former_incumbent_person_id: String,
    pub is_vacant: bool,
}

/// 한 명의 법적 계승 후보. basis/standing/evidence는 원본 claim에서 읽는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionCandidate {
    pub person_id: String,
    pub house_id: String,
    pub claim_record_id: String,
    pub claim_origin: SuccessionClaimOrigin,
    pub priority: SuccessionPriority,
    pub generation_distance: u8,
}

/// 한 Realm의 사망·공석·3인 후보·추정 후계자.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionTransition {
    pub realm_id: String,
    pub succession_target_key: String,
    pub death: IncumbentDeath,
    pub candidates: Vec<SuccessionCandidate>,
    pub presumptive_successor_person_id: String,
    pub presumptive_successor_house_id: String,
    pub vacancy: RealmVacancy,
}

/// 사망 이전 세계 provenance + 한 Realm의 계승 전환.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub pre_succession_world: ClaimPropagationWorld,
    pub transition: SuccessionTransition,
}

impl SuccessionWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
