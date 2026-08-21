// 읽기 전용 세계 관찰 화면 — 선택 상태만 바꾸고 세계 데이터는 바꾸지 않는다

import {
  buildIndexes,
  getHouseRelations,
  getHouseView,
  getInitialSelection,
  getMapTiles,
  getPersonView,
  getRealmView,
  getSuccessionCandidateDetail,
  getSuccessionDisputeView,
  getSuccessionHouseDetail,
  getWorldSummary,
  membersForHouse,
  selectionAfterHouse,
  selectionAfterPerson,
  selectionAfterTerritory,
} from './view-model.js';

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function displayName(value) {
  return value ? escapeHtml(value) : '기록 없음';
}

function houseHeadLine(idx, house) {
  const transition = idx.succession?.transition;
  if (transition && house.headPersonId === transition.death.person_id) {
    return `기존 수장 ${house.headName} · 사망 · 현재 수장 미결정`;
  }
  return `수장 ${house.headName}`;
}

function disputeRealmId(idx) {
  return idx.succession?.transition?.realm_id ?? null;
}

function badgesHtml(badges) {
  if (!badges?.length) return '';
  return `<div class="badge-row">${badges
    .map(
      (badge) =>
        `<span class="badge badge-${escapeHtml(badge.key)}">${escapeHtml(badge.label)}</span>`,
    )
    .join('')}</div>`;
}

function bindSelectable(root, selector, onChoose) {
  root.querySelectorAll(selector).forEach((el) => {
    el.addEventListener('click', () => onChoose(el));
    el.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onChoose(el);
      }
    });
  });
}

function bindRovingList(root, selector, items, selectedId, idOf, onSelect) {
  const cards = [...root.querySelectorAll(selector)];
  cards.forEach((el, index) => {
    el.addEventListener('click', () => onSelect(idOf(el)));
    el.addEventListener('keydown', (event) => {
      const count = items.length;
      let next = null;
      if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
        event.preventDefault();
        next = (index + 1) % count;
      } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
        event.preventDefault();
        next = (index - 1 + count) % count;
      } else if (event.key === 'Home') {
        event.preventDefault();
        next = 0;
      } else if (event.key === 'End') {
        event.preventDefault();
        next = count - 1;
      } else if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        onSelect(idOf(el));
        return;
      }
      if (next !== null) {
        onSelect(idOf(cards[next]) ?? items[next]);
      }
    });
  });
}

function renderSummary(summary) {
  document.getElementById('world-seed').textContent = `seed ${summary.seed}`;
  document.getElementById('world-stats').innerHTML = [
    `Realm ${summary.realmCount}`,
    `영지 ${summary.territoryCount}`,
    `가문 ${summary.houseCount}`,
    `인물 ${summary.personCount}`,
    `Active ${summary.activeCount}`,
    `계승 권리 ${summary.claimCount}`,
  ]
    .map((item) => `<li>${escapeHtml(item)}</li>`)
    .join('');
}

function renderMap(idx, state) {
  const tiles = getMapTiles(idx);
  const root = document.getElementById('world-map');
  root.innerHTML = tiles
    .map((tile) => {
      const selected = tile.id === state.selectedTerritoryId;
      const inRealm = tile.realmId === state.selectedRealmId;
      const realmIndex = String(tile.shortLabel).replace(/^R/, '') || '1';
      const classes = [
        'territory-tile',
        `realm-${realmIndex}`,
        inRealm ? 'in-selected-realm' : '',
        selected ? 'is-selected' : '',
      ]
        .filter(Boolean)
        .join(' ');
      return `<button
        type="button"
        class="${classes}"
        style="grid-column:${tile.x + 1};grid-row:${tile.y + 1}"
        data-territory-id="${escapeHtml(tile.id)}"
        data-realm-id="${escapeHtml(tile.realmId)}"
        aria-label="${escapeHtml(tile.accessibleName)}"
        aria-pressed="${selected ? 'true' : 'false'}"
      >
        <span class="tile-code">${escapeHtml(tile.shortLabel)}</span>
        <span class="tile-mark">${tile.isCapital ? '★' : ''}</span>
      </button>`;
    })
    .join('');

  bindSelectable(root, '.territory-tile', (el) => {
    applySelection(selectionAfterTerritory(idx, el.dataset.territoryId, observer.state));
  });
}

