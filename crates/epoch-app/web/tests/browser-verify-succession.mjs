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
  getCrisisView,
  getInitialSelection,
  getRealmView,
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
  const crisis = getCrisisView(idx, 'realm-01');
  const realm = getRealmView(idx, 'realm-01');
  const initial = getInitialSelection(idx);
  const formerName = idx.personById[succession.transition.death.person_id].name;
  const priority = crisis.priority;
  const restored = crisis.competing.find((item) => item.priority === 'restored_contested_original');
  const derived = crisis.competing.find((item) => item.priority === 'restored_contested_derived');

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
      crisis: document.querySelectorAll('.crisis-candidate').length,
      documentState: document.readyState,
      hidden: document.getElementById('crisis-panel')?.hidden ?? true,
    })`);
    if (readyState.tiles === 36 && readyState.crisis === 3 && readyState.hidden === false) break;
    await sleep(200);
  }
  check(
    'page-ready',
    readyState?.tiles === 36 && readyState?.crisis === 3 && readyState?.hidden === false,
    JSON.stringify(readyState),
  );

  const snapshot = await cdp.eval(`({
    crisisHidden: document.getElementById('crisis-panel')?.hidden ?? true,
    vacancy: document.querySelector('[data-role="vacancy"]')?.textContent.trim(),
    former: document.querySelector('[data-role="former-incumbent"]')?.textContent.trim(),
    incumbentLine: document.querySelector('#realm-detail')?.innerText || '',
    candidates: [...document.querySelectorAll('.crisis-candidate')].map((el) => ({
      name: el.querySelector('h3')?.textContent.trim(),
      personId: el.getAttribute('data-person-id'),
      priority: el.getAttribute('data-candidate-priority'),
      origin: el.getAttribute('data-candidate-origin'),
      text: el.innerText,
    })),
    derivedSource: document.querySelector('[data-derived-source]')?.getAttribute('data-derived-source'),
    pageText: document.body.innerText,
  })`);

  check('vacancy-visible', snapshot.vacancy === '현재 상태: 통치자 공석', snapshot.vacancy);
  check(
    'former-matches-data',
    snapshot.former === `직전 통치자 ${formerName} — 사망`,
    snapshot.former,
  );
  check(
    'dead-not-current-ruler',
    !snapshot.incumbentLine.split('\n').some((line) => line === `통치자${formerName}` || line === `통치자 ${formerName}`)
      && snapshot.incumbentLine.includes('공석'),
    snapshot.incumbentLine.slice(0, 240),
  );
  check('candidate-count', snapshot.candidates.length === 3, String(snapshot.candidates.length));

  const shownPriority = snapshot.candidates.find((item) => item.priority === 'direct_strong_original');
  const shownRestored = snapshot.candidates.find((item) => item.priority === 'restored_contested_original');
  const shownDerived = snapshot.candidates.find((item) => item.priority === 'restored_contested_derived');
  check('direct-priority', shownPriority?.personId === priority.personId, JSON.stringify(shownPriority));
  check('direct-label', Boolean(shownPriority?.text.includes('강한 직계 권리')), shownPriority?.text);
  check('restored-shown', shownRestored?.personId === restored.personId, JSON.stringify(shownRestored));
  check(
    'restored-label',
    Boolean(shownRestored?.text.includes('논쟁 중인 복권 권리')),
    shownRestored?.text,
  );
  check('derived-shown', shownDerived?.personId === derived.personId, JSON.stringify(shownDerived));
  check(
    'derived-label',
    Boolean(shownDerived?.text.includes('혈통을 따라 파생된 복권 권리')),
    shownDerived?.text,
  );
  check(
    'derived-source',
    snapshot.derivedSource === derived.sourcePersonId,
    `${snapshot.derivedSource} != ${derived.sourcePersonId}`,
  );
  check('priority-name', shownPriority?.name === idx.personById[priority.personId].name, shownPriority?.name);
  check('restored-name', shownRestored?.name === idx.personById[restored.personId].name, shownRestored?.name);
  check('derived-name', shownDerived?.name === idx.personById[derived.personId].name, shownDerived?.name);

  await cdp.eval(`document.querySelector('[data-person-id="${priority.personId}"].crisis-candidate')?.click()`);
  await sleep(150);
  const afterClick = await cdp.eval(`({
    selected: document.querySelector('.person-card.is-selected')?.getAttribute('data-person-id'),
    detail: document.querySelector('#person-detail h3')?.textContent.trim(),
  })`);
  check('candidate-selectable', afterClick.selected === priority.personId, JSON.stringify(afterClick));
  check(
    'candidate-opens-person',
    afterClick.detail === idx.personById[priority.personId].name,
    afterClick.detail,
  );

  const banned = [
    '즉시 즉위',
    '승계 확정',
    '최종 당선',
    '왕위 정보',
    '공식 후계자',
    '다음 왕',
    '후계 순위',
    '확정된 승계자',
  ];
  for (const phrase of banned) {
    check(`no-overclaim:${phrase}`, !snapshot.pageText.includes(phrase));
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
          '.summary,.map-grid,.realm-detail,.house-card,.person-card,.person-detail,.crisis-panel,.crisis-candidate,h1,h2'
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
