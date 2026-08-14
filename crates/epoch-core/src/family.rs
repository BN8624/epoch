// M2.1 초기 혼인·혈통망 — 가문 간 배우자 관계와 양친 연결

use crate::error::CoreError;
use crate::rights::RightsWorld;
use serde::{Deserialize, Serialize};

/// FamilyWorld 스키마 버전 (M2.1 initial family layer).
pub const FAMILY_WORLD_SCHEMA_VERSION: u32 = 1;
/// 초기 혼인 기록 수 (국가당 2).
pub const MARRIAGE_COUNT: usize = 12;
/// 양친 parentage 수 (국가당 2).
pub const PARENTAGE_COUNT: usize = 12;

/// 같은 Realm 안의 서로 다른 두 House Current 배우자.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marriage {
    pub id: String,
    pub spouse_person_ids: Vec<String>,
    pub house_ids: Vec<String>,
    pub realm_ids: Vec<String>,
}

/// Family layer에서 확인되는 두 부모와 기존 Young 자녀 연결.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentageLink {
    pub id: String,
    pub marriage_id: String,
    pub child_person_id: String,
    pub parent_person_ids: Vec<String>,
}

/// M2.1 초기 혼인·양친 연결 집합.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialFamilyNetwork {
    pub marriages: Vec<Marriage>,
    pub parentages: Vec<ParentageLink>,
}

/// M1.5 권리 세계 + M2.1 초기 혼인·혈통망.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub rights_world: RightsWorld,
    pub family: InitialFamilyNetwork,
}

impl FamilyWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}
