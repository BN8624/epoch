// 고정 계승 분쟁 fixture — 시나리오 데이터와 표현 헬퍼
/**
 * EPOCH M-1.1 고정 계승 분쟁 fixture.
 * 데이터와 화면 표현을 분리하기 위한 최소 구조 (Rust 코어 계약 아님).
 */

/** @typedef {'public_fact'|'unverified'|'private'} InformationVisibility */
/** @typedef {'legal_primogeniture'|'collateral_blood'|'ancient_line'} ClaimType */
/** @typedef {'kinship'|'military_promise'|'debt'|'education'} RelationshipType */
/** @typedef {'house_order'|'marriage_offer'|'old_debt'} PressureType */

export const scenario = {
  kingdom: {
    id: 'kingdom-arkeon',
    name: '아르케온 왕국',
  },

  ruler: {
    id: 'ruler-edren-iv',
    name: '에드렌 4세',
    healthStatus: '중병으로 의식을 잃었으며 사망이 임박했다.',
    authorityStatus: '왕의 권위가 약해지면서 유력 가문들이 후계자 지지를 공개하기 시작했다.',
    successionDeclaration: '공식 계승 선언은 남기지 않았다.',
    civilWarRisk: '분쟁이 장기화되면 내전 가능성이 있다.',
  },

  candidates: [
    {
      id: 'candidate-seria',
      name: '세리아 아르케온',
      label: '후보 A',
      relationshipToRuler: '에드렌 4세의 장녀. 왕실에서 공식적으로 인정된 직계 자녀.',
      claimType: /** @type {ClaimType} */ ('legal_primogeniture'),
      claimStrengthText: '현행 계승법상 가장 강한 법적 권리',
      claimBasis: '현행 계승법상 가장 강한 법적 권리를 가진다. 왕실이 공식적으로 인정한 직계 장녀로서 현 왕조의 정통 계승 질서에 가장 가깝다.',
      keyStrength: '가장 명확한 직계 혈통과 왕실 행정 조직의 지지',
      keyRisk: '직접 지휘할 군사력이 약해 군사 가문에 의존할 수 있음',
      strengths: [
        '가장 명확한 직계 혈통을 가진다.',
        '왕실 행정 조직과 바렌 가문의 지지를 받는다.',
        '기존 계승 질서를 유지할 수 있다.',
      ],
      weaknesses: [
        '직접 지휘할 군사력이 약하다.',
        '일부 유력 가문과 종교가 다르다.',
        '즉위하더라도 군사 가문에 의존할 가능성이 크다.',
      ],
      oppositionReasons: [
        '군사력이 부족해 즉위 직후 안정을 보장하기 어렵다.',
        '종교 차이로 일부 유력 가문과 봉신의 반발을 살 수 있다.',
      ],
      information: [
        {
          id: 'info-seria-claim',
          text: '세리아는 에드렌 4세의 공식 인정 직계 장녀이며 현행 계승법상 최우선 권리를 가진다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-seria-barren-support',
          text: '바렌 가문은 세리아를 공식 지지한다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-seria-document-rumor',
          text: '왕이 생전에 세리아를 후계자로 인정하는 문서를 작성했다는 소문이 있으나 원본은 공개되지 않았다.',
          visibility: /** @type {InformationVisibility} */ ('unverified'),
        },
      ],
    },
    {
      id: 'candidate-darian',
      name: '다리안 코르벤',
      label: '후보 B',
      relationshipToRuler: '에드렌 4세의 누이의 아들. 왕의 외조카.',
      claimType: /** @type {ClaimType} */ ('collateral_blood'),
      claimStrengthText: '왕실 방계 혈통 — 세리아보다 법적 순위가 낮음',
      claimBasis: '왕실 방계 혈통을 통한 계승권을 주장한다. 세리아보다 법적 순위가 낮다.',
      keyStrength: '북부군을 직접 통솔하며 즉시 수도 안정을 주장',
      keyRisk: '직계를 제친 즉위라는 비판과 과도한 영지·직위 약속',
      strengths: [
        '북부군을 직접 통솔한다.',
        '아르덴 가문과 소렌 가문의 공개 지지를 받는다.',
        '즉시 수도의 안정을 확보할 수 있다고 주장한다.',
      ],
      weaknesses: [
        '직계 후계자를 제치고 왕위를 차지한다는 비판을 받는다.',
        '지지를 얻기 위해 영지와 직위를 과도하게 약속했다.',
        '즉위 뒤 지지 가문을 통제하기 어려울 수 있다.',
      ],
      oppositionReasons: [
        '법적 권리가 세리아보다 약해 내전 위험이 있다.',
        '여러 지지 가문에 같은 핵심 직위를 약속했다는 의심이 있다.',
      ],
      information: [
        {
          id: 'info-darian-claim',
          text: '다리안은 왕의 외조카로 방계 혈통 계승권을 주장하며 법적 순위는 세리아보다 낮다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-darian-house-support',
          text: '아르덴 가문과 소렌 가문은 다리안을 공식 지지한다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-darian-army-move',
          text: '다리안이 왕의 사망 전에 수도로 군대를 이동시킬 계획이라는 보고가 있으나 확인되지 않았다.',
          visibility: /** @type {InformationVisibility} */ ('unverified'),
        },
      ],
    },
    {
      id: 'candidate-mireya',
      name: '미레아 셀칸',
      label: '후보 C',
      relationshipToRuler: '과거 폐위된 왕의 누나인 알레시아의 손녀. 몰락한 왕실 직계 계통의 후손.',
      claimType: /** @type {ClaimType} */ ('ancient_line'),
      claimStrengthText: '오래된 장자 계통 혈통 — 유효성은 논쟁 중',
      claimBasis: '현 왕조보다 오래된 장자 계통의 혈통 권리를 주장한다. 폐위된 계통의 권리가 유효한지는 논쟁 중이다.',
      keyStrength: '오래된 왕통의 상징과 국외 동맹 지원 가능성',
      keyRisk: '국내 기반이 약하고 외국 세력에 왕국을 넘길 수 있다는 의심',
      strengths: [
        '오래된 왕통을 지지하는 세력의 상징이다.',
        '메로바 가문의 지지를 받는다.',
        '국외 동맹국의 자금과 군사 지원 가능성이 있다.',
      ],
      weaknesses: [
        '오랫동안 망명 생활을 했다.',
        '국내 기반이 약하다.',
        '외국 세력에 왕국을 넘길 수 있다는 의심을 받는다.',
      ],
      oppositionReasons: [
        '국내 군사 기반이 약해 단독으로 수도를 장악하기 어렵다.',
        '외국 후원에 의존하면 반역으로 규정될 위험이 있다.',
      ],
      information: [
        {
          id: 'info-mireya-claim',
          text: '미레아는 폐위된 알레시아 계통의 손녀로 오래된 장자 계통 권리를 주장한다. 그 유효성은 논쟁 중이다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-mireya-merova-support',
          text: '메로바 가문은 미레아를 공식 지지한다.',
          visibility: /** @type {InformationVisibility} */ ('public_fact'),
        },
        {
          id: 'info-mireya-foreign-army',
          text: '국외 후원 세력이 실제로 군대를 보낼 의사가 있는지는 알려지지 않았다.',
          visibility: /** @type {InformationVisibility} */ ('unverified'),
        },
      ],
    },
  ],

  houses: [
    {
      id: 'house-arden',
      name: '아르덴 가문',
      supportCandidateId: 'candidate-darian',
      supportStatus: 'declared',
      positiveReasons: [
        {
          code: 'support_military_title_promise',
          text: '다리안은 아르덴 가문에 동부 변경백 직위를 약속했다.',
        },
        {
          code: 'support_military_link',
          text: '아르덴 가문의 군사력이 다리안의 북부군과 직접 연결돼 있다.',
        },
        {
          code: 'support_land_dispute_fear',
          text: '가문 수장은 세리아가 즉위하면 기존 영지 분쟁에서 불리해질 것으로 판단한다.',
        },
      ],
      negativeReasons: [
        {
          code: 'oppose_weaker_legal_claim',
          text: '다리안의 법적 권리가 세리아보다 약해 내전 위험이 있다.',
        },
        {
          code: 'oppose_duplicate_title_rumor',
          text: '약속된 직위를 다른 지지 가문도 요구하고 있다는 소문이 있다.',
        },
      ],
    },
    {
      id: 'house-barren',
      name: '바렌 가문',
      supportCandidateId: 'candidate-seria',
      supportStatus: 'declared',
      positiveReasons: [
        {
          code: 'support_legal_heir',
          text: '세리아는 현행 계승법상 가장 앞선 직계 후계자다.',
        },
        {
          code: 'support_admin_office',
          text: '바렌 가문은 왕실 행정 직위를 유지하려 한다.',
        },
        {
          code: 'support_marriage_tie',
          text: '바렌 가문의 후계자는 세리아의 가까운 친족과 혼인했다.',
        },
      ],
      negativeReasons: [
        {
          code: 'oppose_weak_military',
          text: '세리아에게 즉시 동원할 군사력이 부족하다.',
        },
        {
          code: 'oppose_religion_friction',
          text: '세리아의 종교가 일부 바렌 봉신들의 반발을 일으킬 수 있다.',
        },
      ],
    },
    {
      id: 'house-soren',
      name: '소렌 가문',
      supportCandidateId: 'candidate-darian',
      supportStatus: 'declared',
      positiveReasons: [
        {
          code: 'support_shared_faith',
          text: '다리안은 소렌 가문과 같은 종교를 따른다.',
        },
        {
          code: 'support_northern_protection',
          text: '다리안의 군대가 소렌 가문의 북부 영지를 보호하고 있다.',
        },
        {
          code: 'support_religious_office_fear',
          text: '소렌 가문은 세리아가 즉위하면 종교 직위가 제한될 것을 우려한다.',
        },
      ],
      negativeReasons: [
        {
          code: 'oppose_legal_order_break',
          text: '다리안의 즉위가 법적 계승 질서를 무너뜨릴 수 있다.',
        },
        {
          code: 'oppose_shared_title_suspicion',
          text: '다리안이 아르덴 가문에도 같은 핵심 직위를 약속했다는 의심이 있다.',
        },
      ],
    },
    {
      id: 'house-merova',
      name: '메로바 가문',
      supportCandidateId: 'candidate-mireya',
      supportStatus: 'declared',
      positiveReasons: [
        {
          code: 'support_old_oath',
          text: '메로바 가문은 폐위된 알레시아 계통에 충성을 맹세한 기록을 보유한다.',
        },
        {
          code: 'support_land_restoration',
          text: '미레아가 즉위하면 과거 몰수된 메로바 영지를 돌려받을 수 있다.',
        },
        {
          code: 'support_foreign_trade',
          text: '국외 동맹과의 교역 재개를 기대한다.',
        },
      ],
      negativeReasons: [
        {
          code: 'oppose_weak_domestic_base',
          text: '미레아의 국내 군사 기반이 약하다.',
        },
        {
          code: 'oppose_treason_label',
          text: '외국 후원에 의존하면 메로바 가문도 반역자로 규정될 수 있다.',
        },
      ],
    },
    {
      id: 'house-halbeck',
      name: '할베크 가문',
      supportCandidateId: null,
      supportStatus: 'undecided',
      positiveReasons: [
        {
          code: 'support_port_leverage',
          text: '세 후보 모두 할베크 가문의 항구와 함대가 필요하다.',
        },
        {
          code: 'support_neutrality_bargain',
          text: '중립을 유지하면 더 큰 조건을 요구할 수 있다.',
        },
      ],
      negativeReasons: [
        {
          code: 'oppose_opportunist_label',
          text: '결정을 늦추면 승리한 후보에게 기회주의자로 인식될 수 있다.',
        },
        {
          code: 'oppose_neutral_port_attack',
          text: '내전이 시작되면 중립 항구도 공격 대상이 될 수 있다.',
        },
      ],
    },
  ],

  player: {
    id: 'player-ren-arden',
    name: '렌 아르덴',
    status: '아르덴 가문 수장의 둘째 자녀',
    holding: '없음',
    office: '왕실 기록관 보좌',
    claimText: '어머니 쪽 왕실 혈통을 통한 약한 계승권',
    houseStanceCandidateId: 'candidate-darian',
    houseStanceText: '현재 가문 입장: 후보 B 다리안 지지',
    relationships: [
      {
        id: 'rel-seria',
        candidateId: 'candidate-seria',
        type: /** @type {RelationshipType} */ ('education'),
        text: '세리아와 왕실에서 함께 교육받은 과거가 있다.',
      },
      {
        id: 'rel-darian',
        candidateId: 'candidate-darian',
        type: /** @type {RelationshipType} */ ('military_promise'),
        text: '다리안은 렌의 가문 수장에게 동부 변경백 직위를 약속했다.',
      },
      {
        id: 'rel-mireya',
        candidateId: 'candidate-mireya',
        type: /** @type {RelationshipType} */ ('debt'),
        text: '미레아의 어머니는 과거 렌의 어머니를 망명 중 보호했다.',
      },
    ],
    pressures: [
      {
        id: 'pressure-house',
        type: /** @type {PressureType} */ ('house_order'),
        source: '가문 수장',
        text: '가문 수장은 렌에게 다리안 지지를 공개 선언하라고 요구한다.',
        visibility: /** @type {InformationVisibility} */ ('public_fact'),
      },
      {
        id: 'pressure-seria',
        type: /** @type {PressureType} */ ('marriage_offer'),
        source: '세리아 측',
        text: '세리아 측은 혼인을 통한 동맹 가능성을 비공개로 전달한다.',
        visibility: /** @type {InformationVisibility} */ ('private'),
      },
      {
        id: 'pressure-mireya',
        type: /** @type {PressureType} */ ('old_debt'),
        source: '미레아 측',
        text: '미레아 측은 오래된 보호의 빚을 갚으라고 요구한다.',
        visibility: /** @type {InformationVisibility} */ ('private'),
      },
    ],
  },
};

