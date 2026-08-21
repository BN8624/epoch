// RightsWorld JSON을 읽기 전용 관찰 화면용 뷰로 투영한다

const GENERATION_LABEL = {
  elder: '노년 세대',
  current: '현재 세대',
  young: '후속 세대',
};

const RELATION_LABEL = {
  cooperative: '협력',
  rival: '대립',
  competitive: '경쟁',
};

const ROLE_LABEL = {
  ruler: '통치자',
  house_head: '가문 수장',
  ruling_house_current: '통치 가문 핵심 인물',
};

function layersOf(world) {
  const contextWorld = world.context_world;
  const political = contextWorld.political;
  const dynastic = political.dynastic;
  return {
    seed: world.seed,
    skeleton: dynastic.world,
    population: dynastic.population,
    roster: political.roster,
    context: contextWorld.context,
    rights: world.rights,
  };
}

function byId(list, key = 'id') {
  const map = Object.create(null);
  for (const item of list) map[item[key]] = item;
  return map;
}

export function realmShortLabel(realmId) {
  const match = String(realmId).match(/(\d+)$/);
  return match ? `R${Number(match[1], 10)}` : String(realmId);
}

export function generationLabel(generation) {
  return GENERATION_LABEL[generation] ?? generation;
}

export function identityStance(houseIdentity, realmIdentity) {
  const cultureMajor = houseIdentity.culture_id === realmIdentity.majority_culture_id;
  const religionMajor = houseIdentity.religion_id === realmIdentity.majority_religion_id;
  if (cultureMajor && religionMajor) return '다수 문화 · 다수 종교';
  if (cultureMajor && !religionMajor) return '다수 문화 · 종교 소수';
  if (!cultureMajor && religionMajor) return '문화 소수 · 다수 종교';
  return '문화 소수 · 종교 소수';
}

function nameOf(map, id, fallback = id) {
  return map[id]?.name ?? fallback;
}

function rewardPhrase(rewardKey, realmName) {
  if (typeof rewardKey === 'string' && /:council-seat$/.test(rewardKey)) {
    return `${realmName}의 평의회 자리`;
  }
  return `${realmName}에 대한 약속된 보상`;
}

function standingWord(standing) {
  if (standing === 'strong') return '강한';
  if (standing === 'contested') return '논쟁 중인';
  return null;
}

function claimKindFromBasis(basis) {
  if (basis === 'direct_descent') {
    return { kind: 'direct', title: '직계 권리자', noun: '직계 권리' };
  }
  if (basis === 'restored_line_record') {
    return { kind: 'restored', title: '복권 권리자', noun: '복권 권리' };
  }
  return { kind: basis, title: '계승 권리', noun: '계승 권리' };
}

function claimEvidenceLabel(claim, evidenceById) {
  if (claim.basis === 'direct_descent') {
    return '현 통치자의 알려진 자녀';
  }
  const records = (claim.evidence_record_ids ?? [])
    .map((id) => evidenceById?.[id])
    .filter(Boolean);
  if (records.some((record) => record.kind === 'restored_lineage')) {
    return '옛 계통을 뒷받침하는 역사 기록 보유';
  }
  if (records.length > 0) {
    return '연결된 역사 기록';
  }
  return '연결된 역사 기록 없음';
}

function claimProjection(claim, idx) {
  const person = idx.personById[claim.claimant_person_id];
  const kindInfo = claimKindFromBasis(claim.basis);
  const strength = standingWord(claim.standing);
  return {
    id: claim.id,
    kind: kindInfo.kind,
    title: kindInfo.title,
    standing: claim.standing,
    standingLabel: strength ? `${strength} ${kindInfo.noun}` : kindInfo.noun,
    evidenceLabel: claimEvidenceLabel(claim, idx.evidenceById),
    personId: claim.claimant_person_id,
    personName: person?.name ?? claim.claimant_person_id,
    houseId: claim.claimant_house_id,
  };
}

function informationProjection(item, realmById) {
  const scopeLabel = item.scope === 'public' ? '공개' : '비공개';
  const confidenceLabel = item.confidence === 'confirmed' ? '확인됨' : '미확인';
  let body = '';
  if (item.topic === 'religious_minority') {
    body = '이 Realm에는 다수 종교와 다른 종교를 따르는 유력 가문이 있습니다.';
  } else if (item.topic === 'promise_conflict') {
    body =
      item.confidence === 'confirmed'
        ? '통치자가 같은 평의회 자리를 두 가문에 약속했습니다.'
        : '같은 보상이 다른 가문에도 약속됐을 가능성이 있다는 소문이 있습니다.';
  }
  return {
    id: item.id,
    realmId: item.realm_id,
    realmName: realmById[item.realm_id]?.name ?? item.realm_id,
    topic: item.topic,
    scope: item.scope,
    confidence: item.confidence,
    badge: `${scopeLabel} · ${confidenceLabel}`,
    body,
  };
}

