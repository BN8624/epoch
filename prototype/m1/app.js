// 계승 분쟁 이해 화면 — 시나리오 데이터를 렌더링하고 클릭·키보드 탐색을 처리
import {
  scenario,
  VISIBILITY_LABELS,
  getCandidateDetail,
  getCandidateSummary,
  getHouseDetail,
  getPlayerView,
  getCrisisView,
} from './scenario.js';

/** @type {{ candidateId: string|null, houseId: string|null, playerExpanded: boolean }} */
const state = {
  candidateId: scenario.candidates[0]?.id ?? null,
  houseId: scenario.houses[0]?.id ?? null,
  playerExpanded: false,
};

function el(id) {
  return document.getElementById(id);
}

function visibilityBadge(visibility) {
  const label = VISIBILITY_LABELS[visibility] ?? visibility;
  const icon =
    visibility === 'public_fact'
      ? '●'
      : visibility === 'unverified'
        ? '?'
        : '◆';
  const cls =
    visibility === 'public_fact'
      ? 'badge badge-public'
      : visibility === 'unverified'
        ? 'badge badge-unverified'
        : 'badge badge-private';
  return `<span class="${cls}" title="${escapeHtml(label)}"><span class="badge-icon" aria-hidden="true">${icon}</span><span class="badge-text">${escapeHtml(label)}</span></span>`;
}

function escapeHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function renderCrisis() {
  const crisis = getCrisisView();
  const root = el('crisis-panel');
  if (!root) return;
  root.innerHTML = `
    <header class="crisis-header">
      <p class="eyebrow">${escapeHtml(crisis.kingdomName)}</p>
      <h1>계승 분쟁</h1>
      <p class="crisis-lead">통치자의 권위가 무너지기 직전, 후계 구도를 파악하십시오.</p>
    </header>
    <dl class="crisis-facts">
      <div>
        <dt>현재 통치자</dt>
        <dd>${escapeHtml(crisis.rulerName)}</dd>
      </div>
      <div>
        <dt>건강 상태</dt>
        <dd>${escapeHtml(crisis.healthStatus)}</dd>
      </div>
      <div>
        <dt>권위 상태</dt>
        <dd>${escapeHtml(crisis.authorityStatus)}</dd>
      </div>
      <div>
        <dt>계승 선언</dt>
        <dd>${escapeHtml(crisis.successionDeclaration)}</dd>
      </div>
      <div class="crisis-risk">
        <dt>내전 위험</dt>
        <dd>${escapeHtml(crisis.civilWarRisk)}</dd>
      </div>
    </dl>
  `;
}

function renderCandidateList() {
  const root = el('candidate-list');
  if (!root) return;
  root.innerHTML = '';
  root.setAttribute('role', 'listbox');
  root.setAttribute('aria-label', '후계 후보');

  scenario.candidates.forEach((cand, index) => {
    const summary = getCandidateSummary(cand.id);
    if (!summary) return;
    const selected = state.candidateId === cand.id;
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = `candidate-card${selected ? ' is-selected' : ''}`;
    btn.setAttribute('role', 'option');
    btn.setAttribute('aria-selected', selected ? 'true' : 'false');
    btn.setAttribute(
      'aria-label',
      `${summary.label} ${summary.name}. ${summary.claimStrengthText}. 공개 지지 가문 ${summary.supporterCount}곳`,
    );
    btn.dataset.candidateId = cand.id;
    btn.tabIndex = selected ? 0 : -1;
    btn.innerHTML = `
      <div class="card-top">
        <span class="card-label">${escapeHtml(summary.label)}</span>
        ${selected ? '<span class="selected-mark" aria-hidden="true">선택됨</span>' : ''}
      </div>
      <h3 class="card-name">${escapeHtml(summary.name)}</h3>
      <p class="card-relation">${escapeHtml(summary.relationshipToRuler)}</p>
      <p class="card-claim"><span class="field-label">권리</span> ${escapeHtml(summary.claimStrengthText)}</p>
      <p class="card-strength"><span class="field-label">강점</span> ${escapeHtml(summary.keyStrength)}</p>
      <p class="card-risk"><span class="field-label">위험</span> ${escapeHtml(summary.keyRisk)}</p>
      <p class="card-support"><span class="field-label">공개 지지</span> ${summary.supporterCount}개 가문${
        summary.supporterNames.length
          ? ` · ${escapeHtml(summary.supporterNames.join(', '))}`
          : ''
      }</p>
    `;
    btn.addEventListener('click', () => selectCandidate(cand.id));
    btn.addEventListener('keydown', (e) => onCandidateKeydown(e, index));
    root.appendChild(btn);
  });
}