function renderRealm(idx, state) {
  const realm = getRealmView(idx, state.selectedRealmId);
  const root = document.getElementById('realm-detail');
  if (!realm) {
    root.innerHTML = '<p class="empty-note">선택한 Realm이 없습니다.</p>';
    return;
  }
  root.innerHTML = `
    <h3>${escapeHtml(realm.name)}</h3>
    <span class="aux-id">${escapeHtml(realm.shortLabel)}</span>
    <dl class="fact-list">
      <dt>수도</dt><dd>${escapeHtml(realm.capitalLabel)}</dd>
      ${
        realm.vacant
          ? `<dt>직전 통치자</dt><dd>${escapeHtml(realm.formerIncumbentName)} — 사망</dd>
      <dt>현재 통치자</dt><dd>공석</dd>`
          : `<dt>통치자</dt><dd>${escapeHtml(realm.incumbentName)}</dd>`
      }
      <dt>다수 문화</dt><dd>${escapeHtml(realm.majorityCultureName)}</dd>
      <dt>다수 종교</dt><dd>${escapeHtml(realm.majorityReligionName)}</dd>
      <dt>영지 수</dt><dd>${realm.territoryCount}</dd>
    </dl>
    <h3>계승 권리</h3>
    <div class="claim-card" data-role="${realm.vacant ? 'vacant' : 'incumbent'}">
      <h3>${realm.vacant ? '현재 통치자 공석' : '현재 통치자'}</h3>
      <p>${realm.vacant ? '공석' : escapeHtml(realm.incumbentName)}</p>
    </div>
    <ul class="claim-list">
      ${realm.claims
        .map(
          (claim) => `<li class="claim-card" data-claim-kind="${escapeHtml(claim.kind)}">
            <h3>${escapeHtml(claim.title)}</h3>
            <p>${escapeHtml(claim.personName)}</p>
            <p>${escapeHtml(claim.standingLabel)}</p>
            <p>근거: ${escapeHtml(claim.evidenceLabel)}</p>
          </li>`,
        )
        .join('')}
    </ul>
  `;
}

function renderHouses(idx, state) {
  const realm = getRealmView(idx, state.selectedRealmId);
  const list = document.getElementById('house-list');
  const relationsRoot = document.getElementById('house-relations');
  const houses = realm?.houses ?? [];
  list.innerHTML = houses
    .map((house) => {
      const selected = house.id === state.selectedHouseId;
      return `<button
        type="button"
        class="house-card${selected ? ' is-selected' : ''}"
        data-house-id="${escapeHtml(house.id)}"
        aria-pressed="${selected ? 'true' : 'false'}"
        aria-label="${escapeHtml(
          `${house.name}, ${houseHeadLine(idx, house)}${house.ruling ? ', 통치 가문' : ''}${selected ? ', 선택됨' : ''}`,
        )}"
      >
        <div class="card-head">
          <div class="card-name">${escapeHtml(house.name)}</div>
          ${selected ? '<span class="selection-mark">선택됨</span>' : ''}
        </div>
        <div class="card-meta">${escapeHtml(houseHeadLine(idx, house))}</div>
        <div class="card-meta">거점 ${escapeHtml(house.seatLabel)}</div>
        <div class="card-meta">${escapeHtml(house.cultureName)} · ${escapeHtml(house.religionName)}</div>
        <div class="card-meta">${escapeHtml(house.identityStance)}</div>
        ${
          house.ruling
            ? '<div class="badge-row"><span class="badge badge-ruling">통치 가문</span></div>'
            : ''
        }
      </button>`;
    })
    .join('');

  bindSelectable(list, '.house-card', (el) => {
    applySelection(selectionAfterHouse(idx, el.dataset.houseId, observer.state));
  });

  const relations = state.selectedHouseId ? getHouseRelations(idx, state.selectedHouseId) : [];
  const selectedHouse = state.selectedHouseId ? getHouseView(idx, state.selectedHouseId) : null;
  relationsRoot.innerHTML = selectedHouse
    ? `<h3>${escapeHtml(selectedHouse.name)}의 관계</h3>
       ${
         relations.length
           ? `<ul class="relation-list">${relations
               .map((rel) => `<li class="relation-card">${escapeHtml(rel.sentence)}</li>`)
               .join('')}</ul>`
           : '<p class="empty-note">기록된 가문 관계가 없습니다.</p>'
       }`
    : '';
}

