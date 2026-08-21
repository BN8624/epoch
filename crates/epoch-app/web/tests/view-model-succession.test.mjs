// SuccessionWorld overlay를 읽기 전용 view-model 투영으로 검증한다
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import {
  buildIndexes,
  generationLabel,
  getCrisisView,
  getRealmView,
  getSuccessionCandidateDetail,
  getSuccessionDisputeView,
  getSuccessionHouseDetail,
  getVisibleInformation,
} from '../view-model.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WORKSPACE = path.resolve(HERE, '../../../..');

function exportSuccession(seed, realmId) {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), `epoch-app-vm-succession-${seed}-`));
  const result = spawnSync(
    'cargo',
    ['run', '-q', '-p', 'epoch-app', '--', 'export-succession', String(seed), realmId, outDir],
    {
      cwd: WORKSPACE,
      encoding: 'utf8',
      shell: process.platform === 'win32',
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `export-succession seed=${seed} failed:\n${result.stdout || ''}\n${result.stderr || ''}`,
    );
  }
  const world = JSON.parse(fs.readFileSync(path.join(outDir, 'rights-world.json'), 'utf8'));
  const succession = JSON.parse(
    fs.readFileSync(path.join(outDir, 'succession-world.json'), 'utf8'),
  );
  return { outDir, world, succession, stdout: result.stdout };
}

