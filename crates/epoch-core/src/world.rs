// 시드 기반 세계 골격 — 영지·국가·통치자 도메인 모델

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

/// 세계 골격 스키마 버전 (M1.1).
pub const WORLD_SCHEMA_VERSION: u32 = 1;

/// 고정 격자 가로 크기.
pub const WORLD_WIDTH: u8 = 6;
/// 고정 격자 세로 크기.
pub const WORLD_HEIGHT: u8 = 6;
/// 고정 영지 수 (6×6).
pub const WORLD_TERRITORY_COUNT: usize = 36;
/// 고정 국가 수.
pub const WORLD_REALM_COUNT: usize = 6;
/// 고정 통치자 수 (국가당 1).
pub const WORLD_RULER_COUNT: usize = 6;

/// 생성 provenance — 결정론 디버깅용 최소 메타.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationMeta {
    pub template_id: String,
    pub rng_draws: u64,
}

/// 6×6 격자 위 한 영지.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Territory {
    pub id: String,
    pub x: u8,
    pub y: u8,
    pub realm_id: String,
    pub neighbors: Vec<String>,
}

/// 국가 영역과 수도·통치자 참조.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Realm {
    pub id: String,
    pub name: String,
    pub capital_territory_id: String,
    pub ruler_id: String,
    pub territory_ids: Vec<String>,
}

/// 국가별 통치자 1명 (M1.1 최소 필드만).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruler {
    pub id: String,
    pub name: String,
    pub realm_id: String,
    pub seat_territory_id: String,
}

/// 시드에서 생성된 최소 세계 골격 (기존 M0 WorldState와 분리).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSkeleton {
    pub schema_version: u32,
    pub seed: u64,
    pub width: u8,
    pub height: u8,
    pub generation: GenerationMeta,
    pub territories: Vec<Territory>,
    pub realms: Vec<Realm>,
    pub rulers: Vec<Ruler>,
}

impl WorldSkeleton {
    /// 재생 비교용 compact JSON bytes.
    pub fn to_compact_json_bytes(&self) -> Result<Vec<u8>, CoreError> {
        serde_json::to_vec(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }

    /// 화면 표시용 pretty JSON.
    pub fn to_pretty_json(&self) -> Result<String, CoreError> {
        serde_json::to_string_pretty(self).map_err(|e| CoreError::Serialization(e.to_string()))
    }
}

/// 좌표 (x, y)에서 결정론 영지 ID를 만든다.
pub fn territory_id_at(x: u8, y: u8) -> String {
    let index = u16::from(y) * u16::from(WORLD_WIDTH) + u16::from(x);
    format!("territory-{index:02}")
}

/// 영지 인덱스(0..35)에서 좌표를 복원한다.
pub fn coords_from_index(index: usize) -> (u8, u8) {
    let x = (index % usize::from(WORLD_WIDTH)) as u8;
    let y = (index / usize::from(WORLD_WIDTH)) as u8;
    (x, y)
}