function renderPersons(idx, state) {
  const members = state.selectedHouseId ? membersForHouse(idx, state.selectedHouseId) : null;
  const root = document.getElementById('person-list');
  if (!members) {
    root.innerHTML = '<p class="empty-note">가문을 선택하면 인물이 표시됩니다.</p>';
    return;
  }

  const groups = [
    ['노년 세대', members.elder],
    ['현재 세대', members.current],
    ['후속 세대', members.young],
  ];
  root.innerHTML = groups
    .map(([title, people]) => {
      return `<div class="person-group">
        <h3>${escapeHtml(title)} ${people.length}</h3>
        <div class="person-group-list">
          ${people
            .map((person) => {
              const selected = person.id === state.selectedPersonId;
              return `<button
                type="button"
                class="person-card${selected ? ' is-selected' : ''}"
                data-person-id="${escapeHtml(person.id)}"
                aria-pressed="${selected ? 'true' : 'false'}"
                aria-label="${escapeHtml(
                  `${person.name}, ${person.generationLabel}${selected ? ', 선택됨' : ''}`,
                )}"
              >
                <div class="card-head">
                  <div class="card-name">${escapeHtml(person.name)}</div>
                  ${selected ? '<span class="selection-mark">선택됨</span>' : ''}
                </div>
                <div class="card-meta">${escapeHtml(person.generationLabel)}</div>
                <div class="card-meta">${escapeHtml(person.activityLabel)}</div>
                ${badgesHtml(person.badges)}
              </button>`;
            })
            .join('')}
        </div>
      </div>`;
    })
    .join('');

  bindSelectable(root, '.person-card', (el) => {
    applySelection(selectionAfterPerson(idx, el.dataset.personId, observer.state));
  });
}

