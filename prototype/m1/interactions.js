// 플레이어 제안·행동·고정 결과 — 시나리오 fixture와 책임 분리
/**
 * EPOCH M-1.2 고정 선택·결과 fixture.
 * 범용 분기 엔진이 아니라 결정론적 결과 테이블만 제공한다.
 */

import { scenario } from './scenario.js';

/** @typedef {'review'|'decision'|'resolved'} SessionPhase */

/** NPC 상충 제안 3개 (검토용 카드) */
export const PROPOSALS = [
  {
    id: 'proposal-arden-order',
    proposer: '아르덴 가문 수장',
    demand: '다리안 코르벤을 공개 지지하라.',
    benefit: '가문 보호와 향후 동부 변경백 직위 배분에서 렌의 몫을 보장한다.',
    risk: '명령을 거부하면 가문 내 지위와 상속 기대가 약해진다.',
    visibility: /** @type {'public_fact'} */ ('public_fact'),
    relatedCandidateId: 'candidate-darian',
    relatedHouseId: 'house-arden',
    relatedLabel: '다리안 코르벤 · 아르덴 가문',
  },
  {
    id: 'proposal-seria-marriage',
    proposer: '세리아 아르케온 측 사절',
    demand: '세리아를 지지하고 아르덴 가문 내부 정보를 제공하라.',
    benefit: '세리아의 가까운 친족과 혼인해 왕실과 직접 연결될 기회를 제공한다.',
    risk: '발각되면 아르덴 가문에 대한 배신으로 간주된다.',
    visibility: /** @type {'private'} */ ('private'),
    relatedCandidateId: 'candidate-seria',
    relatedHouseId: 'house-barren',
    relatedLabel: '세리아 아르케온 · 바렌 가문',
  },
  {
    id: 'proposal-mireya-debt',
    proposer: '미레아 셀칸 측 사절',
    demand: '왕실 기록관 보좌 권한을 이용해 알레시아 계통의 권리 기록 사본을 넘겨라.',
    benefit: '미레아가 승리하면 과거 보호의 빚을 갚은 인물로 인정하고 독립된 직위를 보장한다.',
    risk: '기록 반출이 드러나면 반역 조사와 직위 박탈 위험이 있다.',
    visibility: /** @type {'private'} */ ('private'),
    relatedCandidateId: 'candidate-mireya',
    relatedHouseId: 'house-merova',
    relatedLabel: '미레아 셀칸 · 메로바 가문',
  },
];