/** 정보 가시성 라벨 (코드 노출 방지) */
export const VISIBILITY_LABELS = {
  public_fact: '공개된 사실',
  unverified: '확인되지 않은 정보',
  private: '비공개 정보',
};

/** 권리 유형 표시용 (내부 claim_type 코드 비노출) */
export const CLAIM_TYPE_LABELS = {
  legal_primogeniture: '직계 법정 계승',
  collateral_blood: '방계 혈통',
  ancient_line: '오래된 왕통',
};

/**
 * 후보 ID로 후보를 찾는다. 없으면 null.
 * @param {string} candidateId
 */
export function getCandidate(candidateId) {
  if (!candidateId) return null;
  return scenario.candidates.find((c) => c.id === candidateId) ?? null;
}

/**
 * 가문 ID로 가문을 찾는다. 없으면 null.
 * @param {string} houseId
 */
export function getHouse(houseId) {
  if (!houseId) return null;
  return scenario.houses.find((h) => h.id === houseId) ?? null;
}

/**
 * 후보를 공개 지지하는 가문 목록.
 * @param {string} candidateId
 */
export function getSupportingHouses(candidateId) {
  const candidate = getCandidate(candidateId);
  if (!candidate) return [];
  return scenario.houses.filter(
    (h) => h.supportStatus === 'declared' && h.supportCandidateId === candidateId,
  );
}