const SUCCESSION_PRIORITY_COPY = {
  direct_strong_original: {
    role: 'priority',
    title: '법적 우선 후보',
    standingLabel: '강한 직계 권리',
    reason: '통치자의 직계 자녀이므로 법적 우선 후보',
  },
  restored_contested_original: {
    role: 'competing',
    title: '경쟁 권리',
    standingLabel: '논쟁 중인 복권 권리',
    reason: '공통 역사 기록을 가진 가문 주장',
  },
  restored_contested_derived: {
    role: 'competing',
    title: '경쟁 권리',
    standingLabel: '혈통을 따라 파생된 복권 권리',
    reason: '복권 권리가 부모에서 혈통을 따라 한 세대 전파됨',
  },
};

export function buildIndexes(world, successionWorld = null) {
  const layers = layersOf(world);
  const territoryById = byId(layers.skeleton.territories);
  const realmById = byId(layers.skeleton.realms);
  const rulerById = byId(layers.skeleton.rulers);
  const houseById = byId(layers.population.houses);
  const personById = byId(layers.population.persons);
  const cultureById = byId(layers.context.cultures);
  const religionById = byId(layers.context.religions);
  const realmIdentityById = byId(layers.context.realm_identities, 'realm_id');
  const houseIdentityById = byId(layers.context.house_identities, 'house_id');
  const personIdentityById = byId(layers.context.person_identities, 'person_id');
  const rightsByRealm = byId(layers.rights.realms, 'realm_id');
  const claimById = byId(layers.rights.claims);
  const evidenceById = byId(layers.rights.evidence_records);

  const rulerPersonByRealm = Object.create(null);
  for (const link of layers.population.ruler_links) {
    const person = personById[link.person_id];
    if (person) rulerPersonByRealm[person.realm_id] = person.id;
  }

  const activeByPerson = Object.create(null);
  for (const actor of layers.roster.active_actors) {
    activeByPerson[actor.person_id] = actor;
  }
  const supporting = new Set(layers.roster.supporting_person_ids);

  const claimsByPerson = Object.create(null);
  const claimsByRealm = Object.create(null);
  for (const claim of layers.rights.claims) {
    (claimsByPerson[claim.claimant_person_id] ??= []).push(claim);
    (claimsByRealm[claim.realm_id] ??= []).push(claim);
  }

  const housesByRealm = Object.create(null);
  for (const house of layers.population.houses) {
    (housesByRealm[house.realm_id] ??= []).push(house);
  }
  for (const realmId of Object.keys(housesByRealm)) {
    housesByRealm[realmId].sort((a, b) => a.id.localeCompare(b.id));
  }

  const derivedById = Object.create(null);
  for (const derived of successionWorld?.pre_succession_world?.propagation?.derived_claims ?? []) {
    derivedById[derived.id] = derived;
  }

  return {
    world,
    succession: successionWorld,
    derivedById,
    layers,
    territoryById,
    realmById,
    rulerById,
    houseById,
    personById,
    cultureById,
    religionById,
    realmIdentityById,
    houseIdentityById,
    personIdentityById,
    rightsByRealm,
    claimById,
    evidenceById,
    rulerPersonByRealm,
    activeByPerson,
    supporting,
    claimsByPerson,
    claimsByRealm,
    housesByRealm,
  };
}

export function getWorldSummary(idx) {
  const { layers } = idx;
  return {
    seed: layers.seed,
    realmCount: layers.skeleton.realms.length,
    territoryCount: layers.skeleton.territories.length,
    houseCount: layers.population.houses.length,
    personCount: layers.population.persons.length,
    activeCount: layers.roster.active_actors.length,
    claimCount: layers.rights.claims.length,
  };
}

export function getMapTiles(idx) {
  return idx.layers.skeleton.territories.map((territory) => {
    const realm = idx.realmById[territory.realm_id];
    const isCapital = realm?.capital_territory_id === territory.id;
    const realmName = realm?.name ?? territory.realm_id;
    return {
      id: territory.id,
      x: territory.x,
      y: territory.y,
      realmId: territory.realm_id,
      realmName,
      shortLabel: realmShortLabel(territory.realm_id),
      isCapital,
      accessibleName: isCapital
        ? `${realmName}, ${territory.id}, 수도`
        : `${realmName}, ${territory.id}`,
    };
  });
}

