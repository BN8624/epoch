// M2.2 1세대 권리 전파 — 부모 Claim에서 자녀 파생 권리

use crate::error::CoreError;
use crate::family::FamilyWorld;
use serde::{Deserialize, Serialize};

/// ClaimPropagationWorld 스키마 버전 (M2.2 initial derived-claim layer).
pub const CLAIM_PROPAGATION_WORLD_SCHEMA_VERSION: u32 = 1;
/// 초기 파생 claim 수 (국가당 1).
pub const DERIVED_CLAIM_COUNT: usize = 6;
/// 이번 단계가 허용하는 유일한 세대 거리.
pub const DERIVED_GENERATION_DISTANCE: u8 = 1;

/// 원본 SuccessionClaim에서 한 세대 아래로 파생된 권리 기록.
///
/// basis / standing / evidence는 복제하지 않는다. 법적 의미는
/// `source_claim_id`가 가리키는 원본 claim에서 읽는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedSuccessionClaim {
    pub id: String,
    pub realm_id: String,
    pub succession_target_key: String,
    pub claimant_person_id: String,
    pub claimant_house_id: String,
    pub source_claim_id: String,
    pub via_parent_person_id: String,
    pub generation_distance: u8,
}

/// M2.2 초기 파생 권리 집합.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialClaimPropagation {
    pub derived_claims: Vec<DerivedSuccessionClaim>,
}

/// M2.1 가족 세계 + M2.2 1세대 파생 권리.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimPropagationWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub family_world: FamilyWorld,
    pub propagation: InitialClaimPropagation,
}

impl ClaimPropagationWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