/**
 * 후보 카드 요약 모델.
 * @param {string} candidateId
 */
export function getCandidateSummary(candidateId) {
  const candidate = getCandidate(candidateId);
  if (!candidate) return null;
  const supporters = getSupportingHouses(candidateId);
  return {
    id: candidate.id,
    name: candidate.name,
    label: candidate.label,
    relationshipToRuler: candidate.relationshipToRuler,
    claimStrengthText: candidate.claimStrengthText,
    keyStrength: candidate.keyStrength,
    keyRisk: candidate.keyRisk,
    supporterCount: supporters.length,
    supporterNames: supporters.map((h) => h.name),
  };
}

/**
 * 후보 상세 모델 (화면용 — 내부 코드 미포함).
 * @param {string} candidateId
 */
export function getCandidateDetail(candidateId) {
  const candidate = getCandidate(candidateId);
  if (!candidate) return null;
  const supporters = getSupportingHouses(candidateId);
  return {
    id: candidate.id,
    name: candidate.name,
    label: candidate.label,
    relationshipToRuler: candidate.relationshipToRuler,
    claimStrengthText: candidate.claimStrengthText,
    claimBasis: candidate.claimBasis,
    claimTypeLabel: CLAIM_TYPE_LABELS[candidate.claimType] ?? candidate.claimType,
    strengths: [...candidate.strengths],
    weaknesses: [...candidate.weaknesses],
    oppositionReasons: [...candidate.oppositionReasons],
    supportingHouses: supporters.map((h) => ({ id: h.id, name: h.name })),
    information: candidate.information.map((info) => ({
      id: info.id,
      text: info.text,
      visibility: info.visibility,
      visibilityLabel: VISIBILITY_LABELS[info.visibility] ?? info.visibility,
    })),
  };
}