export function rulingHouseIdForRealm(idx, realmId) {
  const incumbentId =
    idx.rightsByRealm[realmId]?.incumbent_person_id ?? idx.rulerPersonByRealm[realmId];
  return idx.personById[incumbentId]?.house_id ?? null;
}

export function getInitialSelection(idx) {
  const realms = [...idx.layers.skeleton.realms].sort((a, b) => a.id.localeCompare(b.id));
  const realm = realms[0];
  const houses = housesForRealm(idx, realm.id);
  const ruling = houses.find((house) => house.ruling) ?? houses[0];
  const rights = idx.rightsByRealm[realm.id];
  return {
    selectedTerritoryId: realm.capital_territory_id,
    selectedRealmId: realm.id,
    selectedHouseId: ruling?.id ?? null,
    selectedPersonId: rights?.incumbent_person_id ?? ruling?.headPersonId ?? null,
  };
}

export function housesForRealm(idx, realmId) {
  const realmIdentity = idx.realmIdentityById[realmId];
  const rulingHouseId = rulingHouseIdForRealm(idx, realmId);
  return (idx.housesByRealm[realmId] ?? []).map((house) => {
    const identity = idx.houseIdentityById[house.id];
    const head = idx.personById[house.head_person_id];
    const seat = idx.territoryById[house.seat_territory_id];
    const ruling = house.id === rulingHouseId;
    return {
      id: house.id,
      name: house.name,
      ruling,
      rulingLabel: ruling ? '통치 가문' : null,
      headPersonId: house.head_person_id,
      headName: head?.name ?? house.head_person_id,
      seatTerritoryId: house.seat_territory_id,
      seatLabel: seat ? `${seat.id} (${seat.x}, ${seat.y})` : house.seat_territory_id,
      cultureName: nameOf(idx.cultureById, identity?.culture_id),
      religionName: nameOf(idx.religionById, identity?.religion_id),
      identityStance: identity && realmIdentity ? identityStance(identity, realmIdentity) : '',
      memberIds: [...house.member_ids],
    };
  });
}

export function getHouseRelations(idx, houseId) {
  const house = idx.houseById[houseId];
  if (!house) return [];
  const out = [];
  for (const rel of idx.layers.context.relations) {
    if (rel.house_a_id !== houseId && rel.house_b_id !== houseId) continue;
    const otherId = rel.house_a_id === houseId ? rel.house_b_id : rel.house_a_id;
    const other = idx.houseById[otherId];
    const kindLabel = RELATION_LABEL[rel.kind] ?? rel.kind;
    out.push({
      otherHouseId: otherId,
      otherHouseName: other?.name ?? otherId,
      kind: rel.kind,
      kindLabel,
      sentence: `${house.name}와 ${other?.name ?? otherId}는 ${kindLabel} 관계입니다.`,
    });
  }
  return out;
}

export function getHouseView(idx, houseId) {
  const house = idx.houseById[houseId];
  if (!house) return null;
  const houses = housesForRealm(idx, house.realm_id);
  const view = houses.find((item) => item.id === houseId);
  if (!view) return null;
  return {
    ...view,
    realmId: house.realm_id,
    realmName: idx.realmById[house.realm_id]?.name ?? house.realm_id,
    relations: getHouseRelations(idx, houseId),
    members: membersForHouse(idx, houseId),
  };
}

export function personBadges(idx, personId) {
  const badges = [];
  const actor = idx.activeByPerson[personId];
  if (actor) {
    badges.push({ key: 'active', label: '적극적 정치 행위자' });
    const reasons = actor.activation_reasons ?? [];
    for (const reason of reasons) {
      const label = ROLE_LABEL[reason];
      if (label) badges.push({ key: reason, label });
    }
    if (reasons.length === 0 && actor.primary_role) {
      const label = ROLE_LABEL[actor.primary_role];
      if (label) badges.push({ key: actor.primary_role, label });
    }
  } else if (idx.supporting.has(personId)) {
    badges.push({ key: 'supporting', label: '보조 인물' });
  }
  for (const claim of getClaimsForPerson(idx, personId)) {
    badges.push({
      key: claim.kind,
      label: claim.kind === 'direct' ? '직계 권리자' : '복권 권리자',
    });
  }
  return badges;
}

export function membersForHouse(idx, houseId) {
  const house = idx.houseById[houseId];
  if (!house) return { elder: [], current: [], young: [], all: [] };
  const all = house.member_ids.map((id) => {
    const person = idx.personById[id];
    return {
      id,
      name: person?.name ?? id,
      generation: person?.generation,
      generationLabel: generationLabel(person?.generation),
      badges: personBadges(idx, id),
      activityLabel: idx.activeByPerson[id] ? '적극적 정치 행위자' : '보조 인물',
    };
  });
  return {
    all,
    elder: all.filter((p) => p.generation === 'elder'),
    current: all.filter((p) => p.generation === 'current'),
    young: all.filter((p) => p.generation === 'young'),
  };
}

