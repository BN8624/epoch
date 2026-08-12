// M1.2 인구·가계 골격 — 가문·인물·통치자 연결 도메인 모델

use crate::error::CoreError;
use crate::world::WorldSkeleton;
use serde::{Deserialize, Serialize};

/// 왕조 세계 스키마 버전 (M1.2 population layer).
pub const DYNASTIC_WORLD_SCHEMA_VERSION: u32 = 1;
/// 고정 가문 수.
pub const HOUSE_COUNT: usize = 18;
/// 고정 인물 수.
pub const PERSON_COUNT: usize = 144;
/// 국가당 가문 수.
pub const HOUSES_PER_REALM: usize = 3;
/// 가문당 인물 수.
pub const PERSONS_PER_HOUSE: usize = 8;
/// 가문당 Elder 수.
pub const ELDERS_PER_HOUSE: usize = 2;
/// 가문당 Current 수.
pub const CURRENTS_PER_HOUSE: usize = 3;
/// 가문당 Young 수.
pub const YOUNGS_PER_HOUSE: usize = 3;

/// 3세대 대역 (serde: elder / current / young).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationBand {
    Elder,
    Current,
    Young,
}

/// 인구 생성 provenance — 결정론 디버깅용 최소 메타.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationGenerationMeta {
    pub rng_draws: u64,
}

/// M1.2 가문 (M0 model::House와 별개 population layer 타입).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct House {
    pub id: String,
    pub name: String,
    pub realm_id: String,
    pub seat_territory_id: String,
    pub head_person_id: String,
    pub member_ids: Vec<String>,
}

/// M1.2 인물 identity (최소 계보 필드만).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub house_id: String,
    pub realm_id: String,
    pub home_territory_id: String,
    pub generation: GenerationBand,
    pub known_parent_ids: Vec<String>,
}

/// 기존 M1.1 Ruler와 Person head 연결.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RulerPersonLink {
    pub ruler_id: String,
    pub person_id: String,
}

/// 가문·인물·통치자 연결 골격.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationSkeleton {
    pub houses: Vec<House>,
    pub persons: Vec<Person>,
    pub ruler_links: Vec<RulerPersonLink>,
    pub generation: PopulationGenerationMeta,
}

/// M1.1 세계 골격 + M1.2 인구 계층.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynasticWorld {
    pub schema_version: u32,
    pub seed: u64,
    pub world: WorldSkeleton,
    pub population: PopulationSkeleton,
}

impl DynasticWorld {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

/// 가문 인덱스(0..17)에서 안정적 house ID를 만든다.
pub fn house_id_at(index: usize) -> String {
    format!("house-{:02}", index + 1)
}

/// 인물 전역 인덱스(0..143)에서 안정적 person ID를 만든다.
pub fn person_id_at(index: usize) -> String {
    format!("person-{:03}", index + 1)
}

/// 가문 내 member index(0..7)의 세대 대역.
pub fn generation_for_member(member_index: usize) -> GenerationBand {
    match member_index {
        0 | 1 => GenerationBand::Elder,
        2..=4 => GenerationBand::Current,
        5..=7 => GenerationBand::Young,
        _ => GenerationBand::Elder,
    }
}