function renderPersonDetail(idx, state) {
  const person = state.selectedPersonId ? getPersonView(idx, state.selectedPersonId) : null;
  const root = document.getElementById('person-detail');
  if (!person) {
    root.innerHTML = '<p class="empty-note">인물을 선택하면 상세가 표시됩니다.</p>';
    return;
  }

  const claimHtml =
    person.claims.length === 0
      ? `<p class="empty-note" data-claim-empty="true">${escapeHtml(person.claimSummary)}</p>`
      : person.claims
          .map(
            (claim) => `<div class="claim-card" data-claim-kind="${escapeHtml(claim.kind)}">
              <h3>계승 권리</h3>
              <p>${escapeHtml(claim.standingLabel)}</p>
              <p>근거: ${escapeHtml(claim.evidenceLabel)}</p>
            </div>`,
          )
          .join('');

  const promiseHtml = person.promises.length
    ? `<ul class="promise-list">${person.promises
        .map((item) => `<li class="promise-card">${escapeHtml(item.sentence)}</li>`)
        .join('')}</ul>`
    : '<p class="empty-note">이 인물이 알고 있는 약속이 없습니다.</p>';

  const infoHtml = person.information.length
    ? `<ul class="info-list">${person.information
        .map(
          (item) => `<li class="info-card" data-info-scope="${escapeHtml(item.scope)}" data-info-topic="${escapeHtml(item.topic)}" data-info-confidence="${escapeHtml(item.confidence)}">
            <h3>${escapeHtml(item.badge)}</h3>
            <p>${escapeHtml(item.body)}</p>
          </li>`,
        )
        .join('')}</ul>`
    : '<p class="empty-note">이 인물에게 보이는 정보가 없습니다.</p>';

  root.innerHTML = `
    <h3>${escapeHtml(person.name)}</h3>
    <dl class="fact-list">
      <dt>Realm</dt><dd>${escapeHtml(person.realmName)}</dd>
      <dt>가문</dt><dd>${escapeHtml(person.houseName)}</dd>
      <dt>세대</dt><dd>${escapeHtml(person.generationLabel)}</dd>
      <dt>거주지</dt><dd>${escapeHtml(person.homeLabel)}</dd>
      <dt>문화</dt><dd>${escapeHtml(person.cultureName)}</dd>
      <dt>종교</dt><dd>${escapeHtml(person.religionName)}</dd>
      <dt>활동</dt><dd>${escapeHtml(person.activityLabel)}</dd>
      ${person.roleLabel ? `<dt>역할</dt><dd>${escapeHtml(person.roleLabel)}</dd>` : ''}
      <dt>부모</dt><dd>${escapeHtml(person.parentLabel)}</dd>
    </dl>
    ${badgesHtml(person.badges)}
    <h3>계승 권리</h3>
    ${claimHtml}
    <h3>이 인물이 아는 약속</h3>
    ${promiseHtml}
    <h3>이 인물이 아는 정보</h3>
    ${infoHtml}
  `;
}

const observer = {
  idx: null,
  state: null,
};

function attrSelector(name, value) {
  return `[${name}="${String(value).replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"]`;
}

function focusRestoreSelector(el) {
  if (!(el instanceof HTMLElement)) return null;
  if (el.dataset.territoryId) {
    return `.territory-tile${attrSelector('data-territory-id', el.dataset.territoryId)}`;
  }
  if (el.dataset.houseId) {
    return `.house-card${attrSelector('data-house-id', el.dataset.houseId)}`;
  }
  if (el.classList.contains('dispute-candidate-card') && el.dataset.personId) {
    return `.dispute-candidate-card${attrSelector('data-person-id', el.dataset.personId)}`;
  }
  if (el.classList.contains('dispute-house-card') && el.dataset.houseId) {
    return `.dispute-house-card${attrSelector('data-house-id', el.dataset.houseId)}`;
  }
  if (el.dataset.personId && el.classList.contains('crisis-candidate')) {
    return `.crisis-candidate${attrSelector('data-person-id', el.dataset.personId)}`;
  }
  if (el.dataset.personId) {
    return `.person-card${attrSelector('data-person-id', el.dataset.personId)}`;
  }
  return null;
}

function applySelection(next) {
  const restore = focusRestoreSelector(document.activeElement);
  observer.state = next;
  renderAll();
  if (restore) {
    document.querySelector(restore)?.focus({ preventScroll: true });
  }
  return observer.state;
}

function hideLegacyCrisisPanel() {
  const panel = document.getElementById('crisis-panel');
  const root = document.getElementById('succession-crisis');
  if (panel) panel.hidden = true;
  if (root) root.innerHTML = '';
}

function selectDisputeCandidate(personId) {
  const { idx, state } = observer;
  const realmId = disputeRealmId(idx);
  const detail = getSuccessionCandidateDetail(idx, realmId, personId);
  if (!detail) return;
  const next = selectionAfterPerson(idx, personId, state);
  applySelection({
    ...next,
    selectedDisputeCandidatePersonId: personId,
    selectedDisputeHouseId: state.selectedDisputeHouseId,
  });
  document
    .querySelector(`.dispute-candidate-card${attrSelector('data-person-id', personId)}`)
    ?.focus({ preventScroll: true });
}

