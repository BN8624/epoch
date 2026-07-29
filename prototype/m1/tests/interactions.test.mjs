// M-1.2 제안·행동·결과 fixture·결정론·격리 검증
import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { scenario, containsInternalLeak, flattenUserFacingStrings } from '../scenario.js';
import {
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
} from '../interactions.js';

describe('제안·행동 fixture 불변식', () => {
  it('NPC 제안은 정확히 3개다', () => {
    assert.equal(PROPOSALS.length, 3);
  });

  it('플레이어 행동은 정확히 5개다', () => {
    assert.equal(ACTIONS.length, 5);
    assert.equal(getActionIds().length, 5);
  });

  it('모든 제안은 제안자, 요구, 이익, 위험, 정보 상태를 가진다', () => {
    for (const p of PROPOSALS) {
      assert.ok(p.proposer, p.id);
      assert.ok(p.demand, p.id);
      assert.ok(p.benefit, p.id);
      assert.ok(p.risk, p.id);
      assert.ok(['public_fact', 'unverified', 'private'].includes(p.visibility), p.id);
      assert.ok(p.relatedLabel, p.id);
    }
  });

  it('모든 행동은 이익, 손실·위험, 영향 대상을 가진다', () => {
    for (const a of ACTIONS) {
      assert.ok(a.label, a.id);
      assert.ok(a.helps, a.id);
      assert.ok(a.benefits.length >= 1, a.id);
      assert.ok(a.losses.length >= 1 || a.risks.length >= 1, a.id);
      assert.ok(a.affected.length >= 1, a.id);
    }
  });

  it('모든 행동은 적어도 하나의 이익과 하나의 손실 또는 위험을 가진다', () => {
    for (const a of ACTIONS) {
      assert.ok(a.benefits.length >= 1, `${a.id} benefits`);
      assert.ok(a.losses.length + a.risks.length >= 1, `${a.id} losses/risks`);
    }
  });

  it('모든 세력을 동시에 만족시키는 행동이 없다', () => {
    // 손실·위험 유무와 독립적으로 helps·benefits의 진영별 긍정 효과만 판별한다.
    // (예: benefits에 세 후보 신뢰 증가가 모두 있으면 손실이 있어도 실패)
    const FACTIONS = ['다리안', '세리아', '미레아'];

    function positivelySatisfies(action, name) {
      const helps = action.helps || '';
      if (helps.includes(name) && !/돕지 않|거절|중립/.test(helps)) {
        return true;
      }
      for (const line of action.benefits) {
        if (!line.includes(name)) continue;
        // 균열·하락 등 피해 서술만 있는 이익 문장은 해당 진영 만족으로 보지 않는다.
        if (/(균열|하락|적대|경계|의심|거부|거절|배신|보복|상실|박탈)/.test(line)) {
          const stillPositive = new RegExp(
            `${name}.{0,16}(신뢰|우호).{0,8}(증가|상승)|${name}.{0,12}(동맹|협력)`,
          ).test(line);
          if (!stillPositive) continue;
        }
        if (
          new RegExp(`${name}.{0,20}(신뢰|우호).{0,8}(증가|상승)`).test(line) ||
          new RegExp(`${name}.{0,16}(동맹|협력|연결|인정|보상|영향력)`).test(line) ||
          new RegExp(`(신뢰|우호).{0,8}(증가|상승).{0,16}${name}`).test(line)
        ) {
          return true;
        }
      }
      return false;
    }

    for (const a of ACTIONS) {
      const satisfied = FACTIONS.filter((name) => positivelySatisfies(a, name));
      assert.ok(
        satisfied.length < FACTIONS.length,
        `${a.id} satisfies all factions: ${satisfied.join(',')}`,
      );
    }
  });

  it('모든 행동 결과는 존재하는 후보·가문·플레이어를 참조한다', () => {
    const candidateIds = new Set(scenario.candidates.map((c) => c.id));
    const houseIds = new Set(scenario.houses.map((h) => h.id));
    const proposalIds = new Set(PROPOSALS.map((p) => p.id));

    function assertCandidateRef(value, context) {
      assert.ok(
        value === null || value === undefined || candidateIds.has(value),
        `${context}: invalid candidate id ${value}`,
      );
    }
    function assertHouseRef(value, context) {
      assert.ok(
        value === null || value === undefined || houseIds.has(value),
        `${context}: invalid house id ${value}`,
      );
    }

    for (const p of PROPOSALS) {
      assertCandidateRef(p.relatedCandidateId, `proposal ${p.id} relatedCandidateId`);
      assertHouseRef(p.relatedHouseId, `proposal ${p.id} relatedHouseId`);
    }

    for (const a of ACTIONS) {
      if (a.responseProposalId != null) {
        assert.ok(
          proposalIds.has(a.responseProposalId),
          `${a.id} responseProposalId ${a.responseProposalId}`,
        );
      }

      const outcome = OUTCOMES[a.id];
      assert.ok(outcome, a.id);
      assert.equal(outcome.actionId, a.id);
      const patch = outcome.worldPatch;
      assert.equal(typeof patch.playerStance, 'string');

      for (const [hid, override] of Object.entries(patch.houseOverrides ?? {})) {
        assert.ok(houseIds.has(hid), `${a.id} houseOverride key ${hid}`);
        if (override && typeof override === 'object' && 'supportCandidateId' in override) {
          assertCandidateRef(
            override.supportCandidateId,
            `${a.id} houseOverrides[${hid}].supportCandidateId`,
          );
        }
      }
      for (const [cid, override] of Object.entries(patch.candidateOverrides ?? {})) {
        assert.ok(candidateIds.has(cid), `${a.id} candidateOverride key ${cid}`);
        if (override && typeof override === 'object') {
          if ('id' in override) assertCandidateRef(override.id, `${a.id} candidateOverrides[${cid}].id`);
          if ('supportCandidateId' in override) {
            assertCandidateRef(
              override.supportCandidateId,
              `${a.id} candidateOverrides[${cid}].supportCandidateId`,
            );
          }
        }
      }
    }
  });

  it('각 결과에는 직접 변화, 주요 파급, 이유, 바뀌지 않은 것이 존재한다', () => {
    for (const a of ACTIONS) {
      const o = OUTCOMES[a.id];
      assert.ok(o.directChanges, a.id);
      assert.ok(o.directChanges.playerStance, a.id);
      assert.ok(o.directChanges.relationChanges.length >= 1, a.id);
      assert.ok(o.directChanges.benefitsGained.length >= 1, a.id);
      assert.ok(o.directChanges.risksCreated.length >= 1, a.id);
      assert.ok(o.ripples.length >= 1, a.id);
      assert.ok(o.reasons.length >= 2 && o.reasons.length <= 4, a.id);
      assert.ok(o.unchanged.length >= 1 && o.unchanged.length <= 3, a.id);
    }
  });

  it('제안 문구가 이슈 명세와 일치한다', () => {
    assert.equal(PROPOSALS[0].proposer, '아르덴 가문 수장');
    assert.equal(PROPOSALS[0].demand, '다리안 코르벤을 공개 지지하라.');
    assert.equal(PROPOSALS[1].proposer, '세리아 아르케온 측 사절');
    assert.equal(PROPOSALS[2].proposer, '미레아 셀칸 측 사절');
  });

  it('행동 표시명이 이슈 명세와 일치한다', () => {
    assert.equal(ACTIONS[0].label, '다리안을 공개 지지하고 직위 약속을 공표한다');
    assert.equal(ACTIONS[1].label, '세리아와 비밀 혼인 동맹을 맺고 지지를 약속한다');
    assert.equal(ACTIONS[2].label, '미레아에게 알레시아 계통의 권리 기록 사본을 제공한다');
    assert.equal(
      ACTIONS[3].label,
      '다리안이 같은 핵심 직위를 중복 약속했다는 정보를 세리아 측에 넘긴다',
    );
    assert.equal(ACTIONS[4].label, '세 진영의 요구를 모두 거절하고 결정을 미룬다');
  });
});

