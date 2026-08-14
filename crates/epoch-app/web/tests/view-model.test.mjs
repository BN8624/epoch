// RightsWorld export JSON을 읽기 전용 view-model 투영으로 검증한다
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import {
  buildIndexes,
  getClaimsForPerson,
  getHouseRelations,
  getInitialSelection,
  getMapTiles,
  getPersonView,
  getRealmView,
  getVisibleInformation,
  getVisiblePromises,
  getWorldSummary,
  housesForRealm,
  membersForHouse,
  rulingHouseIdForRealm,
} from '../view-model.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WORKSPACE = path.resolve(HERE, '../../../..');

function exportWorld(seed) {
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), `epoch-app-vm-${seed}-`));
  const result = spawnSync(
    'cargo',
    ['run', '-q', '-p', 'epoch-app', '--', 'export', String(seed), outDir],
    {
      cwd: WORKSPACE,
      encoding: 'utf8',
      shell: process.platform === 'win32',
    },
  );
  if (result.status !== 0) {
    throw new Error(
      `export seed=${seed} failed:\n${result.stdout || ''}\n${result.stderr || ''}`,
    );
  }
  const world = JSON.parse(fs.readFileSync(path.join(outDir, 'rights-world.json'), 'utf8'));
  return { outDir, world, stdout: result.stdout };
}

function realm01Actors(idx) {
  const houses = housesForRealm(idx, 'realm-01');
  assert.equal(houses.length, 3);
  const ruling = houses.find((house) => house.ruling);
  const nonRuling = houses.filter((house) => !house.ruling);
  assert.ok(ruling);
  assert.equal(nonRuling.length, 2);
  const realmActors = idx.layers.roster.active_actors.filter(
    (actor) => actor.realm_id === 'realm-01',
  );
  const ruler = realmActors.find((actor) => actor.primary_role === 'ruler');
  const rhc = realmActors.find((actor) => actor.primary_role === 'ruling_house_current');
  assert.ok(ruler);
  assert.ok(rhc);
  return {
    houses,
    rulerId: ruler.person_id,
    rulingHouseCurrentId: rhc.person_id,
    firstNonRulingHeadId: nonRuling[0].headPersonId,
    secondNonRulingHeadId: nonRuling[1].headPersonId,
  };
}

function expectedPromiseIds(idx, personId) {
  return new Set(
    idx.layers.context.promises
      .filter((promise) => (promise.known_by_person_ids ?? []).includes(personId))
      .map((promise) => promise.id),
  );
}

function expectedInformationIds(idx, personId) {
  return new Set(
    idx.layers.context.information
      .filter((item) => {
        if (item.scope === 'public') return true;
        if (item.scope === 'private') {
          return (item.known_by_person_ids ?? []).includes(personId);
        }
        return false;
      })
      .map((item) => item.id),
  );
}

function assertIdSet(actualItems, expectedIds) {
  const actualIds = actualItems.map((item) => item.id);
  assert.equal(actualIds.length, new Set(actualIds).size);
  assert.deepEqual([...new Set(actualIds)].sort(), [...expectedIds].sort());
}

function hasConflict(info, confidence) {
  return info.some(
    (item) => item.topic === 'promise_conflict' && item.confidence === confidence,
  );
}