function selectDisputeHouse(houseId) {
  const { idx, state } = observer;
  const realmId = disputeRealmId(idx);
  const detail = getSuccessionHouseDetail(idx, realmId, houseId);
  if (!detail) return;
  const next = selectionAfterHouse(idx, houseId, state);
  applySelection({
    ...next,
    selectedDisputeHouseId: houseId,
    selectedDisputeCandidatePersonId: state.selectedDisputeCandidatePersonId,
  });
  document
    .querySelector(`.dispute-house-card${attrSelector('data-house-id', houseId)}`)
    ?.focus({ preventScroll: true });
}

function renderDisputeCrisis(dispute) {
  const root = document.getElementById('dispute-crisis');
  if (!root) return;
  root.innerHTML = `
    <header class="dispute-crisis-header">
      <p class="eyebrow">${displayName(dispute.realmName)}</p>
      <h1 id="dispute-crisis-heading">계승 분쟁</h1>
    </header>
    <dl class="dispute-facts">
      <div>
        <dt>직전 통치자</dt>
        <dd data-role="former-incumbent">${displayName(dispute.formerIncumbentName)} · 사망</dd>
      </div>
      <div>
        <dt>현재 통치자</dt>
        <dd data-role="vacancy">공석</dd>
      </div>
      <div>
        <dt>법적 상태</dt>
        <dd>${escapeHtml(dispute.legalStatus)}</dd>
      </div>
    </dl>
  `;
}