function onCandidateKeydown(e, index) {
  const count = scenario.candidates.length;
  let next = null;
  if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
    e.preventDefault();
    next = (index + 1) % count;
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    e.preventDefault();
    next = (index - 1 + count) % count;
  } else if (e.key === 'Home') {
    e.preventDefault();
    next = 0;
  } else if (e.key === 'End') {
    e.preventDefault();
    next = count - 1;
  } else if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    selectCandidate(scenario.candidates[index].id);
    return;
  }
  if (next !== null) {
    selectCandidate(scenario.candidates[next].id);
    const cards = el('candidate-list')?.querySelectorAll('.candidate-card');
    cards?.[next]?.focus();
  }
}

function selectCandidate(candidateId) {
  const detail = getCandidateDetail(candidateId);
  if (!detail) {
    // 존재하지 않는 ID — 중단하지 않음
    return;
  }
  state.candidateId = candidateId;
  renderCandidateList();
  renderCandidateDetail();
  // 해당 후보 지지 가문이 있으면 가문 하이라이트 동기화는 선택 사항 — 목록만 갱신
  renderHouseList();
}

function renderCandidateDetail() {
  const root = el('candidate-detail');
  if (!root) return;
  const detail = getCandidateDetail(state.candidateId);
  if (!detail) {
    root.innerHTML = '<p class="empty-hint">후보를 선택하면 상세 정보가 표시됩니다.</p>';
    root.removeAttribute('aria-labelledby');
    return;
  }

  const infoList = detail.information
    .map(
      (info) => `
      <li class="info-item info-${escapeHtml(info.visibility)}">
        ${visibilityBadge(info.visibility)}
        <p>${escapeHtml(info.text)}</p>
      </li>`,
    )
    .join('');

  const houses =
    detail.supportingHouses.length > 0
      ? detail.supportingHouses.map((h) => escapeHtml(h.name)).join(', ')
      : '없음';

  root.innerHTML = `
    <div class="detail-header">
      <p class="eyebrow">${escapeHtml(detail.label)}</p>
      <h2 id="candidate-detail-title">${escapeHtml(detail.name)}</h2>
      <p class="detail-relation">${escapeHtml(detail.relationshipToRuler)}</p>
    </div>
    <section class="detail-block">
      <h3>권리의 근거</h3>
      <p>${escapeHtml(detail.claimBasis)}</p>
      <p class="meta-line"><span class="field-label">권리 성격</span> ${escapeHtml(detail.claimTypeLabel)} · ${escapeHtml(detail.claimStrengthText)}</p>
    </section>
    <div class="detail-columns">
      <section class="detail-block">
        <h3>강점</h3>
        <ul>${detail.strengths.map((s) => `<li>${escapeHtml(s)}</li>`).join('')}</ul>
      </section>
      <section class="detail-block">
        <h3>약점</h3>
        <ul>${detail.weaknesses.map((s) => `<li>${escapeHtml(s)}</li>`).join('')}</ul>
      </section>
    </div>
    <section class="detail-block">
      <h3>공개 지지 가문</h3>
      <p>${houses}</p>
    </section>
    <section class="detail-block">
      <h3>반대·경계 이유</h3>
      <ul>${detail.oppositionReasons.map((s) => `<li>${escapeHtml(s)}</li>`).join('')}</ul>
    </section>
    <section class="detail-block">
      <h3>정보 상태</h3>
      <ul class="info-list">${infoList}</ul>
    </section>
  `;
  root.setAttribute('aria-labelledby', 'candidate-detail-title');
}

