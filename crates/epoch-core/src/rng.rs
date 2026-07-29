// 시뮬레이션 전용 SplitMix64 결정론적 RNG

use serde::{Deserialize, Serialize};

/// 플랫폼·실행마다 동일 시드에서 같은 난수열을 내는 시뮬레이션용 RNG.
/// 암호학적 보안 용도가 아니다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicRng {
    seed: u64,
    state: u64,
    draws: u64,
}

impl DeterministicRng {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            state: seed,
            draws: 0,
        }
    }

    /// SplitMix64 다음 출력. 동일 시드에서 항상 같은 수열을 낸다.
    pub fn next_u64(&mut self) -> u64 {
        self.draws = self.draws.wrapping_add(1);
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    pub fn draws(&self) -> u64 {
        self.draws
    }

    pub fn state(&self) -> u64 {
        self.state
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// basis points(0..=10000) 확률 판정. 원시 추첨값과 성공 여부를 반환한다.
    pub fn roll_basis_points(&mut self, chance_basis_points: u32) -> (u64, bool) {
        let raw = self.next_u64();
        let success = (raw % 10_000) < u64::from(chance_basis_points);
        (raw, success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_seed_0_known_vectors() {
        let mut rng = DeterministicRng::new(0);
        assert_eq!(rng.next_u64(), 0xe220_a839_7b1d_cdaf);
        assert_eq!(rng.next_u64(), 0x6e78_9e6a_a1b9_65f4);
        assert_eq!(rng.next_u64(), 0x06c4_5d18_8009_454f);
    }

    #[test]
    fn splitmix64_seed_1_known_vectors() {
        let mut rng = DeterministicRng::new(1);
        assert_eq!(rng.next_u64(), 0x910a_2dec_8902_5cc1);
        assert_eq!(rng.next_u64(), 0xbeeb_8da1_658e_ec67);
        assert_eq!(rng.next_u64(), 0xf893_a2ee_fb32_555e);
    }

    #[test]
    fn draw_count_increments() {
        let mut rng = DeterministicRng::new(42);
        assert_eq!(rng.draws(), 0);
        let _ = rng.next_u64();
        assert_eq!(rng.draws(), 1);
        let _ = rng.next_u64();
        assert_eq!(rng.draws(), 2);
    }
}