export function getClaimsForPerson(idx, personId) {
  return (idx.claimsByPerson[personId] ?? []).map((claim) => claimProjection(claim, idx));
}

export function getVisiblePromises(idx, personId) {
  return idx.layers.context.promises
    .filter((promise) => (promise.known_by_person_ids ?? []).includes(personId))
    .map((promise) => {
      const promisor = idx.personById[promise.promisor_person_id];
      const promisee = idx.personById[promise.promisee_person_id];
      const realm = idx.realmById[promise.realm_id];
      const realmName = realm?.name ?? promise.realm_id;
      return {
        id: promise.id,
        realmId: promise.realm_id,
        sentence: `${promisor?.name ?? promise.promisor_person_id}이 ${
          promisee?.name ?? promise.promisee_person_id
        }에게 ${rewardPhrase(promise.reward_key, realmName)}를 약속했습니다.`,
      };
    });
}

export function getVisibleInformation(idx, personId) {
  return idx.layers.context.information
    .filter((item) => {
      if (item.scope === 'public') return true;
      if (item.scope === 'private') {
        return (item.known_by_person_ids ?? []).includes(personId);
      }
      return false;
    })
    .map((item) => informationProjection(item, idx.realmById));
}

export function getSuccessionOverlay(idx, realmId) {
  const transition = idx.succession?.transition;
  if (!transition || transition.realm_id !== realmId) return null;
  return transition;
}

const SUCCESSION_SLOT_BY_PRIORITY = {
  direct_strong_original: {
    slot: 'A',
    slotLabel: '후보 A',
    badge: '법적 우선 후보',
    standingLabel: '강한 직계 권리',
    priorityLabel: '강한 직계 권리',
  },
  restored_contested_original: {
    slot: 'B',
    slotLabel: '후보 B',
    badge: '경쟁 권리자',
    standingLabel: '논쟁 중인 복권 권리',
    priorityLabel: '논쟁 중인 복권 권리',
  },
  restored_contested_derived: {
    slot: 'C',
    slotLabel: '후보 C',
    badge: '경쟁 권리자',
    standingLabel: '혈통을 따라 파생된 복권 권리',
    priorityLabel: '혈통을 따라 파생된 복권 권리',
  },
};

function resolvedName(entity) {
  return entity && typeof entity.name === 'string' && entity.name.length > 0 ? entity.name : null;
}

function successionDeceasedPersonId(idx) {
  return idx.succession?.transition?.death?.person_id ?? null;
}

function isDeceasedHouseHead(idx, house) {
  const transition = idx.succession?.transition;
  if (!transition || !house) return false;
  return (
    house.realm_id === transition.realm_id && house.head_person_id === transition.death.person_id
  );
}

export function getSuccessionHeadStatus(idx, houseId) {
  const house = idx.houseById[houseId];
  if (!house) return null;
  const head = idx.personById[house.head_person_id];
  const headName = resolvedName(head);
  const deceased = isDeceasedHouseHead(idx, house);
  if (deceased) {
    return {
      houseId,
      isDeceasedHead: true,
      recordedHeadPersonId: house.head_person_id,
      recordedHeadName: headName,
      currentHeadPersonId: null,
      currentHeadName: null,
      currentHeadUndecided: true,
      cardHeadLines: [
        headName ? `기존 수장: ${headName} · 사망` : null,
        '현재 수장: 미결정',
      ].filter(Boolean),
      detailHeadLines: [
        headName ? `기존 수장: ${headName} · 사망` : null,
        '현재 수장: 미결정',
      ].filter(Boolean),
    };
  }
  return {
    houseId,
    isDeceasedHead: false,
    recordedHeadPersonId: house.head_person_id,
    recordedHeadName: headName,
    currentHeadPersonId: house.head_person_id,
    currentHeadName: headName,
    currentHeadUndecided: false,
    cardHeadLines: [headName ? `수장: ${headName}` : null].filter(Boolean),
    detailHeadLines: [headName ? `수장: ${headName}` : null].filter(Boolean),
  };
}