/** 플레이어 행동 5개 — 선택 전 표시 정보 포함 */
export const ACTIONS = [
  {
    id: 'action-a',
    code: 'A',
    label: '다리안을 공개 지지하고 직위 약속을 공표한다',
    helps: '아르덴 가문 수장 · 다리안 코르벤',
    benefits: [
      '아르덴 가문 수장의 신뢰 증가',
      '다리안의 신뢰 증가',
      '향후 직위 보상 기대',
    ],
    losses: [
      '세리아와 미레아의 신뢰 하락',
      '소렌 가문이 중복 약속을 의심하게 됨',
    ],
    risks: [
      '세리아 측과 미레아 측의 협력 기회 상실',
    ],
    affected: ['다리안 코르벤', '세리아 아르케온', '미레아 셀칸', '아르덴 가문', '소렌 가문'],
    responseProposalId: 'proposal-arden-order',
  },
  {
    id: 'action-b',
    code: 'B',
    label: '세리아와 비밀 혼인 동맹을 맺고 지지를 약속한다',
    helps: '세리아 아르케온 · 바렌 가문',
    benefits: [
      '세리아의 신뢰 증가',
      '왕실과의 직접 혼인 연결 가능성',
      '바렌 가문의 우호 증가',
    ],
    losses: [
      '다리안의 신뢰 하락',
      '아르덴 가문 수장의 신뢰 하락',
    ],
    risks: [
      '비밀이 드러날 경우 배신자로 규정될 위험',
    ],
    affected: ['세리아 아르케온', '다리안 코르벤', '아르덴 가문', '바렌 가문', '할베크 가문'],
    responseProposalId: 'proposal-seria-marriage',
  },
  {
    id: 'action-c',
    code: 'C',
    label: '미레아에게 알레시아 계통의 권리 기록 사본을 제공한다',
    helps: '미레아 셀칸 · 메로바 가문',
    benefits: [
      '미레아의 권리 주장 신뢰도 증가',
      '미레아와 메로바 가문의 신뢰 증가',
      '오래된 보호의 빚을 갚음',
    ],
    losses: [
      '세리아와 다리안 양측의 경계 증가',
    ],
    risks: [
      '기록 반출이 드러날 경우 반역 조사',
      '왕실 기록관 보좌 직위 상실 위험',
    ],
    affected: ['미레아 셀칸', '세리아 아르케온', '다리안 코르벤', '메로바 가문', '할베크 가문'],
    responseProposalId: 'proposal-mireya-debt',
  },
  {
    id: 'action-d',
    code: 'D',
    label: '다리안이 같은 핵심 직위를 중복 약속했다는 정보를 세리아 측에 넘긴다',
    helps: '세리아 아르케온 (정보 협력)',
    benefits: [
      '세리아의 신뢰 증가',
      '다리안 지지 연합에 균열 발생',
      '플레이어가 정보 중개자로서 영향력을 얻음',
    ],
    losses: [
      '다리안의 적대 증가',
      '아르덴 가문 수장의 신뢰 하락',
    ],
    risks: [
      '정보 출처가 드러날 경우 보복 위험',
    ],
    affected: ['세리아 아르케온', '다리안 코르벤', '아르덴 가문', '소렌 가문'],
    responseProposalId: 'proposal-seria-marriage',
  },
  {
    id: 'action-e',
    code: 'E',
    label: '세 진영의 요구를 모두 거절하고 결정을 미룬다',
    helps: '어느 진영도 돕지 않음 (중립)',
    benefits: [
      '아직 어느 진영에도 완전히 묶이지 않음',
      '후속 협상 가능성을 유지함',
    ],
    losses: [
      '세 후보 진영의 신뢰가 모두 소폭 하락',
      '아르덴 가문 수장이 최후통첩을 보냄',
    ],
    risks: [
      '기회주의자로 평가될 위험 증가',
    ],
    affected: ['세리아 아르케온', '다리안 코르벤', '미레아 셀칸', '아르덴 가문'],
    responseProposalId: null,
  },
];

/**
 * 행동별 고정 결과 fixture.
 * 화면용 한국어 문장만 포함하며 원시 점수는 없다.
 */