function renderHouseList() {
  const root = el('house-list');
  if (!root) return;
  root.innerHTML = '';
  root.setAttribute('role', 'listbox');
  root.setAttribute('aria-label', '유력 가문');

  scenario.houses.forEach((house, index) => {
    const detail = getHouseDetail(house.id);
    if (!detail) return;
    const selected = state.houseId === house.id;
    const supportsSelected =
      state.candidateId &&
      house.supportCandidateId === state.candidateId &&
      house.supportStatus === 'declared';

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = [
      'house-card',
      selected ? 'is-selected' : '',
      supportsSelected ? 'supports-selected-candidate' : '',
      house.supportStatus === 'undecided' ? 'is-undecided' : '',
    ]
      .filter(Boolean)
      .join(' ');
    btn.setAttribute('role', 'option');
    btn.setAttribute('aria-selected', selected ? 'true' : 'false');
    btn.setAttribute(
      'aria-label',
      `${detail.name}. 현재 입장: ${detail.supportStatusLabel}`,
    );
    btn.dataset.houseId = house.id;
    btn.tabIndex = selected ? 0 : -1;
    btn.innerHTML = `
      <div class="card-top">
        <h3 class="card-name">${escapeHtml(detail.name)}</h3>
        ${selected ? '<span class="selected-mark" aria-hidden="true">선택됨</span>' : ''}
      </div>
      <p class="house-stance">
        <span class="field-label">입장</span>
        ${escapeHtml(detail.supportStatusLabel)}
      </p>
    `;
    btn.addEventListener('click', () => selectHouse(house.id));
    btn.addEventListener('keydown', (e) => onHouseKeydown(e, index));
    root.appendChild(btn);
  });
}

function onHouseKeydown(e, index) {
  const count = scenario.houses.length;
  let next = null;
  if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
    e.preventDefault();
    next = (index + 1) % count;
  } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
    e.preventDefault();
    next = (index - 1 + count) % count;
  } else if (e.key === 'Home') {
    e.preventDefault();
    next = 0;
  } else if (e.key === 'End') {
    e.preventDefault();
    next = count - 1;
  } else if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    selectHouse(scenario.houses[index].id);
    return;
  }
  if (next !== null) {
    selectHouse(scenario.houses[next].id);
    const cards = el('house-list')?.querySelectorAll('.house-card');
    cards?.[next]?.focus();
  }
}

function selectHouse(houseId) {
  const detail = getHouseDetail(houseId);
  if (!detail) {
    return;
  }
  state.houseId = houseId;
  renderHouseList();
  renderHouseDetail();
}

function renderHouseDetail() {
  const root = el('house-detail');
  if (!root) return;
  const detail = getHouseDetail(state.houseId);
  if (!detail) {
    root.innerHTML = '<p class="empty-hint">가문을 선택하면 지지 이유가 표시됩니다.</p>';
    return;
  }

  root.innerHTML = `
    <div class="detail-header">
      <p class="eyebrow">가문 입장</p>
      <h2 id="house-detail-title">${escapeHtml(detail.name)}</h2>
      <p class="house-stance-large">
        <span class="field-label">현재 입장</span>
        ${escapeHtml(detail.supportStatusLabel)}
      </p>
    </div>
    <div class="detail-columns">
      <section class="detail-block reasons-positive">
        <h3>주요 긍정 이유</h3>
        <ul>${detail.positiveReasons.map((t) => `<li>${escapeHtml(t)}</li>`).join('')}</ul>
      </section>
      <section class="detail-block reasons-negative">
        <h3>주요 부정 이유</h3>
        <ul>${detail.negativeReasons.map((t) => `<li>${escapeHtml(t)}</li>`).join('')}</ul>
      </section>
    </div>
  `;
  root.setAttribute('aria-labelledby', 'house-detail-title');
}