function successionPromiseSentence(idx, promise) {
  const promisor = idx.personById[promise.promisor_person_id];
  const promisee = idx.personById[promise.promisee_person_id];
  const realm = idx.realmById[promise.realm_id];
  const realmName = realm?.name ?? promise.realm_id;
  const reward = rewardPhrase(promise.reward_key, realmName);
  const deceasedId = successionDeceasedPersonId(idx);
  const promiseeName = resolvedName(promisee) ?? promise.promisee_person_id;
  if (deceasedId && promise.promisor_person_id === deceasedId) {
    return `직전 통치자가 생전에 ${promiseeName}에게 ${reward}를 약속함`;
  }
  const promisorName = resolvedName(promisor) ?? promise.promisor_person_id;
  return `${promisorName}이 ${promiseeName}에게 ${reward}를 약속했습니다.`;
}

export function getSuccessionVisiblePromises(idx, personId) {
  return idx.layers.context.promises
    .filter((promise) => (promise.known_by_person_ids ?? []).includes(personId))
    .map((promise) => ({
      id: promise.id,
      realmId: promise.realm_id,
      sentence: successionPromiseSentence(idx, promise),
    }));
}

function derivedProvenance(idx, candidate) {
  const derived = idx.derivedById?.[candidate.claim_record_id];
  if (!derived) {
    return {
      sourceClaimId: null,
      sourcePersonId: null,
      sourcePersonName: null,
      viaParentPersonId: null,
      isKnownChildOfSource: false,
      sentence: null,
    };
  }
  const sourceClaim = idx.claimById[derived.source_claim_id];
  const sourcePersonId = sourceClaim?.claimant_person_id ?? derived.via_parent_person_id ?? null;
  const sourcePerson = sourcePersonId ? idx.personById[sourcePersonId] : null;
  const child = idx.personById[candidate.person_id];
  const isKnownChildOfSource = Boolean(
    sourcePersonId && (child?.known_parent_ids ?? []).includes(sourcePersonId),
  );
  const sourcePersonName = resolvedName(sourcePerson);
  return {
    sourceClaimId: derived.source_claim_id ?? null,
    sourcePersonId,
    sourcePersonName,
    viaParentPersonId: derived.via_parent_person_id ?? null,
    isKnownChildOfSource,
    sentence:
      sourcePersonName && isKnownChildOfSource
        ? `${sourcePersonName}의 자녀로서 복권 권리가 한 세대 전파됨`
        : null,
  };
}

function projectDisputeCandidate(idx, candidate, formerPersonId) {
  const slotInfo = SUCCESSION_SLOT_BY_PRIORITY[candidate.priority];
  const person = idx.personById[candidate.person_id];
  const house = idx.houseById[candidate.house_id];
  const claim =
    candidate.claim_origin === 'derived'
      ? null
      : idx.claimById[candidate.claim_record_id] ?? null;
  const provenance =
    candidate.claim_origin === 'derived' ? derivedProvenance(idx, candidate) : null;
  const sourceClaim = provenance?.sourceClaimId ? idx.claimById[provenance.sourceClaimId] : null;
  const evidenceClaim = claim ?? sourceClaim;
  const isKnownChildOfFormer = Boolean(
    formerPersonId && (person?.known_parent_ids ?? []).includes(formerPersonId),
  );
  const isRestoredLineHead = Boolean(house && house.head_person_id === candidate.person_id);
  const evidenceLabel = evidenceClaim
    ? claimEvidenceLabel(evidenceClaim, idx.evidenceById)
    : null;
  const actor = idx.activeByPerson[candidate.person_id];
  return {
    slot: slotInfo?.slot ?? null,
    slotLabel: slotInfo?.slotLabel ?? null,
    badge: slotInfo?.badge ?? null,
    personId: candidate.person_id,
    personName: resolvedName(person),
    houseId: candidate.house_id,
    houseName: resolvedName(house),
    generation: person?.generation ?? null,
    generationLabel: person ? generationLabel(person.generation) : null,
    activityLabel: actor ? '적극적 정치 행위자' : '보조 인물',
    isActive: Boolean(actor),
    claimRecordId: candidate.claim_record_id,
    origin: candidate.claim_origin,
    priority: candidate.priority,
    generationDistance: candidate.generation_distance,
    standingLabel: slotInfo?.standingLabel ?? null,
    priorityLabel: slotInfo?.priorityLabel ?? null,
    isPriority: candidate.priority === 'direct_strong_original',
    isKnownChildOfFormer,
    isRestoredLineHead,
    evidenceLabel:
      candidate.priority === 'direct_strong_original'
        ? isKnownChildOfFormer
          ? '직전 통치자의 알려진 자녀'
          : null
        : evidenceLabel,
    provenance,
    unresolved: !resolvedName(person) || !resolvedName(house) || !slotInfo,
  };
}