export const OUTCOMES = {
  'action-a': {
    actionId: 'action-a',
    chosenLabel: '다리안을 공개 지지하고 직위 약속을 공표한다',
    responseTo: '아르덴 가문 수장의 명령',
    helpedOrRefused: '다리안 코르벤을 돕고, 세리아·미레아 진영의 요구는 거부했습니다.',
    directChanges: {
      playerStance: '다리안 공개 지지',
      relationChanges: [
        '아르덴 가문 신뢰: 증가',
        '다리안 신뢰: 증가',
        '세리아 신뢰: 하락',
        '미레아 신뢰: 하락',
      ],
      benefitsGained: [
        '가문 수장의 신뢰를 얻었습니다.',
        '다리안의 신뢰가 증가했고 향후 직위 보상 기대가 생겼습니다.',
      ],
      risksCreated: [
        '세리아 측과 미레아 측의 협력 기회를 잃었습니다.',
      ],
    },
    ripples: [
      '동부 변경백 약속이 공개됩니다.',
      '소렌 가문은 다리안 지지에서 동요 상태로 바뀝니다.',
    ],
    reasons: [
      '렌이 가문 명령을 따라 다리안 지지를 공개하자 아르덴 가문 수장과 다리안의 신뢰가 함께 올랐습니다.',
      '공개 선언은 세리아·미레아 측과의 비밀 협상 여지를 닫았습니다.',
      '소렌 가문은 자신에게 약속된 직위가 아르덴 가문에도 약속됐다고 판단해 다리안 지지를 재검토했습니다.',
    ],
    unchanged: [
      '세리아의 법적 계승 순위는 바뀌지 않았습니다.',
      '메로바 가문의 미레아 지지는 유지됩니다.',
      '할베크 가문의 미결정 입장은 유지됩니다.',
    ],
    worldPatch: {
      playerStance: '다리안 공개 지지',
      playerStanceText: '현재 입장: 다리안 공개 지지',
      houseOverrides: {
        'house-soren': {
          supportStatus: 'wavering',
          supportCandidateId: 'candidate-darian',
          supportStatusLabel: '동요',
        },
      },
      candidateOverrides: {},
      newPublicInfo: [
        {
          text: '다리안이 아르덴 가문에 동부 변경백 직위를 약속했다는 사실이 공개되었습니다.',
          visibility: 'public_fact',
        },
      ],
    },
  },
  'action-b': {
    actionId: 'action-b',
    chosenLabel: '세리아와 비밀 혼인 동맹을 맺고 지지를 약속한다',
    responseTo: '세리아 아르케온 측의 비밀 혼인 제안',
    helpedOrRefused: '세리아를 비밀리에 돕고, 아르덴 가문 수장의 명령을 거역했습니다.',
    directChanges: {
      playerStance: '세리아 비밀 지지',
      relationChanges: [
        '세리아 신뢰: 크게 증가',
        '바렌 가문 우호: 증가',
        '아르덴 가문 신뢰: 하락',
        '다리안 신뢰: 하락',
      ],
      benefitsGained: [
        '세리아와의 비밀 혼인 동맹이 성립했습니다.',
        '왕실과 직접 연결될 가능성이 열렸습니다.',
      ],
      risksCreated: [
        '가문 배신 발각 가능성이 생겼습니다.',
      ],
    },
    ripples: [
      '세리아 측 혼인 동맹이 성립합니다.',
      '할베크 가문은 미결정에서 세리아 쪽으로 기울음 상태가 됩니다.',
    ],
    reasons: [
      '비밀 혼인 동맹은 세리아와 바렌 가문에 렌이 유용한 내부 동맹임을 증명했습니다.',
      '가문 명령을 어긴 사실이 드러나면 아르덴 가문은 렌을 배신자로 규정할 수 있습니다.',
      '할베크 가문은 왕실과 아르덴 가문의 내부 분열을 보고 세리아 쪽으로 기울기 시작했습니다.',
    ],
    unchanged: [
      '소렌 가문의 다리안 공개 지지는 유지됩니다.',
      '미레아의 권리 근거 문구는 바뀌지 않았습니다.',
      '메로바 가문의 미레아 지지는 유지됩니다.',
    ],
    worldPatch: {
      playerStance: '세리아 비밀 지지',
      playerStanceText: '현재 입장: 세리아 비밀 지지',
      houseOverrides: {
        'house-halbeck': {
          supportStatus: 'leaning',
          supportCandidateId: 'candidate-seria',
          supportStatusLabel: '세리아 쪽으로 기울음',
        },
      },
      candidateOverrides: {},
      newPublicInfo: [
        {
          text: '세리아 측과 렌 사이의 비밀 혼인 동맹이 성립했습니다. (비공개)',
          visibility: 'private',
        },
      ],
    },
  },
  'action-c': {
    actionId: 'action-c',
    chosenLabel: '미레아에게 알레시아 계통의 권리 기록 사본을 제공한다',
    responseTo: '미레아 셀칸 측의 오래된 빚 요구',
    helpedOrRefused: '미레아를 비밀 협력으로 돕고, 기록 반출 위험을 감수했습니다.',
    directChanges: {
      playerStance: '미레아 비밀 협력',
      relationChanges: [
        '미레아 신뢰: 증가',
        '메로바 가문 신뢰: 증가',
        '세리아 경계: 증가',
        '다리안 경계: 증가',
      ],
      benefitsGained: [
        '오래된 보호의 빚을 갚았습니다.',
        '미레아의 권리 주장 신뢰도가 높아졌습니다.',
      ],
      risksCreated: [
        '반역 조사 및 직위 박탈 가능성이 생겼습니다.',
      ],
    },
    ripples: [
      '알레시아 계통의 권리 기록이 공개 가능한 증거가 됩니다.',
      '미레아의 권리 표시는 논쟁 중인 오래된 왕통에서 기록 증거를 확보한 오래된 왕통으로 바뀝니다.',
      '할베크 가문은 미결정에서 미레아 쪽으로 기울음 상태가 됩니다.',
    ],
    reasons: [
      '기록관 보좌 권한으로 넘긴 사본이 미레아 권리의 실질 증거가 되었습니다.',
      '세리아와 다리안 양측은 기록 반출 가능성을 경계하기 시작했습니다.',
      '할베크 가문은 미레아 측이 증거를 확보한 것을 보고 미레아 쪽으로 기울기 시작했습니다.',
    ],
    unchanged: [
      '아르덴·소렌 가문의 다리안 공개 지지는 유지됩니다.',
      '바렌 가문의 세리아 지지는 유지됩니다.',
      '세리아의 법적 계승 순위는 바뀌지 않았습니다.',
    ],
    worldPatch: {
      playerStance: '미레아 비밀 협력',
      playerStanceText: '현재 입장: 미레아 비밀 협력',
      houseOverrides: {
        'house-halbeck': {
          supportStatus: 'leaning',
          supportCandidateId: 'candidate-mireya',
          supportStatusLabel: '미레아 쪽으로 기울음',
        },
      },
      candidateOverrides: {
        'candidate-mireya': {
          claimStrengthText: '기록 증거를 확보한 오래된 왕통',
          claimBasis:
            '현 왕조보다 오래된 장자 계통의 혈통 권리를 주장한다. 알레시아 계통 권리 기록 사본이 확보되어 증거 기반이 강화되었다.',
        },
      },
      newPublicInfo: [
        {
          text: '알레시아 계통의 권리 기록 사본이 미레아 측에 전달되어 공개 가능한 증거가 되었습니다.',
          visibility: 'unverified',
        },
      ],
    },
  },
  'action-d': {
    actionId: 'action-d',
    chosenLabel: '다리안이 같은 핵심 직위를 중복 약속했다는 정보를 세리아 측에 넘긴다',
    responseTo: '세리아 측과의 정보 협력 (가문 명령은 거부)',
    helpedOrRefused: '세리아 측에 정보를 넘기고, 다리안·아르덴 가문과는 적대 관계를 키웠습니다.',
    directChanges: {
      playerStance: '세리아 측 정보 협력',
      relationChanges: [
        '세리아 신뢰: 증가',
        '다리안 적대: 크게 증가',
        '아르덴 가문 신뢰: 하락',
      ],
      benefitsGained: [
        '세리아의 신뢰가 증가했습니다.',
        '정보 중개자로서 영향력을 얻었습니다.',
      ],
      risksCreated: [
        '보복과 정보원 노출 위험이 생겼습니다.',
      ],
    },
    ripples: [
      '다리안의 중복 직위 약속이 확인되지 않은 정보로 공개됩니다.',
      '소렌 가문은 다리안 지지에서 미결정으로 바뀝니다.',
      '다리안의 공개 지지 가문 수가 2곳에서 1곳으로 감소합니다.',
    ],
    reasons: [
      '중복 약속 정보가 세리아 측에 전달되면서 다리안 지지 연합에 균열이 생겼습니다.',
      '소렌 가문은 같은 직위를 다른 가문에도 약속받았다는 의심이 커져 공개 지지를 철회하고 미결정으로 돌아갔습니다.',
      '아르덴 가문 수장은 가문 명령을 어긴 렌을 신뢰하지 않게 되었습니다.',
    ],
    unchanged: [
      '아르덴 가문의 다리안 공개 지지는 유지됩니다.',
      '메로바 가문의 미레아 지지는 유지됩니다.',
      '할베크 가문의 미결정 입장은 유지됩니다.',
    ],
    worldPatch: {
      playerStance: '세리아 측 정보 협력',
      playerStanceText: '현재 입장: 세리아 측 정보 협력',
      houseOverrides: {
        'house-soren': {
          supportStatus: 'undecided',
          supportCandidateId: null,
          supportStatusLabel: '미결정',
        },
      },
      candidateOverrides: {},
      newPublicInfo: [
        {
          text: '다리안이 같은 핵심 직위를 여러 지지 가문에 중복 약속했다는 정보가 확인되지 않은 상태로 퍼지고 있습니다.',
          visibility: 'unverified',
        },
      ],
    },
  },
  'action-e': {
    actionId: 'action-e',
    chosenLabel: '세 진영의 요구를 모두 거절하고 결정을 미룬다',
    responseTo: '세 진영의 상충하는 제안 전부',
    helpedOrRefused: '세 후보 진영의 요구를 모두 거절하고 결정을 미뤘습니다.',
    directChanges: {
      playerStance: '중립',
      relationChanges: [
        '세리아 진영 신뢰: 소폭 하락',
        '다리안 진영 신뢰: 소폭 하락',
        '미레아 진영 신뢰: 소폭 하락',
      ],
      benefitsGained: [
        '아직 어느 진영에도 완전히 묶이지 않았습니다.',
        '후속 협상 가능성이 유지됩니다.',
      ],
      risksCreated: [
        '모든 진영으로부터 기회주의자로 의심받습니다.',
      ],
    },
    ripples: [
      '가문 수장이 공개 지지를 요구하는 최후통첩을 보냅니다.',
      '후보와 가문의 초기 공개 지지 구도는 바뀌지 않습니다.',
      '후속 선택을 하지 않는 이상 플레이어의 영향력도 증가하지 않습니다.',
    ],
    reasons: [
      '모든 제안을 거절한 렌은 어느 진영에도 확약하지 않아 협상 여지를 남겼습니다.',
      '동시에 세 진영은 렌을 기회주의자로 보기 시작했습니다.',
      '아르덴 가문 수장은 공개 지지를 더 이상 미룰 수 없다며 최후통첩을 보냈습니다.',
    ],
    unchanged: [
      '후보 세 명의 공개 지지 가문 구도는 초기와 같습니다.',
      '할베크 가문의 미결정 입장은 유지됩니다.',
      '미레아의 권리 근거 문구는 바뀌지 않았습니다.',
    ],
    worldPatch: {
      playerStance: '중립',
      playerStanceText: '현재 입장: 중립 (결정을 미룸)',
      houseOverrides: {},
      candidateOverrides: {},
      newPublicInfo: [
        {
          text: '아르덴 가문 수장이 렌에게 다리안 공개 지지를 요구하는 최후통첩을 보냈습니다.',
          visibility: 'public_fact',
        },
      ],
    },
  },
};