function renderPlayer() {
  const root = el('player-panel');
  if (!root) return;
  const player = getPlayerView();
  const expanded = state.playerExpanded;

  const relList = player.relationships
    .map(
      (r) => `
      <li>
        <strong>${escapeHtml(r.candidateName)}</strong>
        <span>${escapeHtml(r.text)}</span>
      </li>`,
    )
    .join('');

  const pressureList = player.pressures
    .map(
      (p) => `
      <li class="info-item info-${escapeHtml(p.visibility)}">
        ${visibilityBadge(p.visibility)}
        <div>
          <p class="pressure-source">${escapeHtml(p.source)}</p>
          <p>${escapeHtml(p.text)}</p>
        </div>
      </li>`,
    )
    .join('');

  root.innerHTML = `
    <button type="button"
      class="player-toggle${expanded ? ' is-expanded' : ''}"
      id="player-toggle"
      aria-expanded="${expanded ? 'true' : 'false'}"
      aria-controls="player-details"
      aria-label="플레이어 ${escapeHtml(player.name)} 정보 ${expanded ? '접기' : '펼치기'}">
      <div class="player-summary">
        <p class="eyebrow">플레이어</p>
        <h2>${escapeHtml(player.name)}</h2>
        <p>${escapeHtml(player.status)}</p>
        <p class="meta-line"><span class="field-label">직위</span> ${escapeHtml(player.office)}</p>
        <p class="meta-line"><span class="field-label">영지</span> ${escapeHtml(player.holding)}</p>
        <p class="meta-line"><span class="field-label">권리</span> ${escapeHtml(player.claimText)}</p>
        <p class="meta-line house-stance-line">${escapeHtml(player.houseStanceText)}</p>
        <span class="expand-hint" aria-hidden="true">${expanded ? '▲ 접기' : '▼ 관계와 압력 보기'}</span>
      </div>
    </button>
    <div id="player-details" class="player-details${expanded ? ' is-open' : ''}" ${expanded ? '' : 'hidden'}>
      <section class="detail-block">
        <h3>후보와의 관계</h3>
        <ul class="relation-list">${relList}</ul>
      </section>
      <section class="detail-block">
        <h3>현재 받는 압력</h3>
        <ul class="info-list pressure-list">${pressureList}</ul>
      </section>
    </div>
  `;

  el('player-toggle')?.addEventListener('click', () => {
    state.playerExpanded = !state.playerExpanded;
    renderPlayer();
    el('player-toggle')?.focus();
  });
}

function renderLegend() {
  const root = el('info-legend');
  if (!root) return;
  root.innerHTML = `
    <h2 class="legend-title">정보 상태 안내</h2>
    <ul class="legend-list">
      <li>${visibilityBadge('public_fact')} — 여러 세력이 공유하는 확정 사실</li>
      <li>${visibilityBadge('unverified')} — 소문·보고이나 원본 또는 확인이 없음</li>
      <li>${visibilityBadge('private')} — 특정 인물·가문만 아는 비공개 정보</li>
    </ul>
  `;
}

export function initApp() {
  renderCrisis();
  renderLegend();
  renderCandidateList();
  renderCandidateDetail();
  renderHouseList();
  renderHouseDetail();
  renderPlayer();
}

// 존재하지 않는 ID 선택 시 안전 (테스트·방어용)
export function trySelectCandidate(id) {
  selectCandidate(id);
  return state.candidateId;
}

export function trySelectHouse(id) {
  selectHouse(id);
  return state.houseId;
}

export function getAppState() {
  return { ...state };
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
  } else {
    initApp();
  }
}
