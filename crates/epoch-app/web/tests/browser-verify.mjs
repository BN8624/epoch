// 내보낸 관찰 사이트를 로컬 서버와 Chrome으로 열어 탐색·비대칭·레이아웃을 검증한다
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
  housesForRealm,
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

function exportSeed1Site() {
  const outDir = path.join(os.tmpdir(), `epoch-app-browser-${process.pid}-${Date.now()}`);
  const result = spawnSync(
    'cargo',
    ['run', '-q', '-p', 'epoch-app', '--', 'export', '1', outDir],
    {
      cwd: WORKSPACE,
      encoding: 'utf8',
      shell: process.platform === 'win32',
    },
  );
  if (result.status !== 0) {
    throw new Error(`export failed:\n${result.stdout || ''}\n${result.stderr || ''}`);
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
  exportDir = exportSeed1Site();
  const world = JSON.parse(await fsp.readFile(path.join(exportDir, 'rights-world.json'), 'utf8'));
  const idx = buildIndexes(world);
  const initial = getInitialSelection(idx);
  const firstRealm = getRealmView(idx, initial.selectedRealmId);
  const otherRealm = idx.layers.skeleton.realms.find((r) => r.id !== initial.selectedRealmId);
  const otherCapital = otherRealm.capital_territory_id;
  const houses = housesForRealm(idx, 'realm-01');
  const realm01 = getRealmView(idx, 'realm-01');
  const actors = {
    rulerId: houses[0].headPersonId,
    rhcId: houses[0].memberIds[3],
    firstHeadId: houses[1].headPersonId,
    secondHeadId: houses[2].headPersonId,
    firstHeadHouseId: houses[1].id,
    secondHeadHouseId: houses[2].id,
    rulingHouseId: houses[0].id,
    directId: realm01.claims.find((c) => c.kind === 'direct').personId,
    restoredId: realm01.claims.find((c) => c.kind === 'restored').personId,
    restoredHouseId: realm01.claims.find((c) => c.kind === 'restored').houseId,
  };

  site = await startStaticServer(exportDir);

  const chromeInfo = resolveChromePath();
  if (!chromeInfo.path) throw new Error(chromeNotFoundMessage(chromeInfo.tried));

  userDataDir = await fsp.mkdtemp(path.join(os.tmpdir(), 'epoch-app-chrome-'));
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
      houses: document.querySelectorAll('.house-card').length,
      persons: document.querySelectorAll('.person-card').length,
      documentState: document.readyState,
      bodyLength: (document.body?.innerText || '').length,
    })`);
    if (readyState.tiles === 36 && readyState.houses === 3 && readyState.persons === 8) break;
    await sleep(200);
  }
  check(
    'page-ready',
    readyState?.tiles === 36 && readyState?.houses === 3 && readyState?.persons === 8,
    JSON.stringify(readyState),
  );

  const snapshot = await cdp.eval(`({
    tiles: document.querySelectorAll('.territory-tile').length,
    capitals: [...document.querySelectorAll('.territory-tile')].filter(el => el.textContent.includes('★')).length,
    realmCodes: [...new Set([...document.querySelectorAll('.territory-tile .tile-code')].map(el => el.textContent.trim()))],
    selectedRealm: document.querySelector('#realm-detail h3')?.textContent.trim(),
    selectedPerson: document.querySelector('#person-detail h3')?.textContent.trim(),
    pageText: document.body.innerText,
  })`);
  check('tile-count', snapshot.tiles === 36, String(snapshot.tiles));
  check('capital-marks', snapshot.capitals === 6, String(snapshot.capitals));
  check('realm-codes', snapshot.realmCodes.length === 6, snapshot.realmCodes.join(','));
  check('initial-realm', snapshot.selectedRealm === firstRealm.name, snapshot.selectedRealm);
  check('initial-person', Boolean(snapshot.selectedPerson), snapshot.selectedPerson);

  await cdp.eval(`document.querySelector('[data-territory-id="${otherCapital}"]')?.click()`);
  await sleep(150);
  const afterTerritory = await cdp.eval(`document.querySelector('#realm-detail h3')?.textContent.trim()`);
  check('territory-click-realm', afterTerritory === otherRealm.name, afterTerritory);

  await cdp.eval(`document.querySelector('[data-territory-id="${idx.realmById['realm-01'].capital_territory_id}"]')?.click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-house-id="${actors.rulingHouseId}"]')?.click()`);
  await sleep(150);
  const houseMembers = await cdp.eval(`document.querySelectorAll('.person-card').length`);
  check('house-members', houseMembers === 8, String(houseMembers));

  await cdp.eval(`document.querySelector('[data-person-id="${actors.directId}"]')?.click()`);
  await sleep(150);
  const directText = await cdp.eval(`document.querySelector('#person-detail')?.innerText || ''`);
  check('direct-claim', /강한 직계 권리/.test(directText) && /직계/.test(directText), directText.slice(0, 200));
  check('direct-not-heir', !/공식 후계자|다음 왕|후계 순위|확정된 승계자/.test(directText), 'overstated heir');

  await cdp.eval(`document.querySelector('[data-house-id="${actors.restoredHouseId}"]')?.click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-person-id="${actors.restoredId}"]')?.click()`);
  await sleep(150);
  const restoredText = await cdp.eval(`document.querySelector('#person-detail')?.innerText || ''`);
  check('restored-claim', /논쟁 중인 복권 권리/.test(restoredText), restoredText.slice(0, 200));

  await cdp.eval(`document.querySelector('[data-house-id="${actors.rulingHouseId}"]')?.click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-person-id="${actors.rulerId}"]')?.click()`);
  await sleep(150);
  const rulerText = await cdp.eval(`document.querySelector('#person-detail')?.innerText || ''`);
  check(
    'ruler-confirmed-conflict',
    /비공개 · 확인됨/.test(rulerText) && /같은 평의회 자리/.test(rulerText),
    rulerText.slice(0, 240),
  );

  await cdp.eval(`document.querySelector('[data-house-id="${actors.firstHeadHouseId}"]')?.click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-person-id="${actors.firstHeadId}"]')?.click()`);
  await sleep(150);
  const rumorText = await cdp.eval(`document.querySelector('#person-detail')?.innerText || ''`);
  check(
    'first-head-unverified',
    /비공개 · 미확인/.test(rumorText) && /소문/.test(rumorText),
    rumorText.slice(0, 240),
  );

  await cdp.eval(`document.querySelector('[data-house-id="${actors.secondHeadHouseId}"]')?.click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-person-id="${actors.secondHeadId}"]')?.click()`);
  await sleep(150);
  const secondText = await cdp.eval(`document.querySelector('#person-detail')?.innerText || ''`);
  check(
    'second-head-no-conflict',
    !/같은 평의회 자리를 두 가문/.test(secondText) && !/소문/.test(secondText) && !/숨겨진 정보/.test(secondText),
    secondText.slice(0, 240),
  );

  await cdp.eval(`document.querySelector('[data-house-id="${actors.rulingHouseId}"]')?.focus()`);
  await cdp.key('Enter', 'Enter', 13);
  await sleep(150);
  const afterHouseKey = await cdp.eval(
    `document.querySelector('.house-card.is-selected')?.getAttribute('data-house-id')`,
  );
  check('keyboard-house', afterHouseKey === actors.rulingHouseId, afterHouseKey);

  await cdp.eval(`document.querySelector('[data-person-id="${actors.rulerId}"]')?.focus()`);
  await cdp.key(' ', 'Space', 32);
  await sleep(150);
  const afterPersonKey = await cdp.eval(
    `document.querySelector('.person-card.is-selected')?.getAttribute('data-person-id')`,
  );
  check('keyboard-person', afterPersonKey === actors.rulerId, afterPersonKey);

  await cdp.eval(`document.querySelector('[data-territory-id="${otherCapital}"]')?.focus()`);
  await cdp.key('Enter', 'Enter', 13);
  await sleep(150);
  const afterTileKey = await cdp.eval(
    `document.querySelector('.territory-tile.is-selected')?.getAttribute('data-territory-id')`,
  );
  check('keyboard-territory', afterTileKey === otherCapital, afterTileKey);

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
      mapWidth: document.querySelector('.map-grid')?.getBoundingClientRect().width || 0,
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
          '.summary,.map-grid,.realm-detail,.house-card,.person-card,.person-detail,h1,h2'
        )];
        for (const el of core) {
          if (!visible(el)) continue;
          const r = el.getBoundingClientRect();
          if (r.right > vw + 1) issues.push('overflow-x:' + (el.className || el.id || el.tagName));
        }
        const cards = [...document.querySelectorAll('.house-card,.person-card')].filter(visible);
        for (let i = 0; i < cards.length; i++) {
          for (let j = i + 1; j < cards.length; j++) {
            if (cards[i].contains(cards[j]) || cards[j].contains(cards[i])) continue;
            const a = cards[i].getBoundingClientRect();
            const b = cards[j].getBoundingClientRect();
            const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
            const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
            if (ox > 2 && oy > 2) issues.push('overlap:' + i + '-' + j);
          }
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
  check('mobile-map-fits', mobile.mapWidth <= 390, String(mobile.mapWidth));
  check('mobile-layout', mobile.issues.length === 0, mobile.issues.join(','));

  const banned = [
    '아르케온',
    '후계 후보',
    '공식 후계자',
    '다음 왕',
    '후계 순위',
    '확정된 승계자',
    '플레이어 행동',
    '상충하는 제안',
  ];
  const pageText = await cdp.eval(`document.body.innerText`);
  for (const phrase of banned) {
    check(`no-m1-phrase:${phrase}`, !pageText.includes(phrase));
  }

  const errors = consoleErrors(cdp);
  check('console-errors', errors.length === 0, JSON.stringify(errors).slice(0, 400));

  if (failures.length) {
    throw new Error(`browser verify failed:\n- ${failures.join('\n- ')}`);
  }

  console.log('BROWSER_VERIFY_OK tiles=36 houses=3 persons=8 desktop=1280x720 mobile=390x664');
} catch (error) {
  console.error(error instanceof Error ? error.stack || error.message : error);
  process.exitCode = 1;
} finally {
  await cleanup();
}
