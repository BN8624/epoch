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
  getCrisisView,
  getRealmView,
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
