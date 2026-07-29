// 계승 분쟁 이해·선택 화면 — 시나리오 렌더링과 제안·행동·결과 흐름
import {
  scenario,
  VISIBILITY_LABELS,
  getCandidateDetail,
  getCandidateSummary,
  getHouseDetail,
  getPlayerView,
  getCrisisView,
} from './scenario.js';
import {
  PROPOSALS,
  ACTIONS,
  createSession,
  resetSession,
  selectAction,
  confirmAction,
  cancelDecision,
  getWorldSupportingHouses,
  getWorldHouseStanceLabel,
} from './interactions.js';

/** @type {{ candidateId: string|null, houseId: string|null, playerExpanded: boolean, session: ReturnType<typeof createSession> }} */
const state = {
  candidateId: scenario.candidates[0]?.id ?? null,
  houseId: scenario.houses[0]?.id ?? null,
  playerExpanded: false,
  session: createSession(),
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

/** 월드 스냅샷을 반영한 후보 요약 */
function candidateSummaryForView(candidateId) {
  const base = getCandidateSummary(candidateId);
  if (!base) return null;
  const world = state.session.world;
  const worldCand = world.candidates.find((c) => c.id === candidateId);
  const supporters = getWorldSupportingHouses(world, candidateId);
  return {
    ...base,
    claimStrengthText: worldCand?.claimStrengthText ?? base.claimStrengthText,
    supporterCount: supporters.length,
    supporterNames: supporters.map((h) => h.name),
  };
}

/** 월드 스냅샷을 반영한 후보 상세 */
function candidateDetailForView(candidateId) {
  const base = getCandidateDetail(candidateId);
  if (!base) return null;
  const world = state.session.world;
  const worldCand = world.candidates.find((c) => c.id === candidateId);
  const supporters = getWorldSupportingHouses(world, candidateId);
  return {
    ...base,
    claimStrengthText: worldCand?.claimStrengthText ?? base.claimStrengthText,
    claimBasis: worldCand?.claimBasis ?? base.claimBasis,
    supportingHouses: supporters.map((h) => ({ id: h.id, name: h.name })),
  };
}

/** 월드 스냅샷을 반영한 가문 상세 */
function houseDetailForView(houseId) {
  const base = getHouseDetail(houseId);
  if (!base) return null;
  const world = state.session.world;
  const wh = world.houses.find((h) => h.id === houseId);
  if (!wh) return base;
  const label = getWorldHouseStanceLabel(world, houseId);
  return {
    ...base,
    supportStatus: wh.supportStatus,
    supportCandidateId: wh.supportCandidateId,
    supportStatusLabel: label ?? base.supportStatusLabel,
    supportCandidateName:
      wh.supportCandidateId
        ? (world.candidates.find((c) => c.id === wh.supportCandidateId)?.name ?? null)
        : null,
  };
}

function renderCrisis() {
  const crisis = getCrisisView();
  const root = el('crisis-panel');
  if (!root) return;
  root.innerHTML = `
    <header class="crisis-header">
      <p class="eyebrow">${escapeHtml(crisis.kingdomName)}</p>
      <h1>계승 분쟁</h1>
      <p class="crisis-lead">통치자의 권위가 무너지기 직전, 후계 구도를 파악하고 한 행동을 선택하십시오.</p>
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
    const summary = candidateSummaryForView(cand.id);
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
      <span class="card-top">
        <span class="card-label">${escapeHtml(summary.label)}</span>
        ${selected ? '<span class="selected-mark" aria-hidden="true">선택됨</span>' : ''}
      </span>
      <span class="card-name">${escapeHtml(summary.name)}</span>
      <span class="card-relation">${escapeHtml(summary.relationshipToRuler)}</span>
      <span class="card-claim"><span class="field-label">권리</span> ${escapeHtml(summary.claimStrengthText)}</span>
      <span class="card-strength"><span class="field-label">강점</span> ${escapeHtml(summary.keyStrength)}</span>
      <span class="card-risk"><span class="field-label">위험</span> ${escapeHtml(summary.keyRisk)}</span>
      <span class="card-support"><span class="field-label">공개 지지</span> ${summary.supporterCount}개 가문${
        summary.supporterNames.length
          ? ` · ${escapeHtml(summary.supporterNames.join(', '))}`
          : ''
      }</span>
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
  const detail = candidateDetailForView(candidateId);
  if (!detail) return;
  state.candidateId = candidateId;
  renderCandidateList();
  renderCandidateDetail();
  renderHouseList();
  // 포인터 선택 뒤 포커스 유지
  const card = el('candidate-list')?.querySelector(`[data-candidate-id="${candidateId}"]`);
  card?.focus();
}

function renderCandidateDetail() {
  const root = el('candidate-detail');
  if (!root) return;
  const detail = candidateDetailForView(state.candidateId);
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
    const detail = houseDetailForView(house.id);
    if (!detail) return;
    const selected = state.houseId === house.id;
    const wh = state.session.world.houses.find((h) => h.id === house.id);
    const supportsSelected =
      state.candidateId &&
      wh?.supportCandidateId === state.candidateId &&
      wh?.supportStatus === 'declared';

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = [
      'house-card',
      selected ? 'is-selected' : '',
      supportsSelected ? 'supports-selected-candidate' : '',
      detail.supportStatus === 'undecided' || detail.supportStatus === 'wavering'
        ? 'is-undecided'
        : '',
      detail.supportStatus === 'leaning' ? 'is-leaning' : '',
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
      <span class="card-top">
        <span class="card-name">${escapeHtml(detail.name)}</span>
        ${selected ? '<span class="selected-mark" aria-hidden="true">선택됨</span>' : ''}
      </span>
      <span class="house-stance">
        <span class="field-label">입장</span>
        ${escapeHtml(detail.supportStatusLabel)}
      </span>
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
  const detail = houseDetailForView(houseId);
  if (!detail) return;
  state.houseId = houseId;
  renderHouseList();
  renderHouseDetail();
  const card = el('house-list')?.querySelector(`[data-house-id="${houseId}"]`);
  card?.focus();
}

function renderHouseDetail() {
  const root = el('house-detail');
  if (!root) return;
  const detail = houseDetailForView(state.houseId);
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
        <ul>${detail.positiveReasons
          .map(
            (r) => `
          <li class="info-item info-${escapeHtml(r.visibility)}">
            ${visibilityBadge(r.visibility)}
            <p>${escapeHtml(r.text)}</p>
          </li>`,
          )
          .join('')}</ul>
      </section>
      <section class="detail-block reasons-negative">
        <h3>주요 부정 이유</h3>
        <ul>${detail.negativeReasons
          .map(
            (r) => `
          <li class="info-item info-${escapeHtml(r.visibility)}">
            ${visibilityBadge(r.visibility)}
            <p>${escapeHtml(r.text)}</p>
          </li>`,
          )
          .join('')}</ul>
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
  const stanceText =
    state.session.phase === 'resolved' && state.session.world.player.stanceText
      ? state.session.world.player.stanceText
      : player.houseStanceText;

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
      <span class="player-summary">
        <span class="eyebrow">플레이어</span>
        <span class="player-name">${escapeHtml(player.name)}</span>
        <span class="player-status">${escapeHtml(player.status)}</span>
        <span class="meta-line"><span class="field-label">직위</span> ${escapeHtml(player.office)}</span>
        <span class="meta-line"><span class="field-label">영지</span> ${escapeHtml(player.holding)}</span>
        <span class="meta-line"><span class="field-label">권리</span> ${escapeHtml(player.claimText)}</span>
        <span class="meta-line house-stance-line">${escapeHtml(stanceText)}</span>
        <span class="expand-hint" aria-hidden="true">${expanded ? '▲ 접기' : '▼ 관계와 압력 보기'}</span>
      </span>
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

function renderProposals() {
  const root = el('proposal-list');
  if (!root) return;
  root.innerHTML = '';
  root.setAttribute('role', 'list');

  PROPOSALS.forEach((proposal, index) => {
    const expanded = state.session.expandedProposalId === proposal.id;
    const card = document.createElement('div');
    card.className = `proposal-card${expanded ? ' is-expanded' : ''}`;
    card.setAttribute('role', 'listitem');

    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'proposal-toggle';
    btn.id = `proposal-toggle-${proposal.id}`;
    btn.setAttribute('aria-expanded', expanded ? 'true' : 'false');
    btn.setAttribute('aria-controls', `proposal-body-${proposal.id}`);
    btn.dataset.proposalId = proposal.id;
    btn.tabIndex = 0;
    btn.innerHTML = `
      <span class="card-top">
        <span class="card-label">제안 ${index + 1}</span>
        ${visibilityBadge(proposal.visibility)}
      </span>
      <span class="proposal-proposer">${escapeHtml(proposal.proposer)}</span>
      <span class="proposal-demand-preview">${escapeHtml(proposal.demand)}</span>
      <span class="expand-hint" aria-hidden="true">${expanded ? '▲ 접기' : '▼ 세부 보기'}</span>
    `;
    btn.addEventListener('click', () => {
      state.session.expandedProposalId =
        state.session.expandedProposalId === proposal.id ? null : proposal.id;
      renderProposals();
      el(`proposal-toggle-${proposal.id}`)?.focus();
    });

    const body = document.createElement('div');
    body.id = `proposal-body-${proposal.id}`;
    body.className = 'proposal-body';
    body.hidden = !expanded;
    body.innerHTML = `
      <dl class="proposal-facts">
        <div>
          <dt>제안자</dt>
          <dd>${escapeHtml(proposal.proposer)}</dd>
        </div>
        <div>
          <dt>요구 행동</dt>
          <dd>${escapeHtml(proposal.demand)}</dd>
        </div>
        <div>
          <dt>제안하는 이익</dt>
          <dd>${escapeHtml(proposal.benefit)}</dd>
        </div>
        <div>
          <dt>거부·배신 시 위험</dt>
          <dd>${escapeHtml(proposal.risk)}</dd>
        </div>
        <div>
          <dt>정보 상태</dt>
          <dd>${visibilityBadge(proposal.visibility)}</dd>
        </div>
        <div>
          <dt>관련 후보·가문</dt>
          <dd>${escapeHtml(proposal.relatedLabel)}</dd>
        </div>
      </dl>
    `;

    card.appendChild(btn);
    card.appendChild(body);
    root.appendChild(card);
  });
}

function renderActions() {
  const root = el('action-list');
  if (!root) return;
  root.innerHTML = '';
  const resolved = state.session.phase === 'resolved';
  root.setAttribute('role', 'listbox');
  root.setAttribute('aria-label', '플레이어 행동');
  root.setAttribute('aria-disabled', resolved ? 'true' : 'false');

  ACTIONS.forEach((action, index) => {
    const selected = state.session.selectedActionId === action.id;
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = [
      'action-card',
      selected ? 'is-selected' : '',
      resolved ? 'is-locked' : '',
    ]
      .filter(Boolean)
      .join(' ');
    btn.setAttribute('role', 'option');
    btn.setAttribute('aria-selected', selected ? 'true' : 'false');
    btn.disabled = resolved;
    btn.dataset.actionId = action.id;
    btn.tabIndex = selected || (!state.session.selectedActionId && index === 0) ? 0 : -1;
    const mark = selected
      ? '<span class="selected-mark" aria-hidden="true">✓ 선택됨</span>'
      : '';
    btn.setAttribute(
      'aria-label',
      `행동 ${action.code}. ${action.label}. ${selected ? '선택됨' : ''}`,
    );
    const asLines = (items, extraClass = '') =>
      items
        .map(
          (b) =>
            `<span class="action-line${extraClass ? ` ${extraClass}` : ''}">· ${escapeHtml(b)}</span>`,
        )
        .join('');
    btn.innerHTML = `
      <span class="card-top">
        <span class="card-label">행동 ${escapeHtml(action.code)}</span>
        ${mark}
      </span>
      <span class="action-label">${escapeHtml(action.label)}</span>
      <span class="action-meta"><span class="field-label">돕는 대상</span> ${escapeHtml(action.helps)}</span>
      <span class="action-meta"><span class="field-label">즉시 이익</span></span>
      ${asLines(action.benefits)}
      <span class="action-meta"><span class="field-label">직접 손실</span></span>
      ${asLines(action.losses)}
      <span class="action-meta"><span class="field-label">주요 위험</span></span>
      ${asLines(action.risks, 'action-risk-line')}
      <span class="action-meta"><span class="field-label">영향 대상</span> ${escapeHtml(action.affected.join(', '))}</span>
    `;
    if (!resolved) {
      btn.addEventListener('click', () => {
        selectAction(state.session, action.id);
        renderActions();
        renderConfirm();
        el(`confirm-panel`)?.querySelector('#btn-confirm')?.focus();
      });
      btn.addEventListener('keydown', (e) => onActionKeydown(e, index));
    }
    root.appendChild(btn);
  });
}

function onActionKeydown(e, index) {
  if (state.session.phase === 'resolved') return;
  const count = ACTIONS.length;
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
    selectAction(state.session, ACTIONS[index].id);
    renderActions();
    renderConfirm();
    el('confirm-panel')?.querySelector('#btn-confirm')?.focus();
    return;
  }
  if (next !== null) {
    selectAction(state.session, ACTIONS[next].id);
    renderActions();
    renderConfirm();
    const cards = el('action-list')?.querySelectorAll('.action-card');
    cards?.[next]?.focus();
  }
}

function renderConfirm() {
  const root = el('confirm-panel');
  if (!root) return;
  const phase = state.session.phase;
  if (phase !== 'decision' || !state.session.selectedActionId) {
    root.hidden = true;
    root.innerHTML = '';
    return;
  }
  const action = ACTIONS.find((a) => a.id === state.session.selectedActionId);
  if (!action) {
    root.hidden = true;
    return;
  }
  root.hidden = false;
  root.innerHTML = `
    <div class="confirm-header">
      <h3 id="confirm-title">선택 확인</h3>
      <p class="confirm-chosen">선택한 행동: <strong>${escapeHtml(action.label)}</strong></p>
      <p class="section-help">이익·손실·위험을 다시 확인한 뒤 확정하십시오. 확정 후에는 이 실행에서 다른 행동을 중첩할 수 없습니다.</p>
    </div>
    <div class="confirm-summary">
      <p><span class="field-label">돕는 대상</span> ${escapeHtml(action.helps)}</p>
      <p><span class="field-label">즉시 이익</span> ${escapeHtml(action.benefits.join(' · '))}</p>
      <p><span class="field-label">직접 손실</span> ${escapeHtml(action.losses.join(' · '))}</p>
      <p><span class="field-label">주요 위험</span> ${escapeHtml(action.risks.join(' · '))}</p>
    </div>
    <div class="confirm-actions">
      <button type="button" class="btn btn-primary" id="btn-confirm">선택 확정</button>
      <button type="button" class="btn btn-secondary" id="btn-cancel">돌아가기</button>
    </div>
  `;
  el('btn-confirm')?.addEventListener('click', () => {
    confirmAction(state.session);
    renderAll();
    const outcomeRoot = el('outcome-panel');
    outcomeRoot?.focus();
    outcomeRoot?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  });
  el('btn-cancel')?.addEventListener('click', () => {
    cancelDecision(state.session);
    renderActions();
    renderConfirm();
    const first = el('action-list')?.querySelector('.action-card');
    first?.focus();
  });
}

function renderOutcome() {
  const section = el('outcome-section');
  const root = el('outcome-panel');
  if (!section || !root) return;

  if (state.session.phase !== 'resolved' || !state.session.result) {
    section.hidden = true;
    root.innerHTML = '';
    return;
  }

  const { outcome, world } = state.session.result;
  const dc = outcome.directChanges;
  section.hidden = false;

  const list = (items) =>
    items.map((t) => `<li>${escapeHtml(t)}</li>`).join('');

  const newInfo =
    world.newPublicInfo?.length > 0
      ? `<ul class="info-list">${world.newPublicInfo
          .map(
            (info) => `
        <li class="info-item info-${escapeHtml(info.visibility)}">
          ${visibilityBadge(info.visibility)}
          <p>${escapeHtml(info.text)}</p>
        </li>`,
          )
          .join('')}</ul>`
      : '<p class="empty-hint">새로 공개된 정보 없음</p>';

  root.innerHTML = `
    <div class="outcome-header">
      <p class="eyebrow">결과</p>
      <h2 id="outcome-heading">행동 결과</h2>
    </div>

    <section class="detail-block outcome-block" aria-labelledby="outcome-chosen-title">
      <h3 id="outcome-chosen-title">선택한 행동</h3>
      <p><span class="field-label">행동</span> ${escapeHtml(outcome.chosenLabel)}</p>
      <p><span class="field-label">응답한 제안</span> ${escapeHtml(outcome.responseTo)}</p>
      <p><span class="field-label">돕거나 거부</span> ${escapeHtml(outcome.helpedOrRefused)}</p>
    </section>

    <section class="detail-block outcome-block" aria-labelledby="outcome-direct-title">
      <h3 id="outcome-direct-title">당신이 직접 바꾼 것</h3>
      <p><span class="field-label">플레이어 입장</span> ${escapeHtml(dc.playerStance)}</p>
      <p class="field-label">관계 변화</p>
      <ul>${list(dc.relationChanges)}</ul>
      <p class="field-label">얻은 이익</p>
      <ul>${list(dc.benefitsGained)}</ul>
      <p class="field-label">발생한 위험</p>
      <ul class="action-risks">${list(dc.risksCreated)}</ul>
    </section>

    <section class="detail-block outcome-block" aria-labelledby="outcome-ripple-title">
      <h3 id="outcome-ripple-title">주요 파급</h3>
      <ul>${list(outcome.ripples)}</ul>
      <p class="field-label">새로 공개되거나 변경된 정보</p>
      ${newInfo}
    </section>

    <section class="detail-block outcome-block" aria-labelledby="outcome-why-title">
      <h3 id="outcome-why-title">왜 이런 결과가 발생했는가</h3>
      <ul class="reason-list">${list(outcome.reasons)}</ul>
    </section>

    <section class="detail-block outcome-block" aria-labelledby="outcome-unchanged-title">
      <h3 id="outcome-unchanged-title">바뀌지 않은 것</h3>
      <ul>${list(outcome.unchanged)}</ul>
    </section>

    <div class="outcome-actions">
      <button type="button" class="btn btn-primary" id="btn-retry">다른 선택 시도</button>
    </div>
  `;

  el('btn-retry')?.addEventListener('click', () => {
    resetSession(state.session);
    renderAll();
    const firstAction = el('action-list')?.querySelector('.action-card');
    firstAction?.focus();
    el('actions-section')?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  });
}

function renderAll() {
  renderCrisis();
  renderLegend();
  renderCandidateList();
  renderCandidateDetail();
  renderHouseList();
  renderHouseDetail();
  renderPlayer();
  renderProposals();
  renderActions();
  renderConfirm();
  renderOutcome();
}

export function initApp() {
  renderAll();
}

export function trySelectCandidate(id) {
  selectCandidate(id);
  return state.candidateId;
}

export function trySelectHouse(id) {
  selectHouse(id);
  return state.houseId;
}

export function getAppState() {
  return {
    candidateId: state.candidateId,
    houseId: state.houseId,
    playerExpanded: state.playerExpanded,
    phase: state.session.phase,
    selectedActionId: state.session.selectedActionId,
  };
}

if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initApp);
  } else {
    initApp();
  }
}