test('seed 1 succession overlay projects three claimants and vacancy', () => {
  const { outDir, world, succession, stdout } = exportSuccession(1, 'realm-01');
  try {
    assert.equal(
      stdout.trim(),
      'APP_SUCCESSION_EXPORT_OK seed=1 realm=realm-01 rights_bytes=66222 succession_bytes=71915 files=6',
    );
    const idx = buildIndexes(world, succession);
    const crisis = getCrisisView(idx, 'realm-01');
    assert.ok(crisis);
    assert.equal(crisis.vacant, true);
    assert.equal(crisis.candidates.length, 3);
    const former = idx.personById[succession.transition.death.person_id];
    assert.equal(crisis.formerIncumbentName, former.name);
    assert.equal(
      crisis.formerIncumbentPersonId,
      succession.transition.death.person_id,
    );
    assert.notEqual(crisis.presumptiveSuccessorPersonId, crisis.formerIncumbentPersonId);

    const priority = crisis.priority;
    assert.ok(priority);
    assert.equal(priority.priority, 'direct_strong_original');
    assert.equal(priority.origin, 'original');
    assert.equal(priority.standingLabel, '강한 직계 권리');
    assert.equal(priority.personId, succession.transition.presumptive_successor_person_id);
    assert.equal(priority.personName, idx.personById[priority.personId].name);

    const competing = crisis.competing;
    assert.equal(competing.length, 2);
    const restored = competing.find((item) => item.priority === 'restored_contested_original');
    const derived = competing.find((item) => item.priority === 'restored_contested_derived');
    assert.ok(restored);
    assert.ok(derived);
    assert.equal(restored.origin, 'original');
    assert.equal(restored.standingLabel, '논쟁 중인 복권 권리');
    assert.equal(derived.origin, 'derived');
    assert.equal(derived.generationDistance, 1);
    assert.equal(derived.standingLabel, '혈통을 따라 파생된 복권 권리');
    const derivedRecord = succession.pre_succession_world.propagation.derived_claims.find(
      (item) => item.id === derived.claimRecordId,
    );
    assert.ok(derivedRecord);
    assert.equal(derived.sourceClaimId, derivedRecord.source_claim_id);
    assert.equal(derived.sourcePersonId, derivedRecord.via_parent_person_id);
    assert.equal(derived.sourcePersonName, idx.personById[derivedRecord.via_parent_person_id].name);
    assert.equal(derived.personName, idx.personById[derived.personId].name);

    const realm = getRealmView(idx, 'realm-01');
    assert.equal(realm.vacant, true);
    assert.equal(realm.incumbentName, '공석');
    assert.equal(realm.formerIncumbentName, former.name);
    assert.notEqual(realm.incumbentName, former.name);

    const other = getRealmView(idx, 'realm-02');
    assert.equal(other.vacant, false);
    assert.ok(other.incumbentName);
    assert.notEqual(other.incumbentName, '공석');
    assert.equal(getCrisisView(idx, 'realm-02'), null);
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});

test('plain rights world has no succession overlay', () => {
  const { outDir, world, succession } = exportSuccession(1, 'realm-01');
  try {
    const idx = buildIndexes(world);
    assert.equal(getCrisisView(idx, 'realm-01'), null);
    assert.equal(getSuccessionDisputeView(idx, 'realm-01'), null);
    assert.equal(getSuccessionCandidateDetail(idx, 'realm-01', 'person-006'), null);
    assert.equal(getSuccessionHouseDetail(idx, 'realm-01', 'house-01'), null);
    const realm = getRealmView(idx, 'realm-01');
    assert.equal(realm.vacant, false);
    assert.notEqual(realm.incumbentName, '공석');
    assert.equal(
      realm.incumbentPersonId,
      succession.pre_succession_world.family_world.rights_world.rights.realms.find(
        (item) => item.realm_id === 'realm-01',
      ).incumbent_person_id,
    );
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});

test('seed 1 dispute view maps priority slots from SuccessionWorld', () => {
  const { outDir, world, succession } = exportSuccession(1, 'realm-01');
  try {
    const idx = buildIndexes(world, succession);
    const dispute = getSuccessionDisputeView(idx, 'realm-01');
    assert.ok(dispute);
    assert.equal(dispute.unresolved, false);
    assert.equal(dispute.realmId, 'realm-01');
    assert.equal(dispute.realmName, idx.realmById['realm-01'].name);
    assert.equal(dispute.formerIncumbentPersonId, 'person-003');
    assert.equal(dispute.formerIncumbentName, idx.personById['person-003'].name);
    assert.equal(dispute.vacant, true);
    assert.equal(dispute.legalStatus, '법적 우선 후보가 있으나 계승은 확정되지 않음');
    assert.equal(dispute.presumptiveSuccessorPersonId, 'person-006');
    assert.equal(dispute.candidates.length, 3);
    assert.deepEqual(
      dispute.candidates.map((item) => item.slot),
      ['A', 'B', 'C'],
    );
    assert.deepEqual(
      dispute.candidates.map((item) => item.priority),
      [
        'direct_strong_original',
        'restored_contested_original',
        'restored_contested_derived',
      ],
    );

    const [candA, candB, candC] = dispute.candidates;
    assert.equal(candA.personId, 'person-006');
    assert.equal(candA.claimRecordId, 'claim-01');
    assert.equal(candA.origin, 'original');
    assert.equal(candA.personName, idx.personById['person-006'].name);
    assert.equal(candA.houseName, idx.houseById['house-01'].name);
    assert.equal(candA.badge, '법적 우선 후보');
    assert.equal(candA.standingLabel, '강한 직계 권리');
    assert.equal(candA.isKnownChildOfFormer, true);
    assert.equal(candA.evidenceLabel, '직전 통치자의 알려진 자녀');
    assert.ok((idx.personById['person-006'].known_parent_ids ?? []).includes('person-003'));

    assert.equal(candB.personId, 'person-019');
    assert.equal(candB.claimRecordId, 'claim-02');
    assert.equal(candB.origin, 'original');
    assert.equal(candB.personName, idx.personById['person-019'].name);
    assert.equal(candB.houseName, idx.houseById['house-03'].name);
    assert.equal(candB.badge, '경쟁 권리자');
    assert.equal(candB.standingLabel, '논쟁 중인 복권 권리');
    assert.equal(candB.isRestoredLineHead, true);
    assert.equal(candB.evidenceLabel, '옛 계통을 뒷받침하는 역사 기록 보유');

    assert.equal(candC.personId, 'person-022');
    assert.equal(candC.claimRecordId, 'derived-claim-01');
    assert.equal(candC.origin, 'derived');
    assert.equal(candC.personName, idx.personById['person-022'].name);
    assert.equal(candC.generationDistance, 1);
    assert.equal(candC.provenance.sourceClaimId, 'claim-02');
    assert.equal(candC.provenance.sourcePersonId, 'person-019');
    assert.equal(candC.provenance.sourcePersonName, idx.personById['person-019'].name);
    assert.equal(candC.provenance.isKnownChildOfSource, true);
    assert.equal(
      candC.provenance.sentence,
      `${idx.personById['person-019'].name}의 자녀로서 복권 권리가 한 세대 전파됨`,
    );
    assert.ok((idx.personById['person-022'].known_parent_ids ?? []).includes('person-019'));

    for (const candidate of dispute.candidates) {
      assert.equal('support' in candidate, false);
      assert.equal('strengths' in candidate, false);
      assert.equal('weaknesses' in candidate, false);
      assert.equal(candidate.personName, idx.personById[candidate.personId].name);
      assert.notEqual(candidate.personName, candidate.personId);
    }

    assert.equal(dispute.houses.length, 3);
    const ruling = dispute.houses.find((house) => house.id === 'house-01');
    assert.ok(ruling);
    assert.equal(ruling.name, idx.houseById['house-01'].name);
    assert.equal(ruling.headStatus.isDeceasedHead, true);
    assert.equal(ruling.headStatus.currentHeadUndecided, true);
    assert.equal(ruling.headStatus.currentHeadPersonId, null);
    assert.ok(ruling.headStatus.cardHeadLines.some((line) => line.includes('사망')));
    assert.ok(ruling.headStatus.cardHeadLines.some((line) => line.includes('미결정')));
    assert.equal(
      ruling.headStatus.cardHeadLines.some((line) => line.startsWith('현재 가문 수장')),
      false,
    );

    const living = dispute.houses.filter((house) => house.id !== 'house-01');
    assert.equal(living.length, 2);
    for (const house of living) {
      assert.equal(house.headStatus.isDeceasedHead, false);
      assert.equal(house.headStatus.currentHeadPersonId, idx.houseById[house.id].head_person_id);
      assert.ok(house.headStatus.cardHeadLines.some((line) => line.startsWith('수장:')));
      assert.equal(house.name, idx.houseById[house.id].name);
    }
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});

test('seed 1 dispute candidate and house details keep privacy and actual relations', () => {
  const { outDir, world, succession } = exportSuccession(1, 'realm-01');
  try {
    const idx = buildIndexes(world, succession);
    const dispute = getSuccessionDisputeView(idx, 'realm-01');
    const candA = getSuccessionCandidateDetail(idx, 'realm-01', 'person-006');
    const candB = getSuccessionCandidateDetail(idx, 'realm-01', 'person-019');
    const candC = getSuccessionCandidateDetail(idx, 'realm-01', 'person-022');
    assert.ok(candA && candB && candC);

    const claimById = Object.fromEntries(idx.layers.rights.claims.map((claim) => [claim.id, claim]));
    const derivedById = Object.fromEntries(
      succession.pre_succession_world.propagation.derived_claims.map((item) => [item.id, item]),
    );

    function assertCandidateIdentity(detail, personId) {
      const person = idx.personById[personId];
      const house = idx.houseById[person.house_id];
      const identity = idx.personIdentityById[personId];
      assert.equal(detail.name, person.name);
      assert.equal(detail.realmId, 'realm-01');
      assert.equal(detail.realmName, idx.realmById['realm-01'].name);
      assert.equal(detail.houseId, person.house_id);
      assert.equal(detail.houseName, house.name);
      assert.equal(detail.generation, person.generation);
      assert.equal(detail.generationLabel, generationLabel(person.generation));
      assert.equal(detail.cultureName, idx.cultureById[identity.culture_id].name);
      assert.equal(detail.religionName, idx.religionById[identity.religion_id].name);
      assert.equal(
        detail.activityLabel,
        idx.activeByPerson[personId] ? '적극적 정치 행위자' : '보조 인물',
      );
      assert.equal(detail.unresolved, false);
    }

    assertCandidateIdentity(candA, 'person-006');
    assert.equal(candA.rights.origin, 'original');
    assert.equal(candA.rights.claimRecordId, 'claim-01');
    assert.equal(candA.rights.priority, 'direct_strong_original');
    assert.equal(candA.rights.standingLabel, '강한 직계 권리');
    assert.equal(candA.rights.priorityLabel, '강한 직계 권리');
    assert.equal(candA.rights.evidenceLabel, '직전 통치자의 알려진 자녀');
    assert.equal(
      candA.rights.generationDistance,
      succession.transition.candidates.find((item) => item.person_id === 'person-006')
        .generation_distance,
    );
    assert.equal(candA.lineage.kind, 'direct');
    assert.equal(candA.lineage.label, '직전 통치자의 알려진 자녀');

    assertCandidateIdentity(candB, 'person-019');
    assert.equal(candB.houseId, 'house-03');
    assert.equal(candB.houseName, idx.houseById['house-03'].name);
    assert.equal(candB.rights.origin, 'original');
    assert.equal(candB.rights.claimRecordId, 'claim-02');
    assert.equal(candB.rights.priority, 'restored_contested_original');
    assert.equal(candB.rights.standingLabel, '논쟁 중인 복권 권리');
    assert.equal(candB.rights.evidenceLabel, '옛 계통을 뒷받침하는 역사 기록 보유');
    assert.equal(candB.lineage.kind, 'direct');
    assert.equal(candB.lineage.label, '복권 계통의 현 가문 수장');
    assert.equal(idx.houseById['house-03'].head_person_id, 'person-019');
    assert.equal(dispute.candidateB.isRestoredLineHead, true);
    const restoredClaim = claimById['claim-02'];
    assert.ok(restoredClaim);
    assert.equal(restoredClaim.claimant_person_id, 'person-019');
    const restoredEvidence = (restoredClaim.evidence_record_ids ?? []).map(
      (id) => idx.evidenceById[id],
    );
    assert.equal(
      restoredEvidence.some((record) => record?.kind === 'restored_lineage'),
      true,
    );

    assertCandidateIdentity(candC, 'person-022');
    assert.equal(candC.houseName, idx.houseById[idx.personById['person-022'].house_id].name);
    assert.equal(candC.rights.origin, 'derived');
    assert.equal(candC.rights.claimRecordId, 'derived-claim-01');
    assert.equal(candC.rights.sourceClaimId, 'claim-02');
    assert.equal(candC.rights.priority, 'restored_contested_derived');
    assert.equal(candC.rights.standingLabel, '혈통을 따라 파생된 복권 권리');
    assert.equal(candC.rights.generationDistance, derivedById['derived-claim-01'].generation_distance);
    assert.equal(candC.lineage.kind, 'derived');
    assert.equal(candC.lineage.sourcePersonId, 'person-019');
    assert.equal(candC.lineage.sourcePersonName, idx.personById['person-019'].name);
    assert.equal(candC.lineage.sourceClaimId, 'claim-02');
    assert.match(candC.lineage.label, /자녀로서/);

    for (const detail of [candA, candB, candC]) {
      assert.equal('strengths' in detail, false);
      assert.equal('weaknesses' in detail, false);
      assert.equal('supportStatus' in detail, false);
      const visible = getVisibleInformation(idx, detail.personId);
      assert.deepEqual(
        detail.information.map((item) => item.id).sort(),
        visible.map((item) => item.id).sort(),
      );
    }

    const house01 = getSuccessionHouseDetail(idx, 'realm-01', 'house-01');
    const house02 = getSuccessionHouseDetail(idx, 'realm-01', 'house-02');
    const house03 = getSuccessionHouseDetail(idx, 'realm-01', 'house-03');
    assert.equal(house01.headStatus.isDeceasedHead, true);
    assert.equal(house01.informationLabel, '직전 수장이 사망 전에 알고 있던 정보');
    assert.ok(house01.promises.some((item) => item.sentence.includes('직전 통치자가 생전에')));
    assert.equal(house01.promises.some((item) => /약속했습니다/.test(item.sentence)), false);

    const head01Info = getVisibleInformation(idx, idx.houseById['house-01'].head_person_id);
    const head02Info = getVisibleInformation(idx, idx.houseById['house-02'].head_person_id);
    const head03Info = getVisibleInformation(idx, idx.houseById['house-03'].head_person_id);
    assert.deepEqual(
      house01.information.map((item) => item.id).sort(),
      head01Info.map((item) => item.id).sort(),
    );
    assert.deepEqual(
      house02.information.map((item) => item.id).sort(),
      head02Info.map((item) => item.id).sort(),
    );
    assert.deepEqual(
      house03.information.map((item) => item.id).sort(),
      head03Info.map((item) => item.id).sort(),
    );

    assert.equal(
      house01.information.some((item) => item.topic === 'promise_conflict' && item.confidence === 'confirmed'),
      true,
    );
    assert.equal(
      house02.information.some((item) => item.topic === 'promise_conflict' && item.confidence === 'unverified'),
      true,
    );
    assert.equal(
      house02.information.some((item) => item.topic === 'promise_conflict' && item.confidence === 'confirmed'),
      false,
    );
    assert.equal(
      house03.information.some((item) => item.topic === 'promise_conflict'),
      false,
    );

    const hiddenFromHouse03 = idx.layers.context.information.filter(
      (item) =>
        item.scope === 'private' &&
        !(item.known_by_person_ids ?? []).includes(idx.houseById['house-03'].head_person_id),
    );
    const house03Ids = new Set(house03.information.map((item) => item.id));
    for (const item of hiddenFromHouse03) {
      assert.equal(house03Ids.has(item.id), false);
    }

    const kindLabel = { cooperative: '협력', rival: '대립', competitive: '경쟁' };
    const rawRelations = idx.layers.context.relations.filter(
      (rel) => rel.house_a_id === 'house-01' || rel.house_b_id === 'house-01',
    );
    assert.equal(house01.relations.length, rawRelations.length);
    for (const rel of rawRelations) {
      const otherHouseId = rel.house_a_id === 'house-01' ? rel.house_b_id : rel.house_a_id;
      const projected = house01.relations.find(
        (item) => item.otherHouseId === otherHouseId && item.kind === rel.kind,
      );
      assert.ok(projected, `missing relation ${rel.kind} ${otherHouseId}`);
      assert.equal(projected.kind, rel.kind);
      assert.equal(projected.kindLabel, kindLabel[rel.kind]);
      assert.equal(
        projected.sentence,
        `${idx.houseById['house-01'].name}와 ${idx.houseById[otherHouseId].name}는 ${kindLabel[rel.kind]} 관계입니다.`,
      );
    }
    assert.equal(getSuccessionDisputeView(idx, 'realm-02'), null);
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});

function personsOf(world) {
  return world.context_world.political.dynastic.population.persons;
}

function housesOf(world) {
  return world.context_world.political.dynastic.population.houses;
}

function assertNoFakeName(value, rawId) {
  assert.notEqual(value, rawId);
  assert.notEqual(value, '기록 없음');
}

test('missing succession person is not replaced with a fake display name', () => {
  const { outDir, world, succession } = exportSuccession(1, 'realm-01');
  try {
    const broken = structuredClone(world);
    personsOf(broken).splice(
      personsOf(broken).findIndex((person) => person.id === 'person-006'),
      1,
    );
    const idx = buildIndexes(broken, succession);
    const dispute = getSuccessionDisputeView(idx, 'realm-01');
    const candA = dispute.candidates.find((item) => item.slot === 'A');
    assert.ok(candA);
    assert.equal(candA.personId, 'person-006');
    assert.equal(candA.personName, null);
    assert.equal(candA.unresolved, true);
    assert.equal(dispute.unresolved, true);
    assertNoFakeName(candA.personName, 'person-006');
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});

test('malformed claim, evidence, house, and slot counts stay unresolved', () => {
  const { outDir, world, succession } = exportSuccession(1, 'realm-01');
  try {
    function disputeOf(nextWorld = world, nextSuccession = succession) {
      return getSuccessionDisputeView(buildIndexes(nextWorld, nextSuccession), 'realm-01');
    }

    const missingOriginal = structuredClone(world);
    missingOriginal.rights.claims = missingOriginal.rights.claims.filter(
      (claim) => claim.id !== 'claim-01',
    );
    const noOriginal = disputeOf(missingOriginal);
    const noOriginalA = noOriginal.candidates.find((item) => item.slot === 'A');
    assert.equal(noOriginalA.unresolved, true);
    assert.equal(noOriginal.unresolved, true);
    assert.equal(noOriginalA.personName, personsOf(world).find((p) => p.id === 'person-006').name);
    assert.notEqual(noOriginalA.personName, 'person-006');

    const missingDerived = structuredClone(succession);
    missingDerived.pre_succession_world.propagation.derived_claims =
      missingDerived.pre_succession_world.propagation.derived_claims.filter(
        (item) => item.id !== 'derived-claim-01',
      );
    const noDerived = disputeOf(world, missingDerived);
    const noDerivedC = noDerived.candidates.find((item) => item.slot === 'C');
    assert.equal(noDerivedC.unresolved, true);
    assert.equal(noDerived.unresolved, true);
    assert.equal(noDerivedC.provenance.sentence, null);
    assert.equal(noDerivedC.provenance.sourcePersonId, null);
    assert.notEqual(noDerivedC.provenance.sourcePersonName, 'person-019');

    const missingSource = structuredClone(world);
    missingSource.rights.claims = missingSource.rights.claims.filter(
      (claim) => claim.id !== 'claim-02',
    );
    const noSource = disputeOf(missingSource);
    const noSourceC = noSource.candidates.find((item) => item.slot === 'C');
    const noSourceB = noSource.candidates.find((item) => item.slot === 'B');
    assert.equal(noSourceC.unresolved, true);
    assert.equal(noSourceB.unresolved, true);
    assert.equal(noSource.unresolved, true);
    assert.equal(noSourceC.provenance.sourcePersonId, null);
    assert.equal(noSourceC.provenance.sentence, null);
    assert.equal(noSourceC.provenance.sourcePersonName, null);

    const missingEvidence = structuredClone(world);
    const restored = missingEvidence.rights.claims.find((claim) => claim.id === 'claim-02');
    const evidenceIds = new Set(restored.evidence_record_ids ?? []);
    restored.evidence_record_ids = [];
    missingEvidence.rights.evidence_records = missingEvidence.rights.evidence_records.filter(
      (record) => !evidenceIds.has(record.id),
    );
    const noEvidence = disputeOf(missingEvidence);
    const noEvidenceB = noEvidence.candidates.find((item) => item.slot === 'B');
    assert.equal(noEvidenceB.unresolved, true);
    assert.equal(noEvidence.unresolved, true);
    assert.equal(noEvidenceB.evidenceLabel, null);
    assert.notEqual(noEvidenceB.evidenceLabel, '연결된 역사 기록 없음');
    assert.notEqual(noEvidenceB.personName, 'person-019');

    const missingHouse = structuredClone(world);
    housesOf(missingHouse).splice(
      housesOf(missingHouse).findIndex((house) => house.id === 'house-03'),
      1,
    );
    const noHouse = disputeOf(missingHouse);
    const noHouseB = noHouse.candidates.find((item) => item.slot === 'B');
    assert.equal(noHouseB.houseName, null);
    assert.equal(noHouseB.unresolved, true);
    assert.equal(noHouse.unresolved, true);
    assert.notEqual(noHouseB.houseName, 'house-03');
    assert.equal(noHouse.houses.length, 2);

    const extraCandidate = structuredClone(succession);
    extraCandidate.transition.candidates = [
      ...extraCandidate.transition.candidates,
      { ...extraCandidate.transition.candidates[0], person_id: 'person-007' },
    ];
    const four = disputeOf(world, extraCandidate);
    assert.equal(four.unresolved, true);
    assert.equal(four.candidates.length, 3);

    const duplicatePriority = structuredClone(succession);
    duplicatePriority.transition.candidates[2] = {
      ...duplicatePriority.transition.candidates[2],
      priority: 'direct_strong_original',
      claim_origin: 'original',
    };
    const dup = disputeOf(world, duplicatePriority);
    assert.equal(dup.unresolved, true);

    const extraHouse = structuredClone(world);
    const copied = structuredClone(housesOf(extraHouse).find((house) => house.id === 'house-02'));
    copied.id = 'house-99';
    housesOf(extraHouse).push(copied);
    const fourHouses = disputeOf(extraHouse);
    assert.equal(fourHouses.houses.length, 4);
    assert.equal(fourHouses.unresolved, true);
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});