/**
 * 가문 상세 모델 (이유 문장만 — 코드·점수 미포함).
 * @param {string} houseId
 */
export function getHouseDetail(houseId) {
  const house = getHouse(houseId);
  if (!house) return null;
  const supported = house.supportCandidateId
    ? getCandidate(house.supportCandidateId)
    : null;
  return {
    id: house.id,
    name: house.name,
    supportStatus: house.supportStatus,
    supportStatusLabel:
      house.supportStatus === 'undecided'
        ? '미결정'
        : supported
          ? `${supported.name} 지지`
          : '미결정',
    supportCandidateId: house.supportCandidateId,
    supportCandidateName: supported ? supported.name : null,
    positiveReasons: house.positiveReasons.map((r) => r.text),
    negativeReasons: house.negativeReasons.map((r) => r.text),
  };
}

/**
 * 플레이어 화면 모델.
 */
export function getPlayerView() {
  const p = scenario.player;
  const houseCandidate = getCandidate(p.houseStanceCandidateId);
  return {
    id: p.id,
    name: p.name,
    status: p.status,
    holding: p.holding,
    office: p.office,
    claimText: p.claimText,
    houseStanceText: p.houseStanceText,
    houseStanceCandidateName: houseCandidate ? houseCandidate.name : null,
    relationships: p.relationships.map((r) => {
      const cand = getCandidate(r.candidateId);
      return {
        id: r.id,
        candidateName: cand ? cand.name : r.candidateId,
        text: r.text,
      };
    }),
    pressures: p.pressures.map((pr) => ({
      id: pr.id,
      source: pr.source,
      text: pr.text,
      visibility: pr.visibility,
      visibilityLabel: VISIBILITY_LABELS[pr.visibility] ?? pr.visibility,
    })),
  };
}