test('seed 1 view-model projection and realm-01 asymmetry', () => {
  const { outDir, world, stdout } = exportWorld(1);
  try {
    assert.match(stdout, /APP_EXPORT_OK seed=1 rights_bytes=66222 files=5/);
    const idx = buildIndexes(world);
    const summary = getWorldSummary(idx);
    assert.equal(summary.seed, 1);
    assert.equal(summary.realmCount, 6);
    assert.equal(summary.territoryCount, 36);
    assert.equal(summary.houseCount, 18);
    assert.equal(summary.personCount, 144);
    assert.equal(summary.claimCount, 12);

    const tiles = getMapTiles(idx);
    assert.equal(tiles.length, 36);
    for (const tile of tiles) {
      const territory = idx.territoryById[tile.id];
      assert.equal(tile.x, territory.x);
      assert.equal(tile.y, territory.y);
      const realm = idx.realmById[territory.realm_id];
      assert.equal(tile.isCapital, realm.capital_territory_id === tile.id);
    }

    const initial = getInitialSelection(idx);
    const firstRealm = [...idx.layers.skeleton.realms].sort((a, b) => a.id.localeCompare(b.id))[0];
    assert.equal(initial.selectedRealmId, firstRealm.id);
    assert.equal(initial.selectedTerritoryId, firstRealm.capital_territory_id);
    const firstHouses = housesForRealm(idx, firstRealm.id);
    const incumbentId = idx.rightsByRealm[firstRealm.id].incumbent_person_id;
    const incumbentHouseId = idx.personById[incumbentId].house_id;
    assert.equal(firstHouses.length, 3);
    assert.equal(rulingHouseIdForRealm(idx, firstRealm.id), incumbentHouseId);
    assert.equal(initial.selectedHouseId, incumbentHouseId);
    assert.equal(initial.selectedPersonId, incumbentId);
    const rulingHouse = firstHouses.find((house) => house.ruling);
    assert.ok(rulingHouse);
    assert.equal(rulingHouse.id, incumbentHouseId);
    assert.equal(firstHouses.filter((h) => h.ruling).length, 1);

    const members = membersForHouse(idx, rulingHouse.id);
    assert.equal(members.all.length, 8);
    assert.equal(members.elder.length, 2);
    assert.equal(members.current.length, 3);
    assert.equal(members.young.length, 3);

    const realmView = getRealmView(idx, firstRealm.id);
    assert.ok(realmView.incumbentName);
    assert.equal(realmView.claims.length, 2);
    const rawDirect = (idx.claimsByRealm[firstRealm.id] ?? []).find(
      (claim) => claim.basis === 'direct_descent',
    );
    const rawRestored = (idx.claimsByRealm[firstRealm.id] ?? []).find(
      (claim) => claim.basis === 'restored_line_record',
    );
    assert.equal(rawDirect.standing, 'strong');
    assert.equal(rawRestored.standing, 'contested');
    assert.equal(realmView.claims[0].kind, 'direct');
    assert.equal(realmView.claims[0].standing, 'strong');
    assert.equal(realmView.claims[0].standingLabel, '강한 직계 권리');
    assert.equal(realmView.claims[1].kind, 'restored');
    assert.equal(realmView.claims[1].standing, 'contested');
    assert.equal(realmView.claims[1].standingLabel, '논쟁 중인 복권 권리');
    assert.ok((rawRestored.evidence_record_ids ?? []).length > 0);
    assert.ok(
      rawRestored.evidence_record_ids.every((id) => idx.evidenceById[id]),
    );
    assert.match(realmView.claims[1].evidenceLabel, /역사 기록/);

    const directPerson = getPersonView(idx, realmView.claims[0].personId);
    assert.equal(directPerson.generation, 'young');
    assert.equal(directPerson.isActive, false);
    assert.equal(directPerson.activityLabel, '보조 인물');
    assert.ok(directPerson.parentNames.includes(realmView.incumbentName));
    assert.equal(getClaimsForPerson(idx, directPerson.id)[0].kind, 'direct');

    const elder = members.elder[0];
    const elderView = getPersonView(idx, elder.id);
    assert.equal(elderView.parentLabel, '알려진 부모 기록 없음');

    const relations = getHouseRelations(idx, rulingHouse.id);
    assert.ok(relations.length >= 1);
    for (const rel of relations) {
      assert.match(rel.sentence, /관계입니다/);
      assert.ok(['협력', '대립', '경쟁'].includes(rel.kindLabel));
      assert.ok(!/cooperative|rival|competitive/i.test(rel.sentence));
    }

    const actors = realm01Actors(idx);
    const rulerInfo = getVisibleInformation(idx, actors.rulerId);
    const rhcInfo = getVisibleInformation(idx, actors.rulingHouseCurrentId);
    const firstHeadInfo = getVisibleInformation(idx, actors.firstNonRulingHeadId);
    const secondHeadInfo = getVisibleInformation(idx, actors.secondNonRulingHeadId);

    const visibilitySubjects = [
      actors.rulerId,
      actors.rulingHouseCurrentId,
      actors.firstNonRulingHeadId,
      actors.secondNonRulingHeadId,
    ];
    for (const personId of visibilitySubjects) {
      assertIdSet(getVisiblePromises(idx, personId), expectedPromiseIds(idx, personId));
      assertIdSet(
        getVisibleInformation(idx, personId),
        expectedInformationIds(idx, personId),
      );
    }

    const publicCount = idx.layers.context.information.filter((i) => i.scope === 'public').length;
    assert.equal(rulerInfo.filter((i) => i.scope === 'public').length, publicCount);
    assert.equal(hasConflict(rulerInfo, 'confirmed'), true);
    assert.equal(hasConflict(rhcInfo, 'confirmed'), true);
    assert.equal(hasConflict(firstHeadInfo, 'unverified'), true);
    assert.equal(hasConflict(rulerInfo, 'unverified'), false);
    assert.equal(hasConflict(rhcInfo, 'unverified'), false);
    assert.equal(hasConflict(firstHeadInfo, 'confirmed'), false);
    assert.equal(hasConflict(secondHeadInfo, 'confirmed'), false);
    assert.equal(hasConflict(secondHeadInfo, 'unverified'), false);
    assert.equal(
      secondHeadInfo.filter((i) => i.topic === 'promise_conflict').length,
      0,
    );

    const hidden = idx.layers.context.information.filter(
      (item) =>
        item.scope === 'private' &&
        !(item.known_by_person_ids ?? []).includes(actors.secondNonRulingHeadId),
    );
    const visibleIds = new Set(secondHeadInfo.map((i) => i.id));
    for (const item of hidden) {
      assert.equal(visibleIds.has(item.id), false);
    }
    assert.equal(
      secondHeadInfo.some((i) => /숨겨진 정보/.test(`${i.badge} ${i.body}`)),
      false,
    );

    const rulerPromises = getVisiblePromises(idx, actors.rulerId);
    const secondHeadPromises = getVisiblePromises(idx, actors.secondNonRulingHeadId);
    assert.ok(!secondHeadPromises[0].sentence.includes('reward:'));
    assert.equal(rulerPromises.length, expectedPromiseIds(idx, actors.rulerId).size);
    assert.equal(
      secondHeadPromises.length,
      expectedPromiseIds(idx, actors.secondNonRulingHeadId).size,
    );
  } finally {
    fs.rmSync(outDir, { recursive: true, force: true });
  }
});
