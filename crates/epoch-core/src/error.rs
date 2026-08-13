// 코어 최소 오류 타입

use std::fmt;

/// 명령 검증·실행 오류. 외부 오류 crate 없이 표현한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    UnknownPlayer {
        id: String,
    },
    UnknownCandidate {
        id: String,
    },
    UnknownHouse {
        id: String,
    },
    UnknownInformation {
        id: String,
    },
    MissingCandidateForSupport {
        house_id: String,
    },
    UnexpectedCandidateForUndecided {
        house_id: String,
        candidate_id: String,
    },
    InvalidChanceBasisPoints {
        value: u32,
    },
    VisibilityRegression {
        from: String,
        to: String,
    },
    EventIdCollision {
        event_id: u64,
    },
    CommandSequenceCollision {
        sequence: u64,
    },
    EventIdOverflow,
    CommandSequenceOverflow,
    WorldTimeOverflow,
    InvalidActionCode {
        action_code: String,
    },
    Serialization(String),
    /// JSON 파싱 실패 또는 필수 필드 누락 등 저장본 디코드 오류.
    SaveDecode(String),
    /// 지원하지 않는 save schema_version.
    UnsupportedSaveSchema {
        version: u32,
    },
    /// 로드 직후 불변식 위반 (부분 상태 반환 금지).
    InvalidSaveInvariant(String),
    /// 세계 골격 생성·검증 불변식 위반 (fail closed).
    InvalidWorld(String),
    /// 인구·가계 골격 생성·검증 불변식 위반 (fail closed).
    InvalidPopulation(String),
    /// 정치 활동 계층 생성·검증 불변식 위반 (fail closed).
    InvalidPolitical(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::UnknownPlayer { id } => write!(f, "unknown player id: {id}"),
            CoreError::UnknownCandidate { id } => write!(f, "unknown candidate id: {id}"),
            CoreError::UnknownHouse { id } => write!(f, "unknown house id: {id}"),
            CoreError::UnknownInformation { id } => write!(f, "unknown information id: {id}"),
            CoreError::MissingCandidateForSupport { house_id } => {
                write!(
                    f,
                    "declared support requires candidate for house: {house_id}"
                )
            }
            CoreError::UnexpectedCandidateForUndecided {
                house_id,
                candidate_id,
            } => {
                write!(
                    f,
                    "undecided support cannot have candidate for house: {house_id} (candidate: {candidate_id})"
                )
            }
            CoreError::InvalidChanceBasisPoints { value } => {
                write!(f, "chance_basis_points out of range: {value} (max 10000)")
            }
            CoreError::VisibilityRegression { from, to } => {
                write!(f, "visibility regression forbidden: {from} -> {to}")
            }
            CoreError::EventIdCollision { event_id } => {
                write!(f, "event id collision: {event_id}")
            }
            CoreError::CommandSequenceCollision { sequence } => {
                write!(f, "command sequence collision: {sequence}")
            }
            CoreError::EventIdOverflow => write!(f, "event id counter overflow"),
            CoreError::CommandSequenceOverflow => {
                write!(f, "command sequence counter overflow")
            }
            CoreError::WorldTimeOverflow => write!(f, "world time overflow"),
            CoreError::InvalidActionCode { action_code } => {
                write!(f, "invalid action code: {action_code}")
            }
            CoreError::Serialization(msg) => write!(f, "serialization error: {msg}"),
            CoreError::SaveDecode(msg) => write!(f, "save decode error: {msg}"),
            CoreError::UnsupportedSaveSchema { version } => {
                write!(f, "unsupported save schema_version: {version}")
            }
            CoreError::InvalidSaveInvariant(msg) => {
                write!(f, "invalid save invariant: {msg}")
            }
            CoreError::InvalidWorld(msg) => write!(f, "invalid world: {msg}"),
            CoreError::InvalidPopulation(msg) => write!(f, "invalid population: {msg}"),
            CoreError::InvalidPolitical(msg) => write!(f, "invalid political: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}