/**
 * 깊은 복사 (JSON 직렬화 가능한 fixture 전용).
 * @template T
 * @param {T} value
 * @returns {T}
 */
export function deepClone(value) {
  return JSON.parse(JSON.stringify(value));
}

/**
 * 시나리오 초기 fixture에서 런타임 월드 스냅샷을 만든다.
 * 원본 scenario 객체를 변형하지 않는다.
 */
export function createInitialWorld() {
  return {
    houses: scenario.houses.map((h) => ({
      id: h.id,
      name: h.name,
      supportCandidateId: h.supportCandidateId,
      supportStatus: h.supportStatus,
      supportStatusLabel: null,
    })),
    candidates: scenario.candidates.map((c) => ({
      id: c.id,
      name: c.name,
      claimStrengthText: c.claimStrengthText,
      claimBasis: c.claimBasis,
    })),
    player: {
      stance: null,
      stanceText: scenario.player.houseStanceText,
    },
    newPublicInfo: [],
  };
}

/**
 * 월드에서 후보를 공개 지지하는 가문 목록.
 * @param {ReturnType<typeof createInitialWorld>} world
 * @param {string} candidateId
 */
export function getWorldSupportingHouses(world, candidateId) {
  return world.houses.filter(
    (h) => h.supportStatus === 'declared' && h.supportCandidateId === candidateId,
  );
}

