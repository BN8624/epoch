// 고정 계승 분쟁 fixture·표현 로직 자동 검증
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  scenario,
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
  VISIBILITY_LABELS,
} from '../scenario.js';

describe('Fixture 불변식', () => {
  it('후보는 정확히 3명이다', () => {
    assert.equal(scenario.candidates.length, 3);
  });

  it('가문은 최소 5개다', () => {
    assert.ok(scenario.houses.length >= 5);
  });

  it('모든 인물·후보·가문 ID는 중복되지 않는다', () => {
    const ids = [
      scenario.ruler.id,
      scenario.player.id,
      ...scenario.candidates.map((c) => c.id),
      ...scenario.houses.map((h) => h.id),
    ];
    assert.equal(new Set(ids).size, ids.length);
  });

  it('모든 후보는 현 통치자와의 관계와 권리 근거를 가진다', () => {
    for (const c of scenario.candidates) {
      assert.ok(c.relationshipToRuler && c.relationshipToRuler.length > 0, c.id);
      assert.ok(c.claimBasis && c.claimBasis.length > 0, c.id);
      assert.ok(c.claimType, c.id);
      assert.ok(c.claimStrengthText, c.id);
    }
  });

  it('모든 후보는 최소 하나의 강점과 하나의 약점을 가진다', () => {
    for (const c of scenario.candidates) {
      assert.ok(c.strengths.length >= 1, c.id);
      assert.ok(c.weaknesses.length >= 1, c.id);
    }
  });

  it('모든 공개 지지는 존재하는 후보를 가리킨다', () => {
    for (const h of scenario.houses) {
      if (h.supportStatus === 'declared') {
        assert.ok(h.supportCandidateId, h.id);
        assert.ok(getCandidate(h.supportCandidateId), `${h.id} -> ${h.supportCandidateId}`);
      }
    }
  });

  it('최소 한 가문은 미결정 상태다', () => {
    assert.ok(scenario.houses.some((h) => h.supportStatus === 'undecided'));
  });

  it('후보 한 명이 모든 가문의 지지를 독점하지 않는다', () => {
    const declared = scenario.houses.filter((h) => h.supportStatus === 'declared');
    assert.ok(declared.length >= 2);
    const byCandidate = new Map();
    for (const h of declared) {
      const n = (byCandidate.get(h.supportCandidateId) || 0) + 1;
      byCandidate.set(h.supportCandidateId, n);
    }
    for (const [cid, n] of byCandidate) {
      assert.ok(n < declared.length, `${cid} monopolizes all declared support`);
    }
    assert.ok(byCandidate.size >= 2, 'at least two candidates have house support');
  });

  it('각 지지 가문(및 모든 가문)에는 긍정 이유와 부정 이유가 모두 존재한다', () => {
    for (const h of scenario.houses) {
      assert.ok(h.positiveReasons.length >= 1, h.id);
      assert.ok(h.negativeReasons.length >= 1, h.id);
      for (const r of h.positiveReasons) {
        assert.ok(r.code && r.text, h.id);
        assert.ok(
          ['public_fact', 'unverified', 'private'].includes(r.visibility),
          `${h.id} positive reason missing visibility`,
        );
      }
      for (const r of h.negativeReasons) {
        assert.ok(r.code && r.text, h.id);
        assert.ok(
          ['public_fact', 'unverified', 'private'].includes(r.visibility),
          `${h.id} negative reason missing visibility`,
        );
      }
    }
  });

  it('아르덴·소렌의 소문·의심 이유는 unverified이다', () => {
    const ardenRumor = scenario.houses
      .find((h) => h.id === 'house-arden')
      .negativeReasons.find((r) => r.code === 'oppose_duplicate_title_rumor');
    assert.equal(ardenRumor.visibility, 'unverified');
    const sorenSuspicion = scenario.houses
      .find((h) => h.id === 'house-soren')
      .negativeReasons.find((r) => r.code === 'oppose_shared_title_suspicion');
    assert.equal(sorenSuspicion.visibility, 'unverified');
  });

  it('모든 불확실한 정보에는 불확실함을 나타내는 상태가 있다', () => {
    for (const c of scenario.candidates) {
      for (const info of c.information) {
        assert.ok(
          ['public_fact', 'unverified', 'private'].includes(info.visibility),
          info.id,
        );
      }
    }
    for (const h of scenario.houses) {
      for (const r of [...h.positiveReasons, ...h.negativeReasons]) {
        assert.ok(
          ['public_fact', 'unverified', 'private'].includes(r.visibility),
          `${h.id}:${r.code}`,
        );
      }
    }
  });

  it('각 후보별로 확인되지 않은 정보가 최소 하나 있다', () => {
    for (const c of scenario.candidates) {
      const unverified = c.information.filter((i) => i.visibility === 'unverified');
      assert.ok(
        unverified.length >= 1,
        `${c.id} must have at least one unverified information item`,
      );
    }
  });

  it('플레이어는 세 후보 진영과 각각 다른 관계 또는 압력을 가진다', () => {
    const player = scenario.player;
    const candidateIds = scenario.candidates.map((c) => c.id);
    const related = new Set(player.relationships.map((r) => r.candidateId));
    assert.equal(related.size, 3);
    for (const id of candidateIds) {
      assert.ok(related.has(id), `missing relationship for ${id}`);
    }
    assert.equal(player.pressures.length, 3);
    const pressureTypes = new Set(player.pressures.map((p) => p.type));
    assert.equal(pressureTypes.size, 3);
  });

  it('시나리오 고정 인물·가문 이름이 정본과 일치한다', () => {
    assert.equal(scenario.kingdom.name, '아르케온 왕국');
    assert.equal(scenario.ruler.name, '에드렌 4세');
    assert.equal(scenario.candidates[0].name, '세리아 아르케온');
    assert.equal(scenario.candidates[1].name, '다리안 코르벤');
    assert.equal(scenario.candidates[2].name, '미레아 셀칸');
    assert.equal(scenario.player.name, '렌 아르덴');
    const houseNames = scenario.houses.map((h) => h.name);
    for (const name of ['아르덴 가문', '바렌 가문', '소렌 가문', '메로바 가문', '할베크 가문']) {
      assert.ok(houseNames.includes(name), name);
    }
  });
});

