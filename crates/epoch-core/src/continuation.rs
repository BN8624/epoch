// M2.5 후속 세대 출생·파생 권리 continuation 계층

use crate::claim_propagation::ClaimPropagationWorld;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};

/// GenerationContinuationWorld 스키마 버전 (M2.5 next-generation layer).
pub const GENERATION_CONTINUATION_WORLD_SCHEMA_VERSION: u32 = 1;
/// 후속 출생 기록 수 (국가당 1).
pub const BIRTH_COUNT: usize = 6;
/// 신생아 수 (국가당 1).
pub const NEWBORN_COUNT: usize = 6;
/// 다음 세대 파생 claim 수 (국가당 1).
pub const NEXT_GENERATION_CLAIM_COUNT: usize = 6;

/// Marriage B에서 결정론적으로 만든 후속 출생.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BirthRecord {
    pub id: String,
    pub realm_id: String,
    pub marriage_id: String,
    pub child_person_id: String,
    pub parent_person_ids: Vec<String>,
}

/// 기존 PopulationSkeleton에 넣지 않는 continuation 전용 신생아.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewbornPerson {
    pub id: String,
    pub name: String,
    pub realm_id: String,
    pub house_id: String,
    pub home_territory_id: String,
    pub culture_id: String,
    pub religion_id: String,
}

/// 복권 원본 claim에서 신생아로 이어진 다음 세대 파생 권리.
///
/// basis / standing / evidence / score / priority는 복제하지 않는다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextGenerationClaim {
    pub id: String,
    pub realm_id: String,
    pub succession_target_key: String,
    pub claimant_person_id: String,
    pub claimant_house_id: String,
    pub source_claim_id: String,
    pub via_parent_person_id: String,
    pub generation_distance: u8,
}

/// M2.5 후속 출생·신생아·다음 세대 권리 집합.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationContinuation {
    pub births: Vec<BirthRecord>,
    pub newborns: Vec<NewbornPerson>,
    pub derived_claims: Vec<NextGenerationClaim>,
}

/// M2.2 권리 전파 세계 + M2.5 후속 세대 continuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationContinuationWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub base_world: ClaimPropagationWorld,
    pub continuation: GenerationContinuation,
}

impl GenerationContinuationWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