/**
 * 월드 기준 가문 입장 라벨.
 * @param {ReturnType<typeof createInitialWorld>} world
 * @param {string} houseId
 */
export function getWorldHouseStanceLabel(world, houseId) {
  const house = world.houses.find((h) => h.id === houseId);
  if (!house) return null;
  if (house.supportStatusLabel) return house.supportStatusLabel;
  if (house.supportStatus === 'undecided' || !house.supportCandidateId) return '미결정';
  const cand = world.candidates.find((c) => c.id === house.supportCandidateId);
  return cand ? `${cand.name} 지지` : '미결정';
}

/**
 * 고정 결과 fixture를 초기 월드에 적용한다.
 * 항상 초기 상태에서 시작하므로 같은 actionId는 같은 결과를 만든다.
 * @param {string} actionId
 */
export function applyAction(actionId) {
  const action = ACTIONS.find((a) => a.id === actionId);
  const outcome = OUTCOMES[actionId];
  if (!action || !outcome) {
    throw new Error(`Unknown action: ${actionId}`);
  }

  const world = createInitialWorld();
  const patch = outcome.worldPatch;

  world.player.stance = patch.playerStance;
  world.player.stanceText = patch.playerStanceText;
  world.newPublicInfo = deepClone(patch.newPublicInfo ?? []);

  for (const [houseId, override] of Object.entries(patch.houseOverrides ?? {})) {
    const house = world.houses.find((h) => h.id === houseId);
    if (!house) continue;
    if ('supportStatus' in override) house.supportStatus = override.supportStatus;
    if ('supportCandidateId' in override) house.supportCandidateId = override.supportCandidateId;
    if ('supportStatusLabel' in override) house.supportStatusLabel = override.supportStatusLabel;
  }

  for (const [candidateId, override] of Object.entries(patch.candidateOverrides ?? {})) {
    const cand = world.candidates.find((c) => c.id === candidateId);
    if (!cand) continue;
    if (override.claimStrengthText) cand.claimStrengthText = override.claimStrengthText;
    if (override.claimBasis) cand.claimBasis = override.claimBasis;
  }

  const result = {
    actionId,
    action,
    outcome: {
      actionId: outcome.actionId,
      chosenLabel: outcome.chosenLabel,
      responseTo: outcome.responseTo,
      helpedOrRefused: outcome.helpedOrRefused,
      directChanges: deepClone(outcome.directChanges),
      ripples: [...outcome.ripples],
      reasons: [...outcome.reasons],
      unchanged: [...outcome.unchanged],
    },
    world,
  };

  return result;
}