function politicalContextForHouse(idx, houseId, realmId) {
  const house = idx.houseById[houseId];
  const identity = idx.houseIdentityById[houseId];
  const realmIdentity = idx.realmIdentityById[realmId];
  const stance =
    identity && realmIdentity ? identityStance(identity, realmIdentity) : null;
  const relations = getHouseRelations(idx, houseId);
  const lines = [];
  if (stance) lines.push(stance);
  for (const rel of relations) {
    if (rel.otherHouseName) lines.push(`${rel.otherHouseName}와 ${rel.kindLabel} 관계`);
  }
  return {
    identityStance: stance,
    cultureName: nameOf(idx.cultureById, identity?.culture_id, null),
    religionName: nameOf(idx.religionById, identity?.religion_id, null),
    relations,
    lines,
  };
}

function projectDisputeHouse(idx, house, realmId) {
  const identity = idx.houseIdentityById[house.id];
  const realmIdentity = idx.realmIdentityById[realmId];
  const headStatus = getSuccessionHeadStatus(idx, house.id);
  const relations = getHouseRelations(idx, house.id);
  return {
    id: house.id,
    name: resolvedName(house),
    realmId: house.realm_id,
    cultureName: nameOf(idx.cultureById, identity?.culture_id, null),
    religionName: nameOf(idx.religionById, identity?.religion_id, null),
    identityStance:
      identity && realmIdentity ? identityStance(identity, realmIdentity) : null,
    headStatus,
    relationSummary: relations.map((rel) => rel.sentence),
    unresolved: !resolvedName(house),
  };
}

export function getSuccessionDisputeView(idx, realmId) {
  const transition = getSuccessionOverlay(idx, realmId);
  if (!transition) return null;
  const realm = idx.realmById[transition.realm_id];
  const former = idx.personById[transition.death.person_id];
  const mapped = (transition.candidates ?? []).map((candidate) =>
    projectDisputeCandidate(idx, candidate, transition.death.person_id),
  );
  const bySlot = Object.create(null);
  for (const candidate of mapped) {
    if (candidate.slot) bySlot[candidate.slot] = candidate;
  }
  const candidates = ['A', 'B', 'C'].map((slot) => bySlot[slot]).filter(Boolean);
  const houses = (idx.housesByRealm[transition.realm_id] ?? []).map((house) =>
    projectDisputeHouse(idx, house, transition.realm_id),
  );
  return {
    realmId: transition.realm_id,
    realmName: resolvedName(realm),
    formerIncumbentPersonId: transition.death.person_id,
    formerIncumbentName: resolvedName(former),
    vacant: transition.vacancy?.is_vacant === true,
    legalStatus: '법적 우선 후보가 있으나 계승은 확정되지 않음',
    presumptiveSuccessorPersonId: transition.presumptive_successor_person_id,
    presumptiveSuccessorHouseId: transition.presumptive_successor_house_id,
    candidates,
    candidateA: bySlot.A ?? null,
    candidateB: bySlot.B ?? null,
    candidateC: bySlot.C ?? null,
    houses,
    unresolved:
      !resolvedName(realm) ||
      !resolvedName(former) ||
      candidates.length !== 3 ||
      candidates.some((item) => item.unresolved) ||
      houses.some((item) => item.unresolved),
  };
}

export function getSuccessionCandidateDetail(idx, realmId, personId) {
  const dispute = getSuccessionDisputeView(idx, realmId);
  if (!dispute) return null;
  const card = dispute.candidates.find((item) => item.personId === personId);
  if (!card) return null;
  const person = getPersonView(idx, personId);
  if (!person) return null;
  const houseContext = politicalContextForHouse(idx, card.houseId, dispute.realmId);
  const politicalLines = [...houseContext.lines];
  if (person.activityLabel) politicalLines.push(person.activityLabel);
  const lineage =
    card.origin === 'derived'
      ? {
          kind: 'derived',
          label: card.provenance?.sentence ?? null,
          sourcePersonId: card.provenance?.sourcePersonId ?? null,
          sourcePersonName: card.provenance?.sourcePersonName ?? null,
          sourceClaimId: card.provenance?.sourceClaimId ?? null,
        }
      : {
          kind: 'direct',
          label: card.isKnownChildOfFormer ? '직전 통치자의 알려진 자녀' : null,
          sourcePersonId: dispute.formerIncumbentPersonId,
          sourcePersonName: dispute.formerIncumbentName,
          sourceClaimId: null,
        };
  return {
    slot: card.slot,
    slotLabel: card.slotLabel,
    badge: card.badge,
    personId: card.personId,
    name: card.personName,
    realmId: dispute.realmId,
    realmName: dispute.realmName,
    houseId: card.houseId,
    houseName: card.houseName,
    generation: card.generation,
    generationLabel: card.generationLabel,
    cultureName: person.cultureName,
    religionName: person.religionName,
    activityLabel: card.activityLabel,
    isActive: card.isActive,
    roleLabel: person.roleLabel,
    rights: {
      standingLabel: card.standingLabel,
      origin: card.origin,
      claimRecordId: card.claimRecordId,
      priority: card.priority,
      priorityLabel: card.priorityLabel,
      evidenceLabel: card.evidenceLabel,
      generationDistance: card.generationDistance,
      sourceClaimId: card.provenance?.sourceClaimId ?? null,
    },
    lineage,
    politicalContext: politicalLines,
    promises: getSuccessionVisiblePromises(idx, personId),
    information: getVisibleInformation(idx, personId),
    unresolved: card.unresolved || !card.personName,
  };
}

