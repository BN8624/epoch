// 코어 최소 오류 타입

use std::fmt;

/// 명령 검증·실행 오류. 외부 오류 crate 없이 표현한다.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    UnknownPlayer { id: String },
    UnknownCandidate { id: String },
    UnknownHouse { id: String },
    UnknownInformation { id: String },
    MissingCandidateForSupport { house_id: String },
    InvalidChanceBasisPoints { value: u32 },
    VisibilityRegression { from: String, to: String },
    EventIdCollision { event_id: u64 },
    CommandSequenceCollision { sequence: u64 },
    EventIdOverflow,
    CommandSequenceOverflow,
    InvalidActionCode { action_code: String },
    Serialization(String),
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
            CoreError::InvalidActionCode { action_code } => {
                write!(f, "invalid action code: {action_code}")
            }
            CoreError::Serialization(msg) => write!(f, "serialization error: {msg}"),
        }
    }
}

impl std::error::Error for CoreError {}