/**
 * 세션 상태 생성 (review → decision → resolved).
 * @returns {{
 *   phase: SessionPhase,
 *   selectedActionId: string|null,
 *   expandedProposalId: string|null,
 *   result: ReturnType<typeof applyAction>|null,
 *   world: ReturnType<typeof createInitialWorld>,
 * }}
 */
export function createSession() {
  return {
    phase: /** @type {SessionPhase} */ ('review'),
    selectedActionId: null,
    expandedProposalId: null,
    result: null,
    world: createInitialWorld(),
  };
}

/**
 * 세션을 초기 상태로 되돌린다. 원본 scenario는 건드리지 않는다.
 * @param {ReturnType<typeof createSession>} session
 */
export function resetSession(session) {
  session.phase = 'review';
  session.selectedActionId = null;
  session.expandedProposalId = null;
  session.result = null;
  session.world = createInitialWorld();
  return session;
}

/**
 * 행동 선택 (확정 전). resolved 상태에서는 무시한다.
 * @param {ReturnType<typeof createSession>} session
 * @param {string} actionId
 */
export function selectAction(session, actionId) {
  if (session.phase === 'resolved') return session;
  const action = ACTIONS.find((a) => a.id === actionId);
  if (!action) return session;
  session.selectedActionId = actionId;
  session.phase = 'decision';
  return session;
}

/**
 * 선택 확정 → 결과 적용. 이미 resolved면 중첩 적용하지 않는다.
 * @param {ReturnType<typeof createSession>} session
 */
export function confirmAction(session) {
  if (session.phase === 'resolved') return session;
  if (!session.selectedActionId) return session;
  const result = applyAction(session.selectedActionId);
  session.result = result;
  session.world = result.world;
  session.phase = 'resolved';
  return session;
}

/**
 * 확인 단계에서 돌아가기.
 * @param {ReturnType<typeof createSession>} session
 */
export function cancelDecision(session) {
  if (session.phase !== 'decision') return session;
  session.phase = 'review';
  session.selectedActionId = null;
  return session;
}

/**
 * 행동 ID 목록.
 */
export function getActionIds() {
  return ACTIONS.map((a) => a.id);
}

/**
 * 결과 객체가 유효한 구조인지 검사.
 * @param {string} actionId
 */
export function getOutcomeView(actionId) {
  const applied = applyAction(actionId);
  return applied.outcome;
}

// 브라우저 전역
if (typeof window !== 'undefined') {
  window.EpochInteractions = {
    PROPOSALS,
    ACTIONS,
    OUTCOMES,
    deepClone,
    createInitialWorld,
    getWorldSupportingHouses,
    getWorldHouseStanceLabel,
    applyAction,
    createSession,
    resetSession,
    selectAction,
    confirmAction,
    cancelDecision,
    getActionIds,
    getOutcomeView,
  };
}