describe('표현 로직', () => {
  it('후보 선택 시 해당 후보 상세 모델이 반환된다', () => {
    for (const c of scenario.candidates) {
      const detail = getCandidateDetail(c.id);
      assert.ok(detail);
      assert.equal(detail.id, c.id);
      assert.equal(detail.name, c.name);
      assert.ok(detail.claimBasis);
      assert.ok(detail.strengths.length >= 1);
      assert.ok(detail.weaknesses.length >= 1);
      assert.ok(Array.isArray(detail.supportingHouses));
      assert.ok(detail.information.length >= 1);
    }
  });

  it('가문 선택 시 해당 가문의 이유 문장과 가시성 라벨이 반환된다', () => {
    for (const h of scenario.houses) {
      const detail = getHouseDetail(h.id);
      assert.ok(detail);
      assert.equal(detail.id, h.id);
      assert.ok(detail.positiveReasons.length >= 1);
      assert.ok(detail.negativeReasons.length >= 1);
      for (const r of [...detail.positiveReasons, ...detail.negativeReasons]) {
        assert.equal(typeof r.text, 'string');
        assert.ok(r.visibilityLabel);
        assert.ok(Object.values(VISIBILITY_LABELS).includes(r.visibilityLabel));
        assert.match(r.text, /[.。]$|[다요음]$|[다]$/);
      }
    }
    const arden = getHouseDetail('house-arden');
    const rumor = arden.negativeReasons.find((r) => r.text.includes('소문'));
    assert.ok(rumor);
    assert.equal(rumor.visibility, 'unverified');
    assert.equal(rumor.visibilityLabel, '확인되지 않은 정보');
    const soren = getHouseDetail('house-soren');
    const suspicion = soren.negativeReasons.find((r) => r.text.includes('의심'));
    assert.ok(suspicion);
    assert.equal(suspicion.visibility, 'unverified');
    assert.equal(suspicion.visibilityLabel, '확인되지 않은 정보');
  });

  it('존재하지 않는 ID를 선택해도 앱이 중단되지 않는다', () => {
    assert.equal(getCandidate('no-such-candidate'), null);
    assert.equal(getHouse('no-such-house'), null);
    assert.equal(getCandidateDetail('missing'), null);
    assert.equal(getHouseDetail('missing'), null);
    assert.equal(getCandidateSummary(''), null);
    assert.deepEqual(getSupportingHouses('missing'), []);
  });

  it('렌더링되는 모든 뷰 모델에 내부 코드·원시 효용이 노출되지 않는다', () => {
    const models = [
      getCrisisView(),
      getPlayerView(),
      ...scenario.candidates.map((c) => getCandidateSummary(c.id)),
      ...scenario.candidates.map((c) => getCandidateDetail(c.id)),
      ...scenario.houses.map((h) => getHouseDetail(h.id)),
    ];
    const userStrings = models.flatMap((m) => flattenUserFacingStrings(m));
    assert.ok(userStrings.length > 0);
    for (const s of userStrings) {
      assert.equal(containsInternalLeak(s), false, `internal leak in: ${s}`);
      assert.doesNotMatch(s, /support_[a-z_]+/);
      assert.doesNotMatch(s, /oppose_[a-z_]+/);
      assert.doesNotMatch(s, /legal_primogeniture|collateral_blood|ancient_line/);
      assert.doesNotMatch(s, /support_reason_code|opposition_reason_code/);
      assert.doesNotMatch(s, /[+\-]\d+(\.\d+)?\b/);
      assert.doesNotMatch(s, /utility|score|효용/i);
    }
  });

  it('원시 효용 숫자를 표시하지 않는다', () => {
    for (const c of scenario.candidates) {
      const d = getCandidateDetail(c.id);
      const texts = [
        ...d.strengths,
        ...d.weaknesses,
        ...d.oppositionReasons,
        d.claimBasis,
      ];
      for (const t of texts) {
        assert.doesNotMatch(t, /[+\-]\d+(\.\d+)?\b/);
        assert.doesNotMatch(t, /utility|score|효용/i);
      }
    }
    for (const h of scenario.houses) {
      const d = getHouseDetail(h.id);
      for (const r of [...d.positiveReasons, ...d.negativeReasons]) {
        assert.doesNotMatch(r.text, /[+\-]\d+(\.\d+)?\b/);
      }
      assert.equal('utility' in d, false);
      assert.equal('score' in d, false);
    }
  });

  it('후보 요약에 공개 지지 가문 수가 반영된다', () => {
    const seria = getCandidateSummary('candidate-seria');
    const darian = getCandidateSummary('candidate-darian');
    const mireya = getCandidateSummary('candidate-mireya');
    assert.equal(seria.supporterCount, 1);
    assert.equal(darian.supporterCount, 2);
    assert.equal(mireya.supporterCount, 1);
  });

  it('할베크는 미결정이며 지지 후보가 없다', () => {
    const h = getHouseDetail('house-halbeck');
    assert.equal(h.supportStatus, 'undecided');
    assert.equal(h.supportCandidateId, null);
    assert.equal(h.supportStatusLabel, '미결정');
  });

  it('정보 가시성 라벨이 코드가 아닌 한국어로 제공된다', () => {
    assert.equal(VISIBILITY_LABELS.public_fact, '공개된 사실');
    assert.equal(VISIBILITY_LABELS.unverified, '확인되지 않은 정보');
    assert.equal(VISIBILITY_LABELS.private, '비공개 정보');
  });

  it('flattenUserFacingStrings는 문자열 목록을 반환한다', () => {
    const d = getHouseDetail('house-arden');
    const flat = flattenUserFacingStrings(d);
    assert.ok(flat.length > 0);
    assert.ok(flat.every((s) => typeof s === 'string'));
    assert.ok(!flat.includes('public_fact'));
    assert.ok(!flat.includes('unverified'));
    assert.ok(flat.includes('확인되지 않은 정보'));
  });
});