describe('결정론과 격리', () => {
  it('같은 초기 상태에 같은 행동을 적용하면 깊은 동등성 기준으로 같은 결과가 나온다', () => {
    for (const id of getActionIds()) {
      const a = applyAction(id);
      const b = applyAction(id);
      assert.deepEqual(a.world, b.world, id);
      assert.deepEqual(a.outcome, b.outcome, id);
    }
  });

  it('서로 다른 행동은 적어도 하나 이상의 다른 상태 변화를 만든다', () => {
    const ids = getActionIds();
    for (let i = 0; i < ids.length; i++) {
      for (let j = i + 1; j < ids.length; j++) {
        const wa = applyAction(ids[i]).world;
        const wb = applyAction(ids[j]).world;
        assert.notDeepEqual(wa, wb, `${ids[i]} vs ${ids[j]}`);
      }
    }
  });

  it('행동 A 적용 뒤 재시작하면 초기 상태와 깊은 동등성을 이룬다', () => {
    const session = createSession();
    const initial = deepClone(session.world);
    selectAction(session, 'action-a');
    confirmAction(session);
    assert.equal(session.phase, 'resolved');
    resetSession(session);
    assert.equal(session.phase, 'review');
    assert.equal(session.selectedActionId, null);
    assert.equal(session.result, null);
    assert.deepEqual(session.world, initial);
  });

  it('행동 A 뒤 재시작하고 행동 B를 적용한 결과에 A의 변경이 남지 않는다', () => {
    const session = createSession();
    selectAction(session, 'action-a');
    confirmAction(session);
    const afterA = deepClone(session.world);
    resetSession(session);
    selectAction(session, 'action-b');
    confirmAction(session);
    const afterB = session.world;
    // A: 소렌 동요 / B: 할베크 세리아 기울음
    const sorenB = afterB.houses.find((h) => h.id === 'house-soren');
    assert.notEqual(sorenB.supportStatusLabel, '동요');
    assert.equal(sorenB.supportStatus, 'declared');
    const halbB = afterB.houses.find((h) => h.id === 'house-halbeck');
    assert.equal(halbB.supportStatusLabel, '세리아 쪽으로 기울음');
    // A 결과와 다름
    assert.notDeepEqual(afterB, afterA);
    // B만 적용한 순수 결과와 동일
    assert.deepEqual(afterB, applyAction('action-b').world);
  });

  it('초기 scenario fixture가 결과 적용 과정에서 변형되지 않는다', () => {
    const snapshot = deepClone({
      houses: scenario.houses,
      candidates: scenario.candidates,
      player: scenario.player,
    });
    for (const id of getActionIds()) {
      applyAction(id);
    }
    const session = createSession();
    selectAction(session, 'action-d');
    confirmAction(session);
    resetSession(session);
    selectAction(session, 'action-c');
    confirmAction(session);
    assert.deepEqual(
      {
        houses: scenario.houses,
        candidates: scenario.candidates,
        player: scenario.player,
      },
      snapshot,
    );
  });

  it('확정 뒤에는 같은 실행에서 두 번째 행동을 중첩 적용할 수 없다', () => {
    const session = createSession();
    selectAction(session, 'action-a');
    confirmAction(session);
    const worldAfterA = deepClone(session.world);
    selectAction(session, 'action-b');
    confirmAction(session);
    assert.deepEqual(session.world, worldAfterA);
    assert.equal(session.result.actionId, 'action-a');
  });

  it('돌아가기로 결과 확정 없이 선택 화면으로 복귀한다', () => {
    const session = createSession();
    selectAction(session, 'action-c');
    assert.equal(session.phase, 'decision');
    cancelDecision(session);
    assert.equal(session.phase, 'review');
    assert.equal(session.selectedActionId, null);
    assert.equal(session.result, null);
  });
});