function renderDisputeCandidates(idx, state, dispute) {
  const list = document.getElementById('dispute-candidate-list');
  const detailRoot = document.getElementById('dispute-candidate-detail');
  if (!list || !detailRoot) return;
  const selectedId = state.selectedDisputeCandidatePersonId;
  list.innerHTML = dispute.candidates
    .map((candidate) => {
      const selected = candidate.personId === selectedId;
      const classes = [
        'dispute-candidate-card',
        selected ? 'is-selected' : '',
        candidate.isPriority ? 'is-priority' : '',
      ]
        .filter(Boolean)
        .join(' ');
      return `<button
        type="button"
        class="${classes}"
        role="option"
        data-person-id="${escapeHtml(candidate.personId)}"
        data-candidate-slot="${escapeHtml(candidate.slot ?? '')}"
        data-candidate-priority="${escapeHtml(candidate.priority)}"
        data-candidate-origin="${escapeHtml(candidate.origin)}"
        data-claim-record="${escapeHtml(candidate.claimRecordId)}"
        ${candidate.provenance?.sourcePersonId ? `data-derived-source="${escapeHtml(candidate.provenance.sourcePersonId)}"` : ''}
        aria-selected="${selected ? 'true' : 'false'}"
        tabindex="${selected ? '0' : '-1'}"
        aria-label="${escapeHtml(
          `${candidate.slotLabel ?? ''} ${candidate.personName ?? ''} ${candidate.standingLabel ?? ''}${selected ? ', 선택됨' : ''}`,
        )}"
      >
        <span class="dispute-card-kicker">
          <span class="dispute-slot">${escapeHtml(candidate.slotLabel ?? '')}</span>
          ${selected ? '<span class="selection-mark">선택됨</span>' : ''}
        </span>
        <div class="card-name">${displayName(candidate.personName)}</div>
        <div class="card-meta">${displayName(candidate.houseName)}</div>
        <div class="card-meta">${escapeHtml(candidate.standingLabel ?? '')}</div>
        <div class="card-meta">${escapeHtml(candidate.generationLabel ?? '')} · ${escapeHtml(candidate.activityLabel ?? '')}</div>
        ${candidate.badge ? `<div class="badge-row"><span class="badge">${escapeHtml(candidate.badge)}</span></div>` : ''}
        ${
          candidate.isPriority && candidate.isKnownChildOfFormer
            ? '<p class="card-meta">직전 통치자의 알려진 자녀</p>'
            : ''
        }
        ${
          candidate.priority === 'restored_contested_original' && candidate.isRestoredLineHead
            ? '<p class="card-meta">복권 계통의 현 가문 수장</p>'
            : ''
        }
        ${
          candidate.evidenceLabel && candidate.priority === 'restored_contested_original'
            ? `<p class="card-meta">${escapeHtml(candidate.evidenceLabel)}</p>`
            : ''
        }
        ${
          candidate.provenance?.sentence
            ? `<p class="card-meta" data-derived-lineage="true">${escapeHtml(candidate.provenance.sentence)}</p>`
            : ''
        }
      </button>`;
    })
    .join('');

  bindRovingList(
    list,
    '.dispute-candidate-card',
    dispute.candidates.map((item) => item.personId),
    selectedId,
    (el) => el.dataset.personId,
    selectDisputeCandidate,
  );

  const detail = selectedId
    ? getSuccessionCandidateDetail(idx, dispute.realmId, selectedId)
    : null;
  if (!detail) {
    detailRoot.innerHTML = '<p class="empty-note">후보를 선택하면 상세가 표시됩니다.</p>';
    return;
  }
  const infoHtml = detail.information.length
    ? `<ul class="info-list">${detail.information
        .map(
          (item) => `<li class="info-card" data-info-scope="${escapeHtml(item.scope)}" data-info-topic="${escapeHtml(item.topic)}" data-info-confidence="${escapeHtml(item.confidence)}">
            <h3>${escapeHtml(item.badge)}</h3>
            <p>${escapeHtml(item.body)}</p>
          </li>`,
        )
        .join('')}</ul>`
    : '<p class="empty-note">이 인물에게 보이는 정보가 없습니다.</p>';
  const promiseHtml = detail.promises.length
    ? `<ul class="promise-list">${detail.promises
        .map((item) => `<li class="promise-card">${escapeHtml(item.sentence)}</li>`)
        .join('')}</ul>`
    : '<p class="empty-note">이 인물이 알고 있는 약속이 없습니다.</p>';
  detailRoot.innerHTML = `
    <h3 id="dispute-candidate-detail-title">${displayName(detail.name)}</h3>
    <h3>신분</h3>
    <dl class="fact-list">
      <dt>이름</dt><dd>${displayName(detail.name)}</dd>
      <dt>Realm</dt><dd>${displayName(detail.realmName)}</dd>
      <dt>가문</dt><dd>${displayName(detail.houseName)}</dd>
      <dt>세대</dt><dd>${escapeHtml(detail.generationLabel ?? '')}</dd>
      <dt>문화</dt><dd>${escapeHtml(detail.cultureName ?? '')}</dd>
      <dt>종교</dt><dd>${escapeHtml(detail.religionName ?? '')}</dd>
      <dt>활동</dt><dd>${escapeHtml(detail.activityLabel ?? '')}</dd>
      ${detail.roleLabel ? `<dt>역할</dt><dd>${escapeHtml(detail.roleLabel)}</dd>` : ''}
    </dl>
    <h3>권리</h3>
    <dl class="fact-list">
      <dt>권리 유형</dt><dd>${escapeHtml(detail.rights.standingLabel ?? '')}</dd>
      <dt>출처</dt><dd>${escapeHtml(detail.rights.origin ?? '')}</dd>
      <dt>기록</dt><dd>${escapeHtml(detail.rights.claimRecordId ?? '')}</dd>
      <dt>법적 우선</dt><dd>${escapeHtml(detail.rights.priorityLabel ?? '')}</dd>
      ${detail.rights.evidenceLabel ? `<dt>근거</dt><dd>${escapeHtml(detail.rights.evidenceLabel)}</dd>` : ''}
      <dt>세대 거리</dt><dd>${escapeHtml(String(detail.rights.generationDistance ?? ''))}</dd>
    </dl>
    <h3>혈통</h3>
    <p>${escapeHtml(detail.lineage.label ?? '')}</p>
    <h3>정치적 맥락</h3>
    ${
      detail.politicalContext.length
        ? `<ul class="relation-list">${detail.politicalContext
            .map((line) => `<li class="relation-card">${escapeHtml(line)}</li>`)
            .join('')}</ul>`
        : '<p class="empty-note">표시할 정치적 맥락이 없습니다.</p>'
    }
    <h3>약속</h3>
    ${promiseHtml}
    <h3>정보</h3>
    ${infoHtml}
  `;
}

