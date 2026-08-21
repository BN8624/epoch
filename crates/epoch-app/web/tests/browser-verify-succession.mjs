// export-succession 관찰 사이트에서 공석·3인 후보·금지 표현을 검증한다
import { spawn, spawnSync } from 'child_process';
import http from 'http';
import fsp from 'fs/promises';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { resolveChromePath, chromeNotFoundMessage } from './chrome-path.mjs';
import {
  buildIndexes,
  getInitialSelection,
  getRealmView,
  getSuccessionDisputeView,
  getVisibleInformation,
} from '../view-model.js';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const WORKSPACE = path.resolve(HERE, '../../../..');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
};

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function startStaticServer(root) {
  const server = http.createServer(async (req, res) => {
    try {
      const rawPath = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
      const rel = rawPath === '/' ? 'index.html' : rawPath.replace(/^\/+/, '');
      const target = path.resolve(root, rel);
      if (target !== root && !target.startsWith(root + path.sep)) {
        res.writeHead(403).end('forbidden');
        return;
      }
      const body = await fsp.readFile(target);
      res.writeHead(200, { 'content-type': MIME[path.extname(target)] ?? 'application/octet-stream' });
      res.end(body);
    } catch {
      res.writeHead(404).end('not found');
    }
  });

  return new Promise((resolve, reject) => {
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      resolve({ server, port, url: `http://127.0.0.1:${port}/` });
    });
  });
}

function closeServer(server) {
  return new Promise((resolve) => {
    if (!server) {
      resolve();
      return;
    }
    server.closeAllConnections?.();
    server.close(() => resolve());
  });
}

async function readDevToolsPort(userDataDir, timeoutMs = 20000) {
  const portFile = path.join(userDataDir, 'DevToolsActivePort');
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const text = await fsp.readFile(portFile, 'utf8');
      const port = parseInt(text.split('\n')[0].trim(), 10);
      if (Number.isInteger(port) && port > 0) return port;
    } catch {
      /* 아직 생성되지 않음 */
    }
    await sleep(150);
  }
  throw new Error(`Chrome did not report a DevTools port within ${timeoutMs}ms (${portFile}).`);
}

function getJson(port, urlPath) {
  return new Promise((resolve, reject) => {
    http
      .get(`http://127.0.0.1:${port}${urlPath}`, (res) => {
        let d = '';
        res.on('data', (c) => {
          d += c;
        });
        res.on('end', () => {
          try {
            resolve(JSON.parse(d));
          } catch (e) {
            reject(e);
          }
        });
      })
      .on('error', reject);
  });
}

class Cdp {
  constructor(wsUrl) {
    this.ws = new WebSocket(wsUrl);
    this.id = 0;
    this.pending = new Map();
    this.console = [];
  }

  ready() {
    this.ws.addEventListener('message', (ev) => {
      const msg = JSON.parse(ev.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(JSON.stringify(msg.error)));
        else resolve(msg.result);
      }
      if (msg.method === 'Runtime.consoleAPICalled') {
        this.console.push(msg.params);
      }
      if (msg.method === 'Runtime.exceptionThrown') {
        this.console.push({ type: 'exception', ...msg.params });
      }
    });
    return new Promise((resolve, reject) => {
      this.ws.addEventListener('open', resolve);
      this.ws.addEventListener('error', reject);
    });
  }

  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  async eval(expression) {
    const r = await this.send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  }

  async key(key, code, windowsVirtualKeyCode) {
    await this.send('Input.dispatchKeyEvent', { type: 'keyDown', key, code, windowsVirtualKeyCode });
    await this.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code, windowsVirtualKeyCode });
  }

  close() {
    try {
      this.ws.close();
    } catch {
      /* 이미 닫힘 */
    }
  }
}

function exportSuccessionSite() {
  const outDir = path.join(os.tmpdir(), `epoch-app-succession-${process.pid}-${Date.now()}`);
  const result = spawnSync(
    'cargo',
    ['run', '-q', '-p', 'epoch-app', '--', 'export-succession', '1', 'realm-01', outDir],
    {
      cwd: WORKSPACE,
      encoding: 'utf8',
      shell: process.platform === 'win32',
    },
  );
  if (result.status !== 0) {
    throw new Error(`export-succession failed:\n${result.stdout || ''}\n${result.stderr || ''}`);
  }
  return outDir;
}

function consoleErrors(cdp) {
  return cdp.console.filter(
    (entry) => entry.type === 'error' || entry.type === 'exception' || entry.exceptionDetails,
  );
}