export function getSuccessionHouseDetail(idx, realmId, houseId) {
  const dispute = getSuccessionDisputeView(idx, realmId);
  if (!dispute) return null;
  const card = dispute.houses.find((item) => item.id === houseId);
  if (!card) return null;
  const house = idx.houseById[houseId];
  if (!house) return null;
  const realmIdentity = idx.realmIdentityById[realmId];
  const identity = idx.houseIdentityById[houseId];
  const headStatus = card.headStatus;
  const knowledgePersonId = house.head_person_id;
  const information = getVisibleInformation(idx, knowledgePersonId);
  const promises = getSuccessionVisiblePromises(idx, knowledgePersonId);
  return {
    id: house.id,
    name: card.name,
    realmId: house.realm_id,
    realmName: dispute.realmName,
    headStatus,
    cultureName: card.cultureName,
    religionName: card.religionName,
    identityStance: card.identityStance,
    majorityCultureName: nameOf(idx.cultureById, realmIdentity?.majority_culture_id, null),
    majorityReligionName: nameOf(idx.religionById, realmIdentity?.majority_religion_id, null),
    relations: getHouseRelations(idx, houseId),
    promises,
    information,
    informationLabel: headStatus.isDeceasedHead
      ? '직전 수장이 사망 전에 알고 있던 정보'
      : '수장이 알고 있는 정보',
    promiseLabel: headStatus.isDeceasedHead
      ? '직전 수장이 사망 전에 알고 있던 약속'
      : '수장이 알고 있는 약속',
    unresolved: card.unresolved,
  };
}

export function getCrisisView(idx, realmId) {
  const transition = getSuccessionOverlay(idx, realmId);
  if (!transition) return null;
  const former = idx.personById[transition.death.person_id];
  const candidates = (transition.candidates ?? []).map((candidate) => {
    const person = idx.personById[candidate.person_id];
    const copy = SUCCESSION_PRIORITY_COPY[candidate.priority] ?? {
      role: 'competing',
      title: '계승 후보',
      standingLabel: '계승 권리',
      reason: '',
    };
    const derived = idx.derivedById?.[candidate.claim_record_id];
    const sourceClaim = derived ? idx.claimById[derived.source_claim_id] : null;
    const sourcePerson = sourceClaim
      ? idx.personById[sourceClaim.claimant_person_id]
      : derived
        ? idx.personById[derived.via_parent_person_id]
        : null;
    return {
      personId: candidate.person_id,
      personName: person?.name ?? candidate.person_id,
      houseId: candidate.house_id,
      claimRecordId: candidate.claim_record_id,
      origin: candidate.claim_origin,
      priority: candidate.priority,
      generationDistance: candidate.generation_distance,
      role: copy.role,
      title: copy.title,
      standingLabel: copy.standingLabel,
      reason: copy.reason,
      isPriority: candidate.priority === 'direct_strong_original',
      sourceClaimId: derived?.source_claim_id ?? null,
      sourcePersonId: sourcePerson?.id ?? derived?.via_parent_person_id ?? null,
      sourcePersonName: sourcePerson?.name ?? derived?.via_parent_person_id ?? null,
    };
  });
  const priority = candidates.find((item) => item.isPriority) ?? null;
  const competing = candidates.filter((item) => !item.isPriority);
  return {
    realmId: transition.realm_id,
    formerIncumbentPersonId: transition.death.person_id,
    formerIncumbentName: former?.name ?? transition.death.person_id,
    vacant: transition.vacancy?.is_vacant === true,
    presumptiveSuccessorPersonId: transition.presumptive_successor_person_id,
    presumptiveSuccessorHouseId: transition.presumptive_successor_house_id,
    candidates,
    priority,
    competing,
  };
}