describe('행동별 핵심 결과', () => {
  it('행동 A: 소렌 가문이 동요하고 플레이어는 다리안 공개 지지', () => {
    const { world, outcome } = applyAction('action-a');
    assert.equal(outcome.directChanges.playerStance, '다리안 공개 지지');
    const soren = world.houses.find((h) => h.id === 'house-soren');
    assert.equal(soren.supportStatusLabel, '동요');
    assert.equal(soren.supportStatus, 'wavering');
    assert.equal(getWorldHouseStanceLabel(world, 'house-soren'), '동요');
    // 동요는 공개 지지(declared)로 세지 않음 — 아르덴만 유지
    assert.equal(getWorldSupportingHouses(world, 'candidate-darian').length, 1);
    assert.equal(
      getWorldSupportingHouses(world, 'candidate-darian')[0].id,
      'house-arden',
    );
  });

  it('행동 B: 할베크가 세리아 쪽으로 기울고 A 상태가 없다', () => {
    const { world, outcome } = applyAction('action-b');
    assert.equal(outcome.directChanges.playerStance, '세리아 비밀 지지');
    const halb = world.houses.find((h) => h.id === 'house-halbeck');
    assert.equal(halb.supportStatusLabel, '세리아 쪽으로 기울음');
    const soren = world.houses.find((h) => h.id === 'house-soren');
    assert.notEqual(soren.supportStatusLabel, '동요');
  });

  it('행동 C: 미레아 권리 문구와 할베크 입장이 바뀐다', () => {
    const { world } = applyAction('action-c');
    const mireya = world.candidates.find((c) => c.id === 'candidate-mireya');
    assert.equal(mireya.claimStrengthText, '기록 증거를 확보한 오래된 왕통');
    const halb = world.houses.find((h) => h.id === 'house-halbeck');
    assert.equal(halb.supportStatusLabel, '미레아 쪽으로 기울음');
  });

  it('행동 D: 소렌 미결정, 다리안 공개 지지 2→1', () => {
    const initial = createInitialWorld();
    assert.equal(getWorldSupportingHouses(initial, 'candidate-darian').length, 2);
    const { world } = applyAction('action-d');
    const soren = world.houses.find((h) => h.id === 'house-soren');
    assert.equal(soren.supportStatus, 'undecided');
    assert.equal(soren.supportCandidateId, null);
    assert.equal(getWorldHouseStanceLabel(world, 'house-soren'), '미결정');
    assert.equal(getWorldSupportingHouses(world, 'candidate-darian').length, 1);
  });

  it('행동 E: 초기 가문 지지 구도가 유지된다', () => {
    const initial = createInitialWorld();
    const { world, outcome } = applyAction('action-e');
    assert.equal(outcome.directChanges.playerStance, '중립');
    for (const h of initial.houses) {
      const w = world.houses.find((x) => x.id === h.id);
      assert.equal(w.supportStatus, h.supportStatus, h.id);
      assert.equal(w.supportCandidateId, h.supportCandidateId, h.id);
    }
  });
});