/**
 * 위기 화면 모델.
 */
export function getCrisisView() {
  const { kingdom, ruler } = scenario;
  return {
    kingdomName: kingdom.name,
    rulerName: ruler.name,
    healthStatus: ruler.healthStatus,
    authorityStatus: ruler.authorityStatus,
    successionDeclaration: ruler.successionDeclaration,
    civilWarRisk: ruler.civilWarRisk,
  };
}

/**
 * 화면용 문자열에 내부 코드·원시 점수가 섞이지 않았는지 검사.
 * @param {string} text
 */
export function containsInternalLeak(text) {
  if (typeof text !== 'string') return false;
  const codePatterns = [
    /support_reason_code/i,
    /opposition_reason_code/i,
    /claim_type/i,
    /information_visibility/i,
    /relationship_type/i,
    /pressure_type/i,
    /support_[a-z_]+/,
    /oppose_[a-z_]+/,
    /legal_primogeniture/,
    /collateral_blood/,
    /ancient_line/,
    /public_fact/,
    /unverified/,
    /\bprivate\b/,
    /[+\-]\d+(\.\d+)?/,
    /utility/i,
  ];
  return codePatterns.some((re) => re.test(text));
}

/**
 * 상세 모델의 모든 사용자 노출 문자열을 평탄화.
 * @param {object} detail
 */
export function flattenUserFacingStrings(detail) {
  if (!detail) return [];
  const out = [];
  const walk = (v) => {
    if (typeof v === 'string') out.push(v);
    else if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === 'object') Object.values(v).forEach(walk);
  };
  walk(detail);
  return out;
}

// 브라우저 전역 (비모듈 script 호환)
if (typeof window !== 'undefined') {
  window.EpochScenario = {
    scenario,
    VISIBILITY_LABELS,
    CLAIM_TYPE_LABELS,
    getCandidate,
    getHouse,
    getSupportingHouses,
    getCandidateSummary,
    getCandidateDetail,
    getHouseDetail,
    getPlayerView,
    getCrisisView,
    containsInternalLeak,
    flattenUserFacingStrings,
  };
}