function renderDisputeHouses(idx, state, dispute) {
  const list = document.getElementById('dispute-house-list');
  const detailRoot = document.getElementById('dispute-house-detail');
  if (!list || !detailRoot) return;
  const selectedId = state.selectedDisputeHouseId;
  list.innerHTML = dispute.houses
    .map((house) => {
      const selected = house.id === selectedId;
      const headLines = house.headStatus?.cardHeadLines ?? [];
      return `<button
        type="button"
        class="dispute-house-card${selected ? ' is-selected' : ''}"
        role="option"
        data-house-id="${escapeHtml(house.id)}"
        data-deceased-head="${house.headStatus?.isDeceasedHead ? 'true' : 'false'}"
        aria-selected="${selected ? 'true' : 'false'}"
        tabindex="${selected ? '0' : '-1'}"
        aria-label="${escapeHtml(`${house.name ?? ''} ${headLines.join(' ')}${selected ? ', 선택됨' : ''}`)}"
      >
        <span class="dispute-card-kicker">
          <span class="card-name">${displayName(house.name)}</span>
          ${selected ? '<span class="selection-mark">선택됨</span>' : ''}
        </span>
        ${headLines.map((line) => `<div class="card-meta">${escapeHtml(line)}</div>`).join('')}
        <div class="card-meta">${escapeHtml(house.cultureName ?? '')} · ${escapeHtml(house.religionName ?? '')}</div>
        <div class="card-meta">${escapeHtml(house.identityStance ?? '')}</div>
        ${house.relationSummary
          .map((line) => `<div class="card-meta">${escapeHtml(line)}</div>`)
          .join('')}
      </button>`;
    })
    .join('');

  bindRovingList(
    list,
    '.dispute-house-card',
    dispute.houses.map((item) => item.id),
    selectedId,
    (el) => el.dataset.houseId,
    selectDisputeHouse,
  );

  const detail = selectedId
    ? getSuccessionHouseDetail(idx, dispute.realmId, selectedId)
    : null;
  if (!detail) {
    detailRoot.innerHTML = '<p class="empty-note">가문을 선택하면 상세가 표시됩니다.</p>';
    return;
  }
  const infoHtml = detail.information.length
    ? `<ul class="info-list">${detail.information
        .map(
          (item) => `<li class="info-card" data-info-scope="${escapeHtml(item.scope)}" data-info-topic="${escapeHtml(item.topic)}" data-info-confidence="${escapeHtml(item.confidence)}">
            <h3>${escapeHtml(item.badge)}</h3>
            <p>${escapeHtml(item.body)}</p>
          </li>`,
        )
        .join('')}</ul>`
    : '<p class="empty-note">표시할 정보가 없습니다.</p>';
  const promiseHtml = detail.promises.length
    ? `<ul class="promise-list">${detail.promises
        .map((item) => `<li class="promise-card">${escapeHtml(item.sentence)}</li>`)
        .join('')}</ul>`
    : '<p class="empty-note">표시할 약속이 없습니다.</p>';
  const headLines = detail.headStatus?.detailHeadLines ?? [];
  detailRoot.innerHTML = `
    <h3 id="dispute-house-detail-title">${displayName(detail.name)}</h3>
    <dl class="fact-list">
      <dt>가문</dt><dd>${displayName(detail.name)}</dd>
      <dt>Realm</dt><dd>${displayName(detail.realmName)}</dd>
      <dt>수장</dt><dd>${escapeHtml(headLines.join(' / '))}</dd>
      <dt>문화</dt><dd>${escapeHtml(detail.cultureName ?? '')}</dd>
      <dt>종교</dt><dd>${escapeHtml(detail.religionName ?? '')}</dd>
      <dt>다수 정체성</dt><dd>${escapeHtml(detail.identityStance ?? '')}</dd>
    </dl>
    <h3>가문 관계</h3>
    ${
      detail.relations.length
        ? `<ul class="relation-list">${detail.relations
            .map((rel) => `<li class="relation-card">${escapeHtml(rel.sentence)}</li>`)
            .join('')}</ul>`
        : '<p class="empty-note">기록된 가문 관계가 없습니다.</p>'
    }
    <h3>${escapeHtml(detail.promiseLabel)}</h3>
    ${promiseHtml}
    <h3>${escapeHtml(detail.informationLabel)}</h3>
    ${infoHtml}
  `;
}

