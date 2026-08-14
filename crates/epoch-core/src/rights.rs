// M1.5 초기 계승 권리 — 직계·복권 권리 도메인 모델

use crate::context::ContextWorld;
use crate::error::CoreError;
use serde::{Deserialize, Serialize};

/// RightsWorld 스키마 버전 (M1.5 initial succession-rights layer).
pub const RIGHTS_WORLD_SCHEMA_VERSION: u32 = 1;
/// 국가별 권리 프로필 수.
pub const REALM_RIGHTS_COUNT: usize = 6;
/// 계승 claim 수 (국가당 2).
pub const SUCCESSION_CLAIM_COUNT: usize = 12;
/// 역사 기록 수 (국가당 1).
pub const RIGHT_EVIDENCE_COUNT: usize = 6;

/// 권리 근거. 초기 fixture는 이 두 종류만 사용한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimBasis {
    DirectDescent,
    RestoredLineRecord,
}

/// 초기 fixture의 권리 설명. 후계 순위가 아니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStanding {
    Strong,
    Contested,
}

/// 역사 기록 종류. 복권 권리만 별도 기록을 둔다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightEvidenceKind {
    RestoredLineage,
}

/// 한 Realm의 통치권 대상과 현재 통치자·claim 목록.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmRights {
    pub realm_id: String,
    pub succession_target_key: String,
    pub incumbent_person_id: String,
    pub claim_ids: Vec<String>,
}

/// 한 인물의 계승 권리 주장.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionClaim {
    pub id: String,
    pub realm_id: String,
    pub succession_target_key: String,
    pub claimant_person_id: String,
    pub claimant_house_id: String,
    pub basis: ClaimBasis,
    pub standing: ClaimStanding,
    pub evidence_record_ids: Vec<String>,
}

/// 복권 권리를 뒷받침하는 역사 기록.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightEvidenceRecord {
    pub id: String,
    pub realm_id: String,
    pub house_id: String,
    pub kind: RightEvidenceKind,
}

/// M1.5 초기 계승 권리 집합.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialRights {
    pub realms: Vec<RealmRights>,
    pub claims: Vec<SuccessionClaim>,
    pub evidence_records: Vec<RightEvidenceRecord>,
}

/// M1.4 맥락 세계 + M1.5 초기 계승 권리.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub context_world: ContextWorld,
    pub rights: InitialRights,
}

impl RightsWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

/// Realm 통치권을 가리키는 안정적 식별자. 아직 Title 객체가 아니다.
pub fn succession_target_key(realm_id: &str) -> String {
    format!("succession:{realm_id}")
}
