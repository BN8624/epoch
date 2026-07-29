// 최소 세계 상태와 도메인 값 타입

use crate::rng::DeterministicRng;
use serde::{Deserialize, Serialize};

/// 가문 지지 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    Declared,
    Undecided,
}

/// 정보 공개 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationVisibility {
    Private,
    Unverified,
    PublicFact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub stance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct House {
    pub id: String,
    pub support_status: SupportStatus,
    /// declared일 때만 후보 ID. undecided/none이면 None.
    pub supported_candidate: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Information {
    pub id: String,
    pub visibility: InformationVisibility,
}

/// M0 실행 계약 증명용 최소 세계 상태.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldState {
    pub seed: u64,
    pub world_time: u64,
    pub rng: DeterministicRng,
    pub next_event_id: u64,
    pub next_command_sequence: u64,
    pub player: Player,
    pub candidates: Vec<Candidate>,
    pub houses: Vec<House>,
    pub information: Vec<Information>,
    pub events: Vec<crate::event::Event>,
}

impl WorldState {
    /// 고정 계승 분쟁 최소 초기 세계를 생성한다.
    pub fn new_initial(seed: u64) -> Self {
        Self {
            seed,
            world_time: 0,
            rng: DeterministicRng::new(seed),
            next_event_id: 1,
            next_command_sequence: 0,
            player: Player {
                id: "player-ren-arden".to_string(),
                stance: "house_darian_support".to_string(),
            },
            candidates: vec![
                Candidate {
                    id: "candidate-seria".to_string(),
                },
                Candidate {
                    id: "candidate-darian".to_string(),
                },
                Candidate {
                    id: "candidate-mireya".to_string(),
                },
            ],
            houses: vec![
                House {
                    id: "house-arden".to_string(),
                    support_status: SupportStatus::Declared,
                    supported_candidate: Some("candidate-darian".to_string()),
                },
                House {
                    id: "house-barren".to_string(),
                    support_status: SupportStatus::Declared,
                    supported_candidate: Some("candidate-seria".to_string()),
                },
                House {
                    id: "house-soren".to_string(),
                    support_status: SupportStatus::Declared,
                    supported_candidate: Some("candidate-darian".to_string()),
                },
                House {
                    id: "house-merova".to_string(),
                    support_status: SupportStatus::Declared,
                    supported_candidate: Some("candidate-mireya".to_string()),
                },
                House {
                    id: "house-halbeck".to_string(),
                    support_status: SupportStatus::Undecided,
                    supported_candidate: None,
                },
            ],
            information: vec![Information {
                id: "info-darian-duplicate-promise".to_string(),
                visibility: InformationVisibility::Private,
            }],
            events: Vec::new(),
        }
    }

    pub fn candidate_exists(&self, id: &str) -> bool {
        self.candidates.iter().any(|c| c.id == id)
    }

    pub fn house_index(&self, id: &str) -> Option<usize> {
        self.houses.iter().position(|h| h.id == id)
    }

    pub fn information_index(&self, id: &str) -> Option<usize> {
        self.information.iter().position(|i| i.id == id)
    }

    pub fn player_id_matches(&self, id: &str) -> bool {
        self.player.id == id
    }

    /// 특정 후보를 declared로 지지하는 가문 수.
    pub fn declared_support_count(&self, candidate_id: &str) -> usize {
        self.houses
            .iter()
            .filter(|h| {
                h.support_status == SupportStatus::Declared
                    && h.supported_candidate.as_deref() == Some(candidate_id)
            })
            .count()
    }

    pub fn allocate_event_id(&mut self) -> u64 {
        let id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);
        id
    }

    pub fn allocate_command_sequence(&mut self) -> u64 {
        let seq = self.next_command_sequence;
        self.next_command_sequence = self.next_command_sequence.saturating_add(1);
        seq
    }
}
