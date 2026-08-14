// 읽기 전용 세계 관찰 화면 — 선택 상태만 바꾸고 세계 데이터는 바꾸지 않는다

import {
  buildIndexes,
  getHouseRelations,
  getHouseView,
  getInitialSelection,
  getMapTiles,
  getPersonView,
  getRealmView,
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
      <dt>통치자</dt><dd>${escapeHtml(realm.incumbentName)}</dd>
      <dt>다수 문화</dt><dd>${escapeHtml(realm.majorityCultureName)}</dd>
      <dt>다수 종교</dt><dd>${escapeHtml(realm.majorityReligionName)}</dd>
      <dt>영지 수</dt><dd>${realm.territoryCount}</dd>
    </dl>
    <h3>계승 권리</h3>
    <div class="claim-card" data-role="incumbent">
      <h3>현재 통치자</h3>
      <p>${escapeHtml(realm.incumbentName)}</p>
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
          `${house.name}, 수장 ${house.headName}${house.ruling ? ', 통치 가문' : ''}`,
        )}"
      >
        <div class="card-name">${escapeHtml(house.name)}</div>
        <div class="card-meta">수장 ${escapeHtml(house.headName)}</div>
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
                aria-label="${escapeHtml(`${person.name}, ${person.generationLabel}`)}"
              >
                <div class="card-name">${escapeHtml(person.name)}</div>
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

function applySelection(next) {
  observer.state = next;
  renderAll();
  return observer.state;
}

function renderAll() {
  const { idx, state } = observer;
  renderMap(idx, state);
  renderRealm(idx, state);
  renderHouses(idx, state);
  renderPersons(idx, state);
  renderPersonDetail(idx, state);
}

async function boot() {
  const page = document.querySelector('.page');
  try {
    const response = await fetch('./rights-world.json');
    if (!response.ok) {
      throw new Error(`rights-world.json 로드 실패 (${response.status})`);
    }
    const world = await response.json();
    observer.idx = buildIndexes(world);
    observer.state = getInitialSelection(observer.idx);
    renderSummary(getWorldSummary(observer.idx));
    renderAll();
  } catch (error) {
    page.innerHTML = `<p class="load-error">${escapeHtml(error.message)}</p>`;
    throw error;
  }
}

boot();