export function getRealmView(idx, realmId) {
  const realm = idx.realmById[realmId];
  if (!realm) return null;
  const capital = idx.territoryById[realm.capital_territory_id];
  const skeletonRuler = idx.rulerById[realm.ruler_id];
  const rights = idx.rightsByRealm[realmId];
  const incumbentId = rights?.incumbent_person_id;
  const incumbent = idx.personById[incumbentId];
  const identity = idx.realmIdentityById[realmId];
  const claims = (idx.claimsByRealm[realmId] ?? []).map((claim) => claimProjection(claim, idx));
  const houses = housesForRealm(idx, realmId);
  const crisis = getCrisisView(idx, realmId);
  const vacant = Boolean(crisis?.vacant);
  return {
    id: realm.id,
    name: realm.name,
    shortLabel: realmShortLabel(realm.id),
    capitalTerritoryId: realm.capital_territory_id,
    capitalLabel: capital
      ? `${capital.id} (${capital.x}, ${capital.y})`
      : realm.capital_territory_id,
    incumbentPersonId: vacant ? null : incumbentId,
    incumbentName: vacant ? '공석' : incumbent?.name ?? skeletonRuler?.name ?? incumbentId,
    formerIncumbentPersonId: crisis?.formerIncumbentPersonId ?? null,
    formerIncumbentName: crisis?.formerIncumbentName ?? null,
    vacant,
    crisis,
    majorityCultureName: nameOf(idx.cultureById, identity?.majority_culture_id),
    majorityReligionName: nameOf(idx.religionById, identity?.majority_religion_id),
    territoryCount: realm.territory_ids.length,
    houses,
    claims,
  };
}

export function getPersonView(idx, personId) {
  const person = idx.personById[personId];
  if (!person) return null;
  const house = idx.houseById[person.house_id];
  const realm = idx.realmById[person.realm_id];
  const identity = idx.personIdentityById[personId];
  const home = idx.territoryById[person.home_territory_id];
  const actor = idx.activeByPerson[personId];
  const parentNames = (person.known_parent_ids ?? []).map(
    (id) => idx.personById[id]?.name ?? id,
  );
  const claims = getClaimsForPerson(idx, personId);
  return {
    id: person.id,
    name: person.name,
    realmId: person.realm_id,
    realmName: realm?.name ?? person.realm_id,
    houseId: person.house_id,
    houseName: house?.name ?? person.house_id,
    generation: person.generation,
    generationLabel: generationLabel(person.generation),
    homeTerritoryId: person.home_territory_id,
    homeLabel: home ? `${home.id} (${home.x}, ${home.y})` : person.home_territory_id,
    cultureName: nameOf(idx.cultureById, identity?.culture_id),
    religionName: nameOf(idx.religionById, identity?.religion_id),
    activityLabel: actor ? '적극적 정치 행위자' : '보조 인물',
    isActive: Boolean(actor),
    roleLabel: actor ? ROLE_LABEL[actor.primary_role] ?? null : null,
    badges: personBadges(idx, personId),
    parentNames,
    parentLabel:
      parentNames.length > 0
        ? `알려진 부모: ${parentNames.join(', ')}`
        : '알려진 부모 기록 없음',
    claims,
    claimSummary:
      claims.length === 0 ? '현재 기록된 계승 권리 없음' : null,
    promises: getVisiblePromises(idx, personId),
    information: getVisibleInformation(idx, personId),
  };
}

export function selectionAfterTerritory(idx, territoryId, current) {
  const territory = idx.territoryById[territoryId];
  if (!territory) return current;
  if (territory.realm_id === current.selectedRealmId) {
    return { ...current, selectedTerritoryId: territoryId };
  }
  const houses = housesForRealm(idx, territory.realm_id);
  const ruling = houses.find((house) => house.ruling) ?? houses[0];
  const rights = idx.rightsByRealm[territory.realm_id];
  return {
    selectedTerritoryId: territoryId,
    selectedRealmId: territory.realm_id,
    selectedHouseId: ruling?.id ?? null,
    selectedPersonId: rights?.incumbent_person_id ?? ruling?.headPersonId ?? null,
  };
}

export function selectionAfterHouse(idx, houseId, current) {
  const house = idx.houseById[houseId];
  if (!house) return current;
  const keepPerson =
    current.selectedPersonId && idx.personById[current.selectedPersonId]?.house_id === houseId;
  return {
    ...current,
    selectedHouseId: houseId,
    selectedRealmId: house.realm_id,
    selectedPersonId: keepPerson ? current.selectedPersonId : house.head_person_id,
  };
}

export function selectionAfterPerson(idx, personId, current) {
  const person = idx.personById[personId];
  if (!person) return current;
  return {
    ...current,
    selectedPersonId: personId,
    selectedHouseId: person.house_id,
    selectedRealmId: person.realm_id,
  };
}