let site = null;
let chrome = null;
let cdp = null;
let userDataDir = null;
let exportDir = null;

async function cleanup() {
  if (cdp) cdp.close();
  if (chrome && chrome.exitCode === null) {
    chrome.kill();
    for (let i = 0; i < 20 && chrome.exitCode === null; i++) await sleep(100);
    if (chrome.exitCode === null) chrome.kill('SIGKILL');
  }
  await closeServer(site?.server);
  if (userDataDir) {
    try {
      await fsp.rm(userDataDir, { recursive: true, force: true, maxRetries: 5 });
    } catch {
      /* 임시 디렉터리 정리 실패는 검증 결과를 바꾸지 않는다 */
    }
  }
  if (exportDir) {
    try {
      await fsp.rm(exportDir, { recursive: true, force: true, maxRetries: 5 });
    } catch {
      /* 동일 */
    }
  }
}

const failures = [];

function check(name, ok, detail) {
  if (!ok) failures.push(detail ? `${name}: ${detail}` : name);
}

try {
  exportDir = exportSuccessionSite();
  const world = JSON.parse(await fsp.readFile(path.join(exportDir, 'rights-world.json'), 'utf8'));
  const succession = JSON.parse(
    await fsp.readFile(path.join(exportDir, 'succession-world.json'), 'utf8'),
  );
  const idx = buildIndexes(world, succession);
  const dispute = getSuccessionDisputeView(idx, 'realm-01');
  const realm = getRealmView(idx, 'realm-01');
  const initial = getInitialSelection(idx);
  const persons = world.context_world.political.dynastic.population.persons;
  const claims = world.rights.claims;
  const derivedClaims = succession.pre_succession_world.propagation.derived_claims;
  const rawCandidates = succession.transition.candidates;
  const rawA = rawCandidates.find((item) => item.priority === 'direct_strong_original');
  const rawB = rawCandidates.find((item) => item.priority === 'restored_contested_original');
  const rawC = rawCandidates.find((item) => item.priority === 'restored_contested_derived');
  const derivedRecord = derivedClaims.find((item) => item.id === rawC.claim_record_id);
  const sourceClaim = claims.find((item) => item.id === derivedRecord.source_claim_id);
  const childOfC = persons.find((item) => item.id === rawC.person_id);
  const sourcePerson = persons.find((item) => item.id === sourceClaim.claimant_person_id);
  const formerName = persons.find(
    (item) => item.id === succession.transition.death.person_id,
  ).name;
  const candA = dispute.candidateA;
  const candB = dispute.candidateB;
  const candC = dispute.candidateC;

  site = await startStaticServer(exportDir);

  const chromeInfo = resolveChromePath();
  if (!chromeInfo.path) throw new Error(chromeNotFoundMessage(chromeInfo.tried));

  userDataDir = await fsp.mkdtemp(path.join(os.tmpdir(), 'epoch-app-chrome-succession-'));
  chrome = spawn(
    chromeInfo.path,
    [
      '--remote-debugging-port=0',
      `--user-data-dir=${userDataDir}`,
      '--headless=new',
      '--disable-gpu',
      '--no-first-run',
      '--no-default-browser-check',
      '--window-size=1280,720',
      'about:blank',
    ],
    { stdio: 'ignore' },
  );

  const devtoolsPort = await readDevToolsPort(userDataDir);
  let pages = null;
  for (let i = 0; i < 25; i++) {
    try {
      pages = await getJson(devtoolsPort, '/json/list');
      if (pages.length) break;
    } catch {
      /* 재시도 */
    }
    await sleep(200);
  }
  if (!pages?.length) throw new Error(`No CDP pages on port ${devtoolsPort}.`);

  const page = pages.find((p) => p.type === 'page') || pages[0];
  cdp = new Cdp(page.webSocketDebuggerUrl);
  await cdp.ready();
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');
  await cdp.send('Page.navigate', { url: site.url });

  const readyDeadline = Date.now() + 20000;
  let readyState = null;
  while (Date.now() < readyDeadline) {
    readyState = await cdp.eval(`({
      tiles: document.querySelectorAll('.territory-tile').length,
      candidates: document.querySelectorAll('.dispute-candidate-card').length,
      houses: document.querySelectorAll('.dispute-house-card').length,
      workspaceHidden: document.getElementById('succession-workspace')?.hidden ?? true,
      documentState: document.readyState,
    })`);
    if (
      readyState.tiles === 36 &&
      readyState.candidates === 3 &&
      readyState.houses === 3 &&
      readyState.workspaceHidden === false
    ) {
      break;
    }
    await sleep(200);
  }
  check(
    'page-ready',
    readyState?.tiles === 36 &&
      readyState?.candidates === 3 &&
      readyState?.houses === 3 &&
      readyState?.workspaceHidden === false,
    JSON.stringify(readyState),
  );

  const snapshot = await cdp.eval(`({
    skipHref: document.querySelector('.skip-link')?.getAttribute('href'),
    mainTag: document.getElementById('workspace')?.tagName,
    successionInMain: Boolean(document.getElementById('workspace')?.querySelector('#succession-workspace')),
    worldInMain: Boolean(document.getElementById('workspace')?.querySelector('#world-context')),
    workspaceHidden: document.getElementById('succession-workspace')?.hidden ?? true,
    contextHeading: document.getElementById('world-context-heading')?.textContent.trim(),
    contextBannerHidden: document.getElementById('world-context-banner')?.hidden ?? true,
    crisisHidden: document.getElementById('crisis-panel')?.hidden ?? true,
    vacancy: document.querySelector('#dispute-crisis [data-role="vacancy"]')?.textContent.trim(),
    former: document.querySelector('#dispute-crisis [data-role="former-incumbent"]')?.textContent.trim(),
    realmName: document.querySelector('#dispute-crisis .eyebrow')?.textContent.trim(),
    heading: document.getElementById('dispute-crisis-heading')?.textContent.trim(),
    incumbentLine: document.querySelector('#realm-detail')?.innerText || '',
    candidates: [...document.querySelectorAll('.dispute-candidate-card')].map((el) => ({
      name: el.querySelector('.card-name')?.textContent.trim(),
      personId: el.getAttribute('data-person-id'),
      slot: el.getAttribute('data-candidate-slot'),
      priority: el.getAttribute('data-candidate-priority'),
      origin: el.getAttribute('data-candidate-origin'),
      claim: el.getAttribute('data-claim-record'),
      selected: el.getAttribute('aria-selected') === 'true',
      tabIndex: el.tabIndex,
      text: el.innerText,
    })),
    houses: [...document.querySelectorAll('.dispute-house-card')].map((el) => ({
      name: el.querySelector('.card-name')?.textContent.trim(),
      houseId: el.getAttribute('data-house-id'),
      deceased: el.getAttribute('data-deceased-head'),
      selected: el.getAttribute('aria-selected') === 'true',
      tabIndex: el.tabIndex,
      text: el.innerText,
    })),
    derivedSource: document.querySelector('[data-derived-source]')?.getAttribute('data-derived-source'),
    derivedLineage: document.querySelector('[data-derived-lineage]')?.textContent.trim(),
    houseDetail: document.getElementById('dispute-house-detail')?.innerText || '',
    html: document.documentElement.outerHTML,
    pageText: document.body.innerText,
  })`);

  check('skip-to-main', snapshot.skipHref === '#workspace', snapshot.skipHref);
  check(
    'main-contains-primary',
    snapshot.mainTag === 'MAIN' && snapshot.successionInMain && snapshot.worldInMain,
    JSON.stringify({
      mainTag: snapshot.mainTag,
      successionInMain: snapshot.successionInMain,
      worldInMain: snapshot.worldInMain,
    }),
  );
  check('workspace-visible', snapshot.workspaceHidden === false, String(snapshot.workspaceHidden));
  check('world-context-visible', snapshot.contextBannerHidden === false, String(snapshot.contextBannerHidden));
  check('world-context-title', snapshot.contextHeading === '세계 맥락', snapshot.contextHeading);
  check('legacy-crisis-hidden', snapshot.crisisHidden === true, String(snapshot.crisisHidden));
  check('realm-name', snapshot.realmName === dispute.realmName, snapshot.realmName);
  check('crisis-heading', snapshot.heading === '계승 분쟁', snapshot.heading);
  check('vacancy-visible', snapshot.vacancy === '공석', snapshot.vacancy);
  check(
    'former-matches-data',
    snapshot.former === `${formerName} · 사망`,
    snapshot.former,
  );
  check(
    'dead-not-current-ruler',
    !snapshot.incumbentLine.split('\n').some((line) => line === `통치자${formerName}` || line === `통치자 ${formerName}`)
      && snapshot.incumbentLine.includes('공석'),
    snapshot.incumbentLine.slice(0, 240),
  );
  check('candidate-count', snapshot.candidates.length === 3, String(snapshot.candidates.length));
  check(
    'candidate-slots',
    snapshot.candidates.map((item) => item.slot).join('') === 'ABC',
    snapshot.candidates.map((item) => item.slot).join(','),
  );

  const shownA = snapshot.candidates.find((item) => item.slot === 'A');
  const shownB = snapshot.candidates.find((item) => item.slot === 'B');
  const shownC = snapshot.candidates.find((item) => item.slot === 'C');
  check('direct-priority', shownA?.personId === candA.personId, JSON.stringify(shownA));
  check(
    'direct-label',
    Boolean(shownA?.text.includes('강한 직계 권리') && shownA?.text.includes('법적 우선 후보')),
    shownA?.text,
  );
  check('restored-shown', shownB?.personId === candB.personId, JSON.stringify(shownB));
  check(
    'restored-label',
    Boolean(shownB?.text.includes('논쟁 중인 복권 권리') && shownB?.text.includes('경쟁 권리자')),
    shownB?.text,
  );
  check('derived-shown', shownC?.personId === candC.personId, JSON.stringify(shownC));
  check(
    'derived-label',
    Boolean(shownC?.text.includes('혈통을 따라 파생된 복권 권리')),
    shownC?.text,
  );
  check('raw-c-claim', rawC.claim_record_id === 'derived-claim-01', rawC.claim_record_id);
  check('raw-c-source-claim', derivedRecord.source_claim_id === sourceClaim.id, derivedRecord.source_claim_id);
  check('raw-c-source-is-b', sourceClaim.claimant_person_id === rawB.person_id, sourceClaim.claimant_person_id);
  check(
    'raw-c-via-parent',
    derivedRecord.via_parent_person_id === sourceClaim.claimant_person_id,
    derivedRecord.via_parent_person_id,
  );
  check(
    'raw-c-parentage',
    (childOfC.known_parent_ids ?? []).includes(sourceClaim.claimant_person_id),
    JSON.stringify(childOfC.known_parent_ids),
  );
  check(
    'derived-source',
    snapshot.derivedSource === sourceClaim.claimant_person_id,
    `${snapshot.derivedSource} != ${sourceClaim.claimant_person_id}`,
  );
  check(
    'derived-lineage',
    snapshot.derivedLineage === `${sourcePerson.name}의 자녀로서 복권 권리가 한 세대 전파됨`,
    snapshot.derivedLineage,
  );
  check('priority-name', shownA?.name === idx.personById[candA.personId].name, shownA?.name);
  check('restored-name', shownB?.name === idx.personById[candB.personId].name, shownB?.name);
  check('derived-name', shownC?.name === idx.personById[candC.personId].name, shownC?.name);
  check('house-count', snapshot.houses.length === 3, String(snapshot.houses.length));
  for (const house of snapshot.houses) {
    check(`house-name:${house.houseId}`, house.name === idx.houseById[house.houseId].name, house.name);
  }
  const h0 = snapshot.houses.find((item) => item.houseId === 'house-01');
  check('h0-deceased-flag', h0?.deceased === 'true', JSON.stringify(h0));
  check('h0-deceased-copy', Boolean(h0?.text.includes('사망') && h0?.text.includes('미결정')), h0?.text);
  check('h0-not-current-head', !/현재 가문 수장/.test(h0?.text ?? ''), h0?.text);
  check(
    'h0-detail-deceased',
    snapshot.houseDetail.includes('기존 수장') && snapshot.houseDetail.includes('사망 전에 알고 있던'),
    snapshot.houseDetail.slice(0, 280),
  );
  check('default-candidate-a', shownA?.selected === true && shownA?.tabIndex === 0, JSON.stringify(shownA));
  check(
    'roving-candidates',
    snapshot.candidates.filter((item) => item.tabIndex === 0).length === 1 &&
      snapshot.candidates.filter((item) => item.tabIndex === -1).length === 2,
    JSON.stringify(snapshot.candidates.map((item) => item.tabIndex)),
  );

  await cdp.eval(
    `document.querySelector('.dispute-candidate-card[data-person-id="${rawB.person_id}"]')?.click()`,
  );
  await sleep(150);
  const afterClick = await cdp.eval(`({
    disputeSelected: document.querySelector('.dispute-candidate-card.is-selected')?.getAttribute('data-person-id'),
    disputeDetail: document.querySelector('#dispute-candidate-detail-title')?.textContent.trim(),
    observerSelected: document.querySelector('.person-card.is-selected')?.getAttribute('data-person-id'),
    focus: document.activeElement?.classList.contains('dispute-candidate-card')
      ? document.activeElement.getAttribute('data-person-id')
      : null,
    aria: document.querySelector('.dispute-candidate-card.is-selected')?.getAttribute('aria-selected'),
  })`);
  check('candidate-selectable', afterClick.disputeSelected === rawB.person_id, JSON.stringify(afterClick));
  check(
    'candidate-detail-changes',
    afterClick.disputeDetail === persons.find((item) => item.id === rawB.person_id).name,
    afterClick.disputeDetail,
  );
  check('observer-person-synced', afterClick.observerSelected === rawB.person_id, afterClick.observerSelected);
  check('candidate-focus', afterClick.focus === rawB.person_id, afterClick.focus);
  check('candidate-aria', afterClick.aria === 'true', afterClick.aria);

  async function rovingState(listSelector, idAttr) {
    return cdp.eval(`(() => {
      const cards = [...document.querySelectorAll(${JSON.stringify(listSelector)})];
      const selected = cards.find((el) => el.getAttribute('aria-selected') === 'true');
      const focus = document.activeElement;
      return {
        ids: cards.map((el) => el.getAttribute(${JSON.stringify(idAttr)})),
        selectedId: selected?.getAttribute(${JSON.stringify(idAttr)}) ?? null,
        tabIndexes: cards.map((el) => el.tabIndex),
        aria: cards.map((el) => el.getAttribute('aria-selected')),
        focusId: focus?.getAttribute(${JSON.stringify(idAttr)}) ?? null,
        focusMatches: Boolean(focus?.matches(${JSON.stringify(listSelector)})),
      };
    })()`);
  }

  function checkRoving(label, state, expectedId) {
    const selectedIndex = state.ids.indexOf(expectedId);
    check(`${label}-selected`, state.selectedId === expectedId, JSON.stringify(state));
    check(
      `${label}-focus`,
      state.focusId === expectedId && state.focusMatches,
      JSON.stringify(state),
    );
    check(
      `${label}-tabindex`,
      selectedIndex >= 0 &&
        state.tabIndexes.length === 3 &&
        state.tabIndexes.every((value, index) => (index === selectedIndex ? value === 0 : value === -1)),
      JSON.stringify(state.tabIndexes),
    );
    check(
      `${label}-aria`,
      selectedIndex >= 0 &&
        state.aria.every((value, index) => (index === selectedIndex ? value === 'true' : value === 'false')),
      JSON.stringify(state.aria),
    );
  }

  async function press(key, code, vk) {
    await cdp.key(key, code, vk);
    await sleep(120);
  }

  const candidateIds = [rawA.person_id, rawB.person_id, rawC.person_id];
  await cdp.eval(
    `document.querySelector('.dispute-candidate-card[data-person-id="${rawA.person_id}"]')?.click()`,
  );
  await sleep(120);
  await cdp.eval(
    `document.querySelector('.dispute-candidate-card[data-person-id="${rawA.person_id}"]')?.focus()`,
  );
  checkRoving('cand-start', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[0]);
  await press('ArrowRight', 'ArrowRight', 39);
  checkRoving('cand-right', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[1]);
  await press('ArrowDown', 'ArrowDown', 40);
  checkRoving('cand-down', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[2]);
  await press('ArrowLeft', 'ArrowLeft', 37);
  checkRoving('cand-left', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[1]);
  await press('ArrowUp', 'ArrowUp', 38);
  checkRoving('cand-up', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[0]);
  await press('End', 'End', 35);
  checkRoving('cand-end', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[2]);
  await press('Home', 'Home', 36);
  checkRoving('cand-home', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[0]);
  await cdp.eval(
    `document.querySelector('.dispute-candidate-card[data-person-id="${candidateIds[1]}"]')?.focus()`,
  );
  await press('Enter', 'Enter', 13);
  checkRoving('cand-enter', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[1]);
  await cdp.eval(
    `document.querySelector('.dispute-candidate-card[data-person-id="${candidateIds[2]}"]')?.focus()`,
  );
  await press(' ', 'Space', 32);
  checkRoving('cand-space', await rovingState('.dispute-candidate-card', 'data-person-id'), candidateIds[2]);

  await cdp.eval(`document.querySelector('.dispute-house-card[data-house-id="house-03"]')?.click()`);
  await sleep(150);
  const afterHouse = await cdp.eval(`({
    disputeSelected: document.querySelector('.dispute-house-card.is-selected')?.getAttribute('data-house-id'),
    disputeTitle: document.querySelector('#dispute-house-detail-title')?.textContent.trim(),
    observerSelected: document.querySelector('.house-card.is-selected')?.getAttribute('data-house-id'),
    detail: document.getElementById('dispute-house-detail')?.innerText || '',
  })`);
  check('house-selectable', afterHouse.disputeSelected === 'house-03', JSON.stringify(afterHouse));
  check(
    'house-detail-changes',
    afterHouse.disputeTitle === idx.houseById['house-03'].name,
    afterHouse.disputeTitle,
  );
  check('observer-house-synced', afterHouse.observerSelected === 'house-03', afterHouse.observerSelected);
  check(
    'house03-no-confirmed-conflict',
    !afterHouse.detail.includes('같은 평의회 자리를 두 가문에') && !afterHouse.detail.includes('소문'),
    afterHouse.detail.slice(0, 240),
  );

  const houseIds = snapshot.houses.map((item) => item.houseId);
  await cdp.eval(
    `document.querySelector('.dispute-house-card[data-house-id="${houseIds[0]}"]')?.click()`,
  );
  await sleep(120);
  await cdp.eval(
    `document.querySelector('.dispute-house-card[data-house-id="${houseIds[0]}"]')?.focus()`,
  );
  checkRoving('house-start', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[0]);
  await press('ArrowRight', 'ArrowRight', 39);
  checkRoving('house-right', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[1]);
  await press('ArrowDown', 'ArrowDown', 40);
  checkRoving('house-down', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[2]);
  await press('ArrowLeft', 'ArrowLeft', 37);
  checkRoving('house-left', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[1]);
  await press('ArrowUp', 'ArrowUp', 38);
  checkRoving('house-up', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[0]);
  await press('End', 'End', 35);
  checkRoving('house-end', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[2]);
  await press('Home', 'Home', 36);
  checkRoving('house-home', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[0]);
  await cdp.eval(
    `document.querySelector('.dispute-house-card[data-house-id="${houseIds[2]}"]')?.focus()`,
  );
  await press('Enter', 'Enter', 13);
  checkRoving('house-enter', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[2]);
  await cdp.eval(
    `document.querySelector('.dispute-house-card[data-house-id="${houseIds[1]}"]')?.focus()`,
  );
  await press(' ', 'Space', 32);
  checkRoving('house-space', await rovingState('.dispute-house-card', 'data-house-id'), houseIds[1]);

  async function renderedInfo(rootSelector) {
    const selector = rootSelector + ' .info-card[data-info-id]';
    return cdp.eval(
      '(function(){var nodes=document.querySelectorAll(' +
        JSON.stringify(selector) +
        ');var out=[];for(var i=0;i<nodes.length;i++){var el=nodes[i];out.push({id:el.getAttribute("data-info-id"),topic:el.getAttribute("data-info-topic"),scope:el.getAttribute("data-info-scope"),confidence:el.getAttribute("data-info-confidence"),text:el.innerText});}return out;})()',
    );
  }

  async function rootHtml(rootSelector) {
    return cdp.eval(
      'document.querySelector(' + JSON.stringify(rootSelector) + ') ? document.querySelector(' + JSON.stringify(rootSelector) + ').innerHTML : ""',
    );
  }

  function assertExactInfo(label, rendered, personId, html) {
    const expected = getVisibleInformation(idx, personId);
    const expectedIds = expected.map((item) => item.id).sort();
    const gotIds = rendered.map((item) => item.id).sort();
    check(`${label}-ids`, JSON.stringify(gotIds) === JSON.stringify(expectedIds), JSON.stringify({ gotIds, expectedIds }));
    const hiddenIds = idx.layers.context.information
      .filter((item) => !expectedIds.includes(item.id))
      .map((item) => item.id);
    for (const infoId of hiddenIds) {
      check(`${label}-hidden:${infoId}`, !html.includes(infoId), infoId);
    }
    return { rendered, expected };
  }

  for (const candidate of [rawA, rawB, rawC]) {
    await cdp.eval(
      `document.querySelector('.dispute-candidate-card[data-person-id="${candidate.person_id}"]')?.click()`,
    );
    await sleep(120);
    const rendered = await renderedInfo('#dispute-candidate-detail');
    const html = await rootHtml('#dispute-candidate-detail');
    assertExactInfo(`candidate-${candidate.person_id}`, rendered, candidate.person_id, html);
  }

  const publicInfo = idx.layers.context.information.filter((item) => item.scope === 'public');
  check('public-info-exists', publicInfo.length > 0, String(publicInfo.length));
  const houseHeads = {
    'house-01': idx.houseById['house-01'].head_person_id,
    'house-02': idx.houseById['house-02'].head_person_id,
    'house-03': idx.houseById['house-03'].head_person_id,
  };
  for (const [houseId, headId] of Object.entries(houseHeads)) {
    await cdp.eval(`document.querySelector('.dispute-house-card[data-house-id="${houseId}"]')?.click()`);
    await sleep(120);
    const rendered = await renderedInfo('#dispute-house-detail');
    const html = await rootHtml('#dispute-house-detail');
    const { expected } = assertExactInfo(`house-${houseId}`, rendered, headId, html);
    for (const item of publicInfo) {
      check(
        `house-${houseId}-public:${item.id}`,
        expected.some((visible) => visible.id === item.id) &&
          rendered.some((card) => card.id === item.id && card.scope === 'public'),
        item.id,
      );
    }
    const confirmedConflict = expected.filter(
      (item) => item.topic === 'promise_conflict' && item.confidence === 'confirmed',
    );
    const rumor = expected.filter(
      (item) => item.topic === 'promise_conflict' && item.confidence === 'unverified',
    );
    if (houseId === 'house-01') {
      check(
        'house01-confirmed-conflict',
        confirmedConflict.length > 0 &&
          rendered.some(
            (card) =>
              card.topic === 'promise_conflict' &&
              card.confidence === 'confirmed' &&
              card.text.includes('같은 평의회 자리를 두 가문에'),
          ),
        JSON.stringify(rendered),
      );
    }
    if (houseId === 'house-02') {
      check(
        'house02-rumor-only',
        rumor.length > 0 &&
          confirmedConflict.length === 0 &&
          rendered.some(
            (card) =>
              card.topic === 'promise_conflict' &&
              card.confidence === 'unverified' &&
              card.text.includes('소문'),
          ) &&
          !rendered.some((card) => card.topic === 'promise_conflict' && card.confidence === 'confirmed'),
        JSON.stringify(rendered),
      );
    }
    if (houseId === 'house-03') {
      check(
        'house03-unknown-head',
        !expected.some((item) => item.topic === 'promise_conflict') &&
          !rendered.some((card) => card.topic === 'promise_conflict') &&
          !html.includes('같은 평의회 자리를 두 가문에') &&
          !html.includes('소문'),
        html.slice(0, 240),
      );
    }
  }

  const pageTextNow = await cdp.eval(`document.body.innerText`);
  const htmlNow = await cdp.eval(`document.documentElement.outerHTML`);
  const banned = [
    '새 왕',
    '즉위',
    '최종 승자',
    '왕위 확보',
    '즉시 즉위',
    '승계 확정',
    '최종 당선',
    '왕위 정보',
    '공식 후계자',
    '다음 왕',
    '후계 순위',
    '확정된 승계자',
    '왕위 획득',
    '새 통치자',
  ];
  for (const phrase of banned) {
    check(`no-overclaim:${phrase}`, !pageTextNow.includes(phrase));
  }
  const fixtureNames = [
    '아르케온',
    '에드렌 4세',
    '세리아',
    '다리안',
    '미레아',
    '아르덴 가문',
    '바렌 가문',
    '소렌 가문',
    '메로바 가문',
  ];
  for (const phrase of fixtureNames) {
    check(`no-fixture:${phrase}`, !pageTextNow.includes(phrase));
  }
  const actionUi = ['플레이어 위치', '상충하는 제안', '플레이어 행동', '행동 확정', '혼인 수락', '지지 선언'];
  for (const phrase of actionUi) {
    check(`no-action-ui:${phrase}`, !pageTextNow.includes(phrase));
  }
  check('no-support-invention', !/공개 지지|A 지지|B 지지|C 지지/.test(pageTextNow), 'support copy present');
  const visibleNow = new Set(
    [
      ...getVisibleInformation(idx, rawA.person_id),
      ...getVisibleInformation(idx, rawB.person_id),
      ...getVisibleInformation(idx, rawC.person_id),
      ...Object.values(houseHeads).flatMap((headId) => getVisibleInformation(idx, headId)),
    ].map((item) => item.id),
  );
  for (const item of idx.layers.context.information) {
    if (visibleNow.has(item.id)) continue;
    check(`no-hidden-info-id:${item.id}`, !htmlNow.includes(item.id), item.id);
  }

  async function measure(w, h) {
    await cdp.send('Emulation.setDeviceMetricsOverride', {
      width: w,
      height: h,
      deviceScaleFactor: 1,
      mobile: w < 500,
    });
    await sleep(350);
    return cdp.eval(`({
      innerWidth: window.innerWidth,
      scrollWidth: document.documentElement.scrollWidth,
      clientWidth: document.documentElement.clientWidth,
      hasHScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
      issues: (() => {
        const vw = window.innerWidth;
        const issues = [];
        const visible = (el) => {
          if (!el || el.hidden) return false;
          const s = getComputedStyle(el);
          if (s.display === 'none' || s.visibility === 'hidden') return false;
          const r = el.getBoundingClientRect();
          return r.width > 0 && r.height > 0;
        };
        const core = [...document.querySelectorAll(
          '.summary,.map-grid,.realm-detail,.house-card,.person-card,.person-detail,.succession-workspace,.dispute-candidate-card,.dispute-house-card,.dispute-detail,h1,h2'
        )];
        for (const el of core) {
          if (!visible(el)) continue;
          const r = el.getBoundingClientRect();
          if (r.right > vw + 1) issues.push('overflow-x:' + (el.className || el.id || el.tagName));
        }
        return issues;
      })(),
    })`);
  }

  const desktop = await measure(1280, 720);
  check('desktop-overflow', !desktop.hasHScroll, JSON.stringify(desktop));
  check('desktop-layout', desktop.issues.length === 0, desktop.issues.join(','));

  const mobile = await measure(390, 664);
  check('mobile-overflow', !mobile.hasHScroll, JSON.stringify(mobile));
  check('mobile-layout', mobile.issues.length === 0, mobile.issues.join(','));

  await cdp.eval(`window.scrollTo(0, 0)`);
  await sleep(150);
  const firstScreen = await cdp.eval(`({
    vh: window.innerHeight,
    realm: (() => {
      const el = document.querySelector('#dispute-crisis .eyebrow');
      const r = el?.getBoundingClientRect();
      return r ? r.top >= 0 && r.bottom <= window.innerHeight : false;
    })(),
    death: (() => {
      const el = document.querySelector('#dispute-crisis [data-role="former-incumbent"]');
      const r = el?.getBoundingClientRect();
      return r ? r.top >= 0 && r.top < window.innerHeight : false;
    })(),
    vacancy: (() => {
      const el = document.querySelector('#dispute-crisis [data-role="vacancy"]');
      const r = el?.getBoundingClientRect();
      return r ? r.top >= 0 && r.top < window.innerHeight : false;
    })(),
    priority: (() => {
      const el = document.querySelector('.dispute-candidate-card[data-candidate-slot="A"]');
      const r = el?.getBoundingClientRect();
      return r ? r.top >= 0 && r.top < window.innerHeight : false;
    })(),
    candidateCount: document.querySelectorAll('.dispute-candidate-card').length,
  })`);
  check('mobile-first-realm', firstScreen.realm, JSON.stringify(firstScreen));
  check('mobile-first-death', firstScreen.death, JSON.stringify(firstScreen));
  check('mobile-first-vacancy', firstScreen.vacancy, JSON.stringify(firstScreen));
  check('mobile-first-priority', firstScreen.priority, JSON.stringify(firstScreen));
  check('mobile-first-three-candidates', firstScreen.candidateCount === 3, String(firstScreen.candidateCount));

  const errors = consoleErrors(cdp);
  check('console-errors', errors.length === 0, JSON.stringify(errors).slice(0, 400));
  check('initial-realm', initial.selectedRealmId === 'realm-01' || realm.id === 'realm-01', initial.selectedRealmId);

  if (failures.length) {
    throw new Error(`succession browser verify failed:\n- ${failures.join('\n- ')}`);
  }

  console.log('SUCCESSION_BROWSER_VERIFY_OK candidates=3 vacant=1 desktop=1280x720 mobile=390x664');
} catch (error) {
  console.error(error instanceof Error ? error.stack || error.message : error);
  process.exitCode = 1;
} finally {
  await cleanup();
}