function renderSuccessionWorkspace(idx, state) {
  const workspace = document.getElementById('succession-workspace');
  const banner = document.getElementById('world-context-banner');
  const pageTitle = document.getElementById('page-title');
  hideLegacyCrisisPanel();
  const realmId = disputeRealmId(idx);
  const dispute = realmId ? getSuccessionDisputeView(idx, realmId) : null;
  if (!workspace) return;
  if (!dispute) {
    workspace.hidden = true;
    if (banner) banner.hidden = true;
    if (pageTitle) pageTitle.textContent = '세계 관찰';
    document.getElementById('dispute-crisis').innerHTML = '';
    document.getElementById('dispute-candidate-list').innerHTML = '';
    document.getElementById('dispute-candidate-detail').innerHTML = '';
    document.getElementById('dispute-house-list').innerHTML = '';
    document.getElementById('dispute-house-detail').innerHTML = '';
    return;
  }
  workspace.hidden = false;
  if (banner) banner.hidden = false;
  if (pageTitle) pageTitle.textContent = '세계 맥락';
  renderDisputeCrisis(dispute);
  renderDisputeCandidates(idx, state, dispute);
  renderDisputeHouses(idx, state, dispute);
}

function renderAll() {
  const { idx, state } = observer;
  renderSuccessionWorkspace(idx, state);
  renderMap(idx, state);
  renderRealm(idx, state);
  renderHouses(idx, state);
  renderPersons(idx, state);
  renderPersonDetail(idx, state);
}

async function loadOptionalJson(path) {
  const response = await fetch(path);
  if (response.status === 404) return null;
  if (!response.ok) {
    throw new Error(`${path} 로드 실패 (${response.status})`);
  }
  return response.json();
}

async function boot() {
  const page = document.querySelector('.page');
  try {
    const response = await fetch('./rights-world.json');
    if (!response.ok) {
      throw new Error(`rights-world.json 로드 실패 (${response.status})`);
    }
    const world = await response.json();
    const succession = await loadOptionalJson('./succession-world.json');
    observer.idx = buildIndexes(world, succession);
    const initial = getInitialSelection(observer.idx);
    const realmId = disputeRealmId(observer.idx);
    const dispute = realmId ? getSuccessionDisputeView(observer.idx, realmId) : null;
    const firstCandidate = dispute?.candidateA ?? dispute?.candidates[0] ?? null;
    observer.state = {
      ...initial,
      selectedDisputeCandidatePersonId: firstCandidate?.personId ?? null,
      selectedDisputeHouseId: firstCandidate?.houseId ?? dispute?.houses[0]?.id ?? null,
    };
    renderSummary(getWorldSummary(observer.idx));
    renderAll();
  } catch (error) {
    page.innerHTML = `<p class="load-error">${escapeHtml(error.message)}</p>`;
    throw error;
  }
}

boot();