describe('사용자 문자열 비노출', () => {
  it('결과 이유에 내부 코드·원시 점수가 없다', () => {
    for (const id of getActionIds()) {
      const { outcome } = applyAction(id);
      const userFacing = {
        chosenLabel: outcome.chosenLabel,
        responseTo: outcome.responseTo,
        helpedOrRefused: outcome.helpedOrRefused,
        directChanges: outcome.directChanges,
        ripples: outcome.ripples,
        reasons: outcome.reasons,
        unchanged: outcome.unchanged,
      };
      const strings = flattenUserFacingStrings(userFacing);
      assert.ok(strings.length > 0, id);
      for (const s of strings) {
        assert.equal(containsInternalLeak(s), false, `${id}: ${s}`);
        assert.doesNotMatch(s, /utility|score|효용/i);
        assert.doesNotMatch(s, /action-[a-e]|proposal-/);
        assert.doesNotMatch(s, /[+\-]\d+(\.\d+)?\b/);
      }
      for (const r of outcome.reasons) {
        assert.match(r, /[가-힣]/);
      }
    }
  });

  it('행동·제안 표시 문자열에 내부 enum이 없다', () => {
    const strings = [
      ...PROPOSALS.flatMap((p) => [p.proposer, p.demand, p.benefit, p.risk, p.relatedLabel]),
      ...ACTIONS.flatMap((a) => [
        a.label,
        a.helps,
        ...a.benefits,
        ...a.losses,
        ...a.risks,
        ...a.affected,
      ]),
    ];
    for (const s of strings) {
      assert.equal(containsInternalLeak(s), false, s);
    }
  });
});
