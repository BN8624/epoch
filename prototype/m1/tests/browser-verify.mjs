// 정적 서버와 Chrome을 직접 띄워 레이아웃·상호작용·의미구조·콘솔을 검증한다 (의존성 없음)
import { spawn } from 'child_process';
import http from 'http';
import fsp from 'fs/promises';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { resolveChromePath, chromeNotFoundMessage } from './chrome-path.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SITE_ROOT = path.resolve(HERE, '..');

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
};

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

// ---------- 정적 서버 (OS가 할당한 포트 사용) ----------
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

// Chrome이 --remote-debugging-port=0으로 실제 할당한 포트를 읽는다
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

// ---------- 자원 ----------
let site = null;
let chrome = null;
let cdp = null;
let userDataDir = null;

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
}

const failures = [];

try {
  site = await startStaticServer(SITE_ROOT);

  const chromeInfo = resolveChromePath();
  if (!chromeInfo.path) throw new Error(chromeNotFoundMessage(chromeInfo.tried));

  userDataDir = await fsp.mkdtemp(path.join(os.tmpdir(), 'epoch-m1-chrome-'));
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

  // --- 페이지 준비 대기: 후보 3, 가문 5, 플레이어 토글 1, 제안 3, 행동 5 ---
  const readyDeadline = Date.now() + 20000;
  let readyState = null;
  while (Date.now() < readyDeadline) {
    readyState = await cdp.eval(`({
      candidates: document.querySelectorAll('.candidate-card').length,
      houses: document.querySelectorAll('.house-card').length,
      playerToggle: document.querySelectorAll('#player-toggle').length,
      proposals: document.querySelectorAll('.proposal-toggle').length,
      actions: document.querySelectorAll('.action-card').length,
      documentState: document.readyState,
      bodyLength: (document.body?.innerText || '').length,
    })`);
    if (
      readyState.candidates === 3 &&
      readyState.houses === 5 &&
      readyState.playerToggle === 1 &&
      readyState.proposals === 3 &&
      readyState.actions === 5
    ) {
      break;
    }
    await sleep(200);
  }
  if (
    !(
      readyState?.candidates === 3 &&
      readyState?.houses === 5 &&
      readyState?.playerToggle === 1 &&
      readyState?.proposals === 3 &&
      readyState?.actions === 5
    )
  ) {
    const consoleDump = cdp.console
      .filter((c) => c.type === 'error' || c.type === 'exception' || c.exceptionDetails)
      .map((c) => JSON.stringify(c))
      .join('\n');
    throw new Error(
      [
        'Page did not become ready in time.',
        `  url: ${site.url}`,
        `  candidate cards: ${readyState?.candidates ?? 'n/a'} (expected 3)`,
        `  house cards: ${readyState?.houses ?? 'n/a'} (expected 5)`,
        `  player toggle: ${readyState?.playerToggle ?? 'n/a'} (expected 1)`,
        `  proposals: ${readyState?.proposals ?? 'n/a'} (expected 3)`,
        `  actions: ${readyState?.actions ?? 'n/a'} (expected 5)`,
        `  document.readyState: ${readyState?.documentState ?? 'n/a'}`,
        `  page load failed: ${(readyState?.bodyLength ?? 0) === 0 ? 'yes' : 'no'}`,
        `  console errors:${consoleDump ? '\n' + consoleDump : ' none'}`,
      ].join('\n'),
    );
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
      candidateCount: document.querySelectorAll('.candidate-card').length,
      houseCount: document.querySelectorAll('.house-card').length,
      selectedCandidate: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
      selectedHouse: document.querySelector('.house-card.is-selected .card-name')?.textContent,
      titles: {
        ruler: document.querySelector('.crisis-facts dd')?.textContent,
        c1: document.querySelectorAll('.candidate-card .card-name')[0]?.textContent,
        c2: document.querySelectorAll('.candidate-card .card-name')[1]?.textContent,
        c3: document.querySelectorAll('.candidate-card .card-name')[2]?.textContent,
        player: document.querySelector('#player-panel .player-name')?.textContent,
      },
      hasPublicBadge: !!document.querySelector('.badge-public'),
      hasUnverifiedBadge: !!document.querySelector('.badge-unverified'),
      ariaNames: [...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => el.getAttribute('aria-label')).filter(Boolean).length,
      touchTargets: [...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => {
          const r = el.getBoundingClientRect();
          return { w: r.width, h: r.height, tag: el.className };
        }),
      minTouchW: Math.min(...[...document.querySelectorAll('.candidate-card,.house-card,#player-toggle,.proposal-toggle,.action-card,.btn')]
        .map(el => el.getBoundingClientRect().width).filter(w => w > 0)),
      minTouchH: Math.min(...[...document.querySelectorAll('.candidate-card,.house-card,#player-toggle,.proposal-toggle,.action-card,.btn')]
        .map(el => el.getBoundingClientRect().height).filter(h => h > 0)),
      layoutIssues: (() => {
        const issues = [];
        const vw = window.innerWidth;
        const core = [...document.querySelectorAll(
          '.candidate-card,.house-card,#player-toggle,#candidate-detail,#house-detail,.crisis-facts,h1,h2,.proposal-card,.action-card,#confirm-panel,#outcome-panel,.btn'
        )];
        for (const el of core) {
          const r = el.getBoundingClientRect();
          const style = getComputedStyle(el);
          if (r.width > 0 && r.right > vw + 1) issues.push('overflow-x:' + (el.className || el.id || el.tagName));
          if (style.overflow === 'hidden' || style.overflowX === 'hidden' || style.overflowY === 'hidden') {
            if (el.scrollWidth > el.clientWidth + 2) issues.push('clipped-x:' + (el.className || el.id || el.tagName));
            if (el.scrollHeight > el.clientHeight + 2 && el.clientHeight > 0 && style.overflowY === 'hidden') {
              issues.push('clipped-y:' + (el.className || el.id || el.tagName));
            }
          }
        }
        // 카드 간 겹침 검사
        const cards = [...document.querySelectorAll('.candidate-card,.house-card')];
        for (let i = 0; i < cards.length; i++) {
          for (let j = i + 1; j < cards.length; j++) {
            const a = cards[i].getBoundingClientRect();
            const b = cards[j].getBoundingClientRect();
            const overlapX = Math.min(a.right, b.right) - Math.max(a.left, b.left);
            const overlapY = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
            if (overlapX > 2 && overlapY > 2) {
              issues.push('overlap:' + i + '-' + j);
            }
          }
        }
        return issues;
      })(),
    })`);
  }

  // review·decision·resolved 각 상태에서 실제 레이아웃 값으로 넘침·잘림·겹침을 감사한다.
  // 범용 레이아웃 엔진이 아니라 M-1.2 화면의 지정 요소만 검사한다.
  async function auditLayout(w, h, phase) {
    await cdp.send('Emulation.setDeviceMetricsOverride', {
      width: w,
      height: h,
      deviceScaleFactor: 1,
      mobile: w < 500,
    });
    await sleep(350);
    return cdp.eval(`(() => {
      const vw = window.innerWidth;
      const label = (el) => el.id
        ? '#' + el.id
        : (el.className ? '.' + String(el.className).trim().split(/\\s+/).join('.') : el.tagName);
      // 숨겨진 요소는 겹침·넘침 검사 대상에서 제외한다
      const visible = (el) => {
        if (!el || el.hidden || el.closest('[hidden]')) return false;
        const s = getComputedStyle(el);
        if (s.display === 'none' || s.visibility === 'hidden' || Number(s.opacity) === 0) return false;
        const r = el.getBoundingClientRect();
        return r.width > 0 && r.height > 0;
      };
      const rectOf = (el) => el.getBoundingClientRect();
      const overlaps = (a, b) => {
        const ra = rectOf(a);
        const rb = rectOf(b);
        const ox = Math.min(ra.right, rb.right) - Math.max(ra.left, rb.left);
        const oy = Math.min(ra.bottom, rb.bottom) - Math.max(ra.top, rb.top);
        return ox > 2 && oy > 2;
      };
      const issues = [];

      // 1) 뷰포트 이탈과 의도치 않은 잘림
      const TARGETS = [
        '#confirm-panel', '#outcome-panel', '#btn-confirm', '#btn-cancel', '#btn-retry',
        '.proposal-card', '.action-card', '.confirm-actions', '.outcome-actions',
      ];
      const targets = [];
      for (const sel of TARGETS) {
        const all = [...document.querySelectorAll(sel)];
        const vis = all.filter(visible);
        targets.push({ selector: sel, total: all.length, visible: vis.length });
        for (const el of vis) {
          const r = rectOf(el);
          const s = getComputedStyle(el);
          const hidesX = s.overflowX === 'hidden' || s.overflow === 'hidden';
          const hidesY = s.overflowY === 'hidden' || s.overflow === 'hidden';
          if (r.right > vw + 1) issues.push('offscreen-right:' + sel);
          if (r.left < -1) issues.push('offscreen-left:' + sel);
          if (hidesX && el.scrollWidth > el.clientWidth + 2) issues.push('clipped-x:' + sel);
          if (hidesY && el.clientHeight > 0 && el.scrollHeight > el.clientHeight + 2) {
            issues.push('clipped-y:' + sel);
          }
        }
      }

      // 2) 그룹별 겹침 (부모-자식 포함 관계는 겹침으로 보지 않는다)
      const groups = {};
      const checkGroup = (name, els) => {
        const vis = els.filter(visible);
        let pairs = 0;
        for (let i = 0; i < vis.length; i++) {
          for (let j = i + 1; j < vis.length; j++) {
            if (vis[i].contains(vis[j]) || vis[j].contains(vis[i])) continue;
            pairs++;
            if (overlaps(vis[i], vis[j])) {
              issues.push('overlap:' + name + ':' + label(vis[i]) + '|' + label(vis[j]));
            }
          }
        }
        groups[name] = { visible: vis.length, pairs };
        return vis;
      };

      checkGroup('action-cards', [...document.querySelectorAll('.action-card')]);
      checkGroup('proposal-cards', [...document.querySelectorAll('.proposal-card')]);
      checkGroup('confirm-actions', [...document.querySelectorAll('.confirm-actions .btn')]);
      checkGroup('outcome-actions', [...document.querySelectorAll('.outcome-actions .btn')]);

      // 3) 확인 버튼과 돌아가기 버튼
      const btnConfirm = document.querySelector('#btn-confirm');
      const btnCancel = document.querySelector('#btn-cancel');
      const confirmPairVisible = visible(btnConfirm) && visible(btnCancel);
      if (confirmPairVisible && overlaps(btnConfirm, btnCancel)) {
        issues.push('overlap:confirm-cancel');
      }
      groups['confirm-cancel'] = { visible: confirmPairVisible ? 2 : 0, pairs: confirmPairVisible ? 1 : 0 };

      // 4) 결과 패널의 재시도 버튼과 결과 내용 구역
      const btnRetry = document.querySelector('#btn-retry');
      let retryPairs = 0;
      if (visible(btnRetry)) {
        const blocks = [...document.querySelectorAll('#outcome-panel .outcome-block')];
        for (const block of blocks) {
          if (!visible(block) || block.contains(btnRetry) || btnRetry.contains(block)) continue;
          retryPairs++;
          if (overlaps(btnRetry, block)) issues.push('overlap:retry|' + label(block));
        }
      }
      groups['retry-vs-outcome'] = { visible: visible(btnRetry) ? 1 : 0, pairs: retryPairs };

      // 5) 같은 컨테이너 안의 형제 버튼
      const parents = new Set();
      for (const btn of document.querySelectorAll('button')) {
        if (btn.parentElement) parents.add(btn.parentElement);
      }
      let siblingPairs = 0;
      let siblingButtons = 0;
      for (const parent of parents) {
        const btns = [...parent.children].filter((el) => el.tagName === 'BUTTON').filter(visible);
        siblingButtons += btns.length;
        for (let i = 0; i < btns.length; i++) {
          for (let j = i + 1; j < btns.length; j++) {
            siblingPairs++;
            if (overlaps(btns[i], btns[j])) {
              issues.push('overlap:sibling-buttons:' + label(btns[i]) + '|' + label(btns[j]));
            }
          }
        }
      }
      groups['sibling-buttons'] = { visible: siblingButtons, pairs: siblingPairs };

      return {
        phase: ${JSON.stringify(phase)},
        viewport: vw + 'x' + window.innerHeight,
        hasHScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
        confirmVisible: visible(document.querySelector('#confirm-panel')),
        outcomeVisible: visible(document.querySelector('#outcome-panel')),
        targets,
        groups,
        issues,
      };
    })()`);
  }

  // 결과·확인 단계에서 초기 선택 화면(review)으로 되돌린다
  async function resetToReview() {
    await cdp.eval(`(() => {
      const outcomeOpen = !document.querySelector('#outcome-section')?.hidden;
      if (outcomeOpen) {
        document.querySelector('#btn-retry')?.click();
        return 'retry';
      }
      const confirmOpen = !document.querySelector('#confirm-panel')?.hidden;
      if (confirmOpen) {
        document.querySelector('#btn-cancel')?.click();
        return 'cancel';
      }
      return 'already-review';
    })()`);
    await sleep(300);
  }

  const desktop = await measure(1280, 720);

  // --- 버튼 의미 구조 검증 ---
  const semantics = await cdp.eval(`(() => {
    const FORBIDDEN = ['DIV','P','H1','H2','H3','H4','H5','H6','SECTION','ARTICLE','BUTTON','A','INPUT','SELECT','TEXTAREA','UL','OL','LI','DL','DT','DD'];
    const inspect = (selector, kind) => [...document.querySelectorAll(selector)].map((el, index) => ({
      kind,
      index,
      tag: el.tagName,
      forbidden: [...new Set([...el.querySelectorAll('*')].map(n => n.tagName).filter(t => FORBIDDEN.includes(t)))],
    }));
    return [
      ...inspect('.candidate-card', 'candidate'),
      ...inspect('.house-card', 'house'),
      ...inspect('#player-toggle', 'player'),
      ...inspect('.proposal-toggle', 'proposal'),
      ...inspect('.action-card', 'action'),
    ];
  })()`);

  // --- 후보 A·B·C 명시 선택 및 상세 검증 ---
  const candidateExpect = [
    { index: 0, name: '세리아 아르케온' },
    { index: 1, name: '다리안 코르벤' },
    { index: 2, name: '미레아 셀칸' },
  ];
  const candidateClicks = [];
  for (const exp of candidateExpect) {
    await cdp.eval(`document.querySelectorAll('.candidate-card')[${exp.index}].click()`);
    await sleep(200);
    candidateClicks.push(
      await cdp.eval(`({
        selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
        selectedCount: document.querySelectorAll('.candidate-card.is-selected').length,
        detailTitle: document.querySelector('#candidate-detail-title')?.textContent,
        hasClaim: !!document.querySelector('#candidate-detail .detail-block'),
        infoCount: document.querySelectorAll('#candidate-detail .info-list li').length,
      })`),
    );
  }

  // --- 다섯 가문 각각 선택·상세·이유 검증 ---
  const houseExpect = [
    { id: 'house-arden', name: '아르덴 가문', stanceHas: '다리안' },
    { id: 'house-barren', name: '바렌 가문', stanceHas: '세리아' },
    { id: 'house-soren', name: '소렌 가문', stanceHas: '다리안' },
    { id: 'house-merova', name: '메로바 가문', stanceHas: '미레아' },
    { id: 'house-halbeck', name: '할베크 가문', stanceHas: '미결정' },
  ];
  const houseClicks = [];
  for (const exp of houseExpect) {
    await cdp.eval(`document.querySelector('[data-house-id="${exp.id}"]').click()`);
    await sleep(150);
    houseClicks.push(
      await cdp.eval(`({
        id: '${exp.id}',
        selected: document.querySelector('.house-card.is-selected .card-name')?.textContent,
        selectedCount: document.querySelectorAll('.house-card.is-selected').length,
        title: document.querySelector('#house-detail-title')?.textContent,
        stance: document.querySelector('.house-stance-large')?.textContent?.replace(/\\s+/g,' ').trim(),
        positiveCount: document.querySelectorAll('.reasons-positive li').length,
        negativeCount: document.querySelectorAll('.reasons-negative li').length,
      })`),
    );
  }

  await cdp.eval(`document.querySelector('#player-toggle').click()`);
  await sleep(250);
  const afterPlayer = await cdp.eval(`({
    expanded: document.querySelector('#player-toggle')?.getAttribute('aria-expanded'),
    relations: document.querySelectorAll('.relation-list li').length,
    pressures: document.querySelectorAll('.pressure-list li').length,
    privateBadges: document.querySelectorAll('.badge-private').length,
  })`);

  // --- 키보드: 후보 탐색 ---
  await cdp.eval(`document.querySelectorAll('.candidate-card')[0].click()`);
  await sleep(150);
  await cdp.eval(`document.querySelectorAll('.candidate-card')[0].focus()`);
  await cdp.key('ArrowRight', 'ArrowRight', 39);
  await sleep(250);
  const afterCandidateKey = await cdp.eval(`({
    selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
    detailTitle: document.querySelector('#candidate-detail-title')?.textContent,
    activeClass: document.activeElement?.className || '',
    activeIsCandidate: document.activeElement?.classList?.contains('candidate-card') || false,
  })`);

  await cdp.key('ArrowRight', 'ArrowRight', 39);
  await sleep(200);
  const afterCandidateKey2 = await cdp.eval(`({
    selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
    detailTitle: document.querySelector('#candidate-detail-title')?.textContent,
    activeIsCandidate: document.activeElement?.classList?.contains('candidate-card') || false,
  })`);

  // --- 키보드: 가문 탐색 ---
  await cdp.eval(`document.querySelector('[data-house-id="house-arden"]').click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('[data-house-id="house-arden"]').focus()`);
  await cdp.key('ArrowRight', 'ArrowRight', 39);
  await sleep(250);
  const afterHouseKey = await cdp.eval(`({
    selected: document.querySelector('.house-card.is-selected .card-name')?.textContent,
    title: document.querySelector('#house-detail-title')?.textContent,
    activeIsHouse: document.activeElement?.classList?.contains('house-card') || false,
    positiveCount: document.querySelectorAll('.reasons-positive li').length,
  })`);

  await cdp.key('ArrowRight', 'ArrowRight', 39);
  await sleep(200);
  const afterHouseKey2 = await cdp.eval(`({
    selected: document.querySelector('.house-card.is-selected .card-name')?.textContent,
    title: document.querySelector('#house-detail-title')?.textContent,
    activeIsHouse: document.activeElement?.classList?.contains('house-card') || false,
  })`);

  // 행동별 확인 패널 표시명 (선택 직후 일치 검증)
  const ACTION_CONFIRM = {
    'action-a': '다리안을 공개 지지하고 직위 약속을 공표한다',
    'action-b': '세리아와 비밀 혼인 동맹을 맺고 지지를 약속한다',
    'action-c': '미레아에게 알레시아 계통의 권리 기록 사본을 제공한다',
    'action-d': '다리안이 같은 핵심 직위를 중복 약속했다는 정보를 세리아 측에 넘긴다',
    'action-e': '세 진영의 요구를 모두 거절하고 결정을 미룬다',
  };

  async function readConfirmPanel() {
    return cdp.eval(`({
      confirmVisible: !document.querySelector('#confirm-panel')?.hidden,
      chosenText: document.querySelector('#confirm-panel .confirm-chosen')?.textContent || '',
      hasConfirm: !!document.querySelector('#btn-confirm'),
      hasCancel: !!document.querySelector('#btn-cancel'),
      phaseDecision: !document.querySelector('#confirm-panel')?.hidden &&
        !!document.querySelector('#btn-confirm'),
    })`);
  }

  // --- M-1.2: 제안·행동·확인·결과 핵심 흐름 ---
  const proposalExpand = await cdp.eval(`(() => {
    const btn = document.querySelectorAll('.proposal-toggle')[0];
    btn?.click();
    return {
      count: document.querySelectorAll('.proposal-toggle').length,
      expanded: document.querySelectorAll('.proposal-card.is-expanded').length,
      bodyVisible: !!document.querySelector('.proposal-card.is-expanded .proposal-body:not([hidden])'),
      proposer: document.querySelector('.proposal-card.is-expanded .proposal-proposer')?.textContent,
    };
  })()`);
  await sleep(150);

  // 행동 A 선택 → 확인 패널 (decision 상태)
  await cdp.eval(`document.querySelector('[data-action-id="action-a"]').click()`);
  await sleep(200);
  const confirmA = await readConfirmPanel();
  // 데스크톱 decision 레이아웃: 확인 패널·버튼 가로 넘침·겹침
  const desktopDecision = await measure(1280, 720);

  // 돌아가기 → 선택 화면 복귀
  await cdp.eval(`document.querySelector('#btn-cancel')?.click()`);
  await sleep(200);
  const afterCancel = await cdp.eval(`({
    confirmHidden: !!document.querySelector('#confirm-panel')?.hidden,
    outcomeHidden: !!document.querySelector('#outcome-section')?.hidden,
    selectedActions: document.querySelectorAll('.action-card.is-selected').length,
    focusAction: document.activeElement?.classList?.contains('action-card') || false,
    actionEnabled: !document.querySelector('[data-action-id="action-a"]')?.disabled,
    reviewReady: !!document.querySelector('#confirm-panel')?.hidden &&
      !!document.querySelector('#outcome-section')?.hidden,
  })`);

  // 행동 A 확정 → resolved
  await cdp.eval(`document.querySelector('[data-action-id="action-a"]').click()`);
  await sleep(150);
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const resultA = await cdp.eval(`({
    outcomeVisible: !document.querySelector('#outcome-section')?.hidden,
    heading: document.querySelector('#outcome-heading')?.textContent,
    body: document.querySelector('#outcome-panel')?.innerText || '',
    sorenStance: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    focusIsOutcome: document.activeElement?.id === 'outcome-panel' || document.activeElement?.closest?.('#outcome-panel') != null,
    activeId: document.activeElement?.id || '',
  })`);
  // 데스크톱 resolved 레이아웃: 결과 패널·버튼
  const desktopResolved = await measure(1280, 720);

  // 다른 선택 시도 후 행동 B
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(250);
  const afterRetry = await cdp.eval(`({
    outcomeHidden: !!document.querySelector('#outcome-section')?.hidden,
    sorenStance: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    actionEnabled: !document.querySelector('[data-action-id="action-a"]')?.disabled,
    focusAction: document.activeElement?.classList?.contains('action-card') || false,
  })`);

  await cdp.eval(`document.querySelector('[data-action-id="action-b"]').click()`);
  await sleep(150);
  const confirmB = await readConfirmPanel();
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const resultB = await cdp.eval(`({
    body: document.querySelector('#outcome-panel')?.innerText || '',
    sorenStance: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    halbStance: document.querySelector('[data-house-id="house-halbeck"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    hasAWaver: (document.querySelector('#outcome-panel')?.innerText || '').includes('동요') &&
      (document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent || '').includes('동요'),
  })`);

  // 행동 C
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(200);
  await cdp.eval(`document.querySelector('[data-action-id="action-c"]').click()`);
  await sleep(150);
  const confirmC = await readConfirmPanel();
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const resultC = await cdp.eval(`({
    body: document.querySelector('#outcome-panel')?.innerText || '',
    mireyaClaim: document.querySelectorAll('.candidate-card')[2]?.querySelector('.card-claim')?.textContent || '',
    halbStance: document.querySelector('[data-house-id="house-halbeck"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
  })`);

  // 행동 D
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(200);
  await cdp.eval(`document.querySelector('[data-action-id="action-d"]').click()`);
  await sleep(150);
  const confirmD = await readConfirmPanel();
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const resultD = await cdp.eval(`({
    body: document.querySelector('#outcome-panel')?.innerText || '',
    sorenStance: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    darianSupport: document.querySelectorAll('.candidate-card')[1]?.querySelector('.card-support')?.textContent || '',
  })`);

  // 행동 E
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(200);
  await cdp.eval(`document.querySelector('[data-action-id="action-e"]').click()`);
  await sleep(150);
  const confirmE = await readConfirmPanel();
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const resultE = await cdp.eval(`({
    body: document.querySelector('#outcome-panel')?.innerText || '',
    stances: {
      arden: document.querySelector('[data-house-id="house-arden"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
      barren: document.querySelector('[data-house-id="house-barren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
      soren: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
      merova: document.querySelector('[data-house-id="house-merova"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
      halb: document.querySelector('[data-house-id="house-halbeck"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
    },
  })`);

  // 최종 재시작
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(250);
  const finalReset = await cdp.eval(`({
    outcomeHidden: !!document.querySelector('#outcome-section')?.hidden,
    actions: document.querySelectorAll('.action-card:not(:disabled)').length,
    sorenStance: document.querySelector('[data-house-id="house-soren"] .house-stance')?.textContent?.replace(/\\s+/g,' ').trim(),
  })`);

  // 모바일 review → decision → resolved 레이아웃
  const mobile = await measure(390, 664);
  await cdp.eval(`document.querySelector('[data-action-id="action-a"]').click()`);
  await sleep(200);
  const mobileDecision = await measure(390, 664);
  await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
  await sleep(300);
  const mobileResolved = await measure(390, 664);
  // 검증 후 재시작해 초기 상태로 정리
  await cdp.eval(`document.querySelector('#btn-retry')?.click()`);
  await sleep(200);

  // --- 레이아웃 감사: review·decision·resolved × 1280x720·390x664 ---
  const layoutAudits = [];
  for (const [vpW, vpH, vpTag] of [
    [1280, 720, 'desktop'],
    [390, 664, 'mobile'],
  ]) {
    await resetToReview();
    layoutAudits.push(await auditLayout(vpW, vpH, `${vpTag} review`));

    await cdp.eval(`document.querySelector('[data-action-id="action-a"]').click()`);
    await sleep(250);
    layoutAudits.push(await auditLayout(vpW, vpH, `${vpTag} decision`));

    await cdp.eval(`document.querySelector('#btn-confirm')?.click()`);
    await sleep(350);
    layoutAudits.push(await auditLayout(vpW, vpH, `${vpTag} resolved`));

    await resetToReview();
  }

  const consoleErrors = cdp.console.filter(
    (c) => c.type === 'error' || c.type === 'exception' || c.exceptionDetails,
  );

  const result = {
    chrome: { path: chromeInfo.path, source: chromeInfo.source },
    server: { url: site.url, port: site.port },
    semantics,
    desktop,
    desktopDecision,
    desktopResolved,
    candidateClicks,
    houseClicks,
    afterPlayer,
    afterCandidateKey,
    afterCandidateKey2,
    afterHouseKey,
    afterHouseKey2,
    proposalExpand,
    confirmA,
    confirmB,
    confirmC,
    confirmD,
    confirmE,
    afterCancel,
    resultA,
    afterRetry,
    resultB,
    resultC,
    resultD,
    resultE,
    finalReset,
    mobile,
    mobileDecision,
    mobileResolved,
    layoutAudits,
    consoleLogCount: cdp.console.length,
    consoleErrors,
  };

  console.log(JSON.stringify(result, null, 2));

  if (desktop.hasHScroll) failures.push('desktop horizontal scroll');
  if (mobile.hasHScroll) failures.push('mobile horizontal scroll');
  if (desktop.candidateCount !== 3) failures.push('candidate count');
  if (desktop.houseCount !== 5) failures.push('house count');
  if (desktop.titles?.c1 !== '세리아 아르케온') failures.push('candidate A name');
  if (desktop.titles?.c2 !== '다리안 코르벤') failures.push('candidate B name');
  if (desktop.titles?.c3 !== '미레아 셀칸') failures.push('candidate C name');
  if (desktop.titles?.player !== '렌 아르덴') failures.push('player name');

  // 버튼 의미 구조
  if (semantics.filter((s) => s.kind === 'candidate').length !== 3) failures.push('semantics candidate count');
  if (semantics.filter((s) => s.kind === 'house').length !== 5) failures.push('semantics house count');
  if (semantics.filter((s) => s.kind === 'player').length !== 1) failures.push('semantics player count');
  if (semantics.filter((s) => s.kind === 'proposal').length !== 3) failures.push('semantics proposal count');
  if (semantics.filter((s) => s.kind === 'action').length !== 5) failures.push('semantics action count');
  for (const s of semantics) {
    if (s.tag !== 'BUTTON') failures.push(`${s.kind}[${s.index}] is ${s.tag}, expected BUTTON`);
    if (s.forbidden.length) {
      failures.push(`${s.kind}[${s.index}] contains forbidden element(s): ${s.forbidden.join(',')}`);
    }
  }

  // M-1.2 제안·선택·결과
  if (proposalExpand?.count !== 3) failures.push('proposal count');
  if (!(proposalExpand?.expanded >= 1)) failures.push('proposal expand');
  if (!proposalExpand?.bodyVisible) failures.push('proposal body');
  if (!confirmA?.confirmVisible) failures.push('confirm panel after select A');
  if (!confirmA?.chosenText?.includes(ACTION_CONFIRM['action-a'])) {
    failures.push('confirm shows action A label');
  }
  if (!confirmA?.hasConfirm || !confirmA?.hasCancel) failures.push('confirm buttons');
  // 행동 B~E: 선택 직후 확인 패널이 해당 행동명으로 갱신되는지 검증
  for (const [id, label, panel] of [
    ['action-b', ACTION_CONFIRM['action-b'], confirmB],
    ['action-c', ACTION_CONFIRM['action-c'], confirmC],
    ['action-d', ACTION_CONFIRM['action-d'], confirmD],
    ['action-e', ACTION_CONFIRM['action-e'], confirmE],
  ]) {
    if (!panel?.confirmVisible) failures.push(`confirm panel after select ${id}`);
    if (!panel?.chosenText?.includes(label)) {
      failures.push(`confirm shows ${id} label (got ${panel?.chosenText})`);
    }
    if (!panel?.hasConfirm || !panel?.hasCancel) failures.push(`confirm buttons for ${id}`);
  }
  if (!afterCancel?.confirmHidden) failures.push('cancel hides confirm');
  if (!afterCancel?.outcomeHidden) failures.push('cancel keeps outcome hidden');
  if (afterCancel?.selectedActions !== 0) {
    failures.push(`cancel clears selection (selectedActions=${afterCancel?.selectedActions})`);
  }
  if (!afterCancel?.reviewReady) failures.push('cancel returns to selection screen');
  if (!afterCancel?.focusAction) failures.push('cancel restores focus to action card');
  if (!afterCancel?.actionEnabled) failures.push('cancel keeps actions enabled');
  if (!resultA?.outcomeVisible) failures.push('outcome after A');
  if (!resultA?.body?.includes('다리안 공개 지지')) failures.push('outcome A stance');
  if (!resultA?.sorenStance?.includes('동요')) failures.push(`soren wavering after A got ${resultA?.sorenStance}`);
  if (!resultA?.focusIsOutcome && resultA?.activeId !== 'outcome-panel') {
    failures.push(`focus after outcome A (active=${resultA?.activeId})`);
  }
  if (!afterRetry?.outcomeHidden) failures.push('retry hides outcome');
  if (!afterRetry?.sorenStance?.includes('다리안')) failures.push(`retry restores soren got ${afterRetry?.sorenStance}`);
  if (!afterRetry?.actionEnabled) failures.push('actions re-enabled after retry');
  if (!afterRetry?.focusAction) failures.push('focus action after retry');
  if (!resultB?.halbStance?.includes('세리아')) failures.push(`halbeck lean seria after B got ${resultB?.halbStance}`);
  if (resultB?.sorenStance?.includes('동요')) failures.push('A wavering leaked into B');
  if (!resultB?.body?.includes('세리아 비밀 지지')) failures.push('outcome B stance');
  if (!resultC?.mireyaClaim?.includes('기록 증거를 확보한 오래된 왕통')) {
    failures.push(`mireya claim after C got ${resultC?.mireyaClaim}`);
  }
  if (!resultC?.halbStance?.includes('미레아')) failures.push(`halbeck lean mireya after C got ${resultC?.halbStance}`);
  if (!resultD?.sorenStance?.includes('미결정')) failures.push(`soren undecided after D got ${resultD?.sorenStance}`);
  if (!resultD?.darianSupport?.includes('1개 가문')) {
    failures.push(`darian support count after D got ${resultD?.darianSupport}`);
  }
  if (!resultE?.stances?.arden?.includes('다리안')) failures.push('E arden stance');
  if (!resultE?.stances?.barren?.includes('세리아')) failures.push('E barren stance');
  if (!resultE?.stances?.soren?.includes('다리안')) failures.push('E soren stance');
  if (!resultE?.stances?.merova?.includes('미레아')) failures.push('E merova stance');
  if (!resultE?.stances?.halb?.includes('미결정')) failures.push('E halb stance');
  if (!resultE?.body?.includes('중립')) failures.push('outcome E stance');
  if (!finalReset?.outcomeHidden) failures.push('final reset hides outcome');
  if (finalReset?.actions !== 5) failures.push('final reset actions enabled');
  if (!finalReset?.sorenStance?.includes('다리안')) failures.push('final reset soren');

  // decision/resolved 상태 레이아웃 (데스크톱·모바일)
  for (const [name, snap] of [
    ['desktop decision', desktopDecision],
    ['desktop resolved', desktopResolved],
    ['mobile decision', mobileDecision],
    ['mobile resolved', mobileResolved],
  ]) {
    if (snap?.hasHScroll) failures.push(`${name} horizontal scroll`);
    if (snap?.layoutIssues?.length) {
      failures.push(`${name} layout: ${snap.layoutIssues.join(';')}`);
    }
  }

  // review·decision·resolved × 데스크톱·모바일 겹침·넘침·클리핑 감사
  const AUDIT_EXPECT = [
    'desktop review', 'desktop decision', 'desktop resolved',
    'mobile review', 'mobile decision', 'mobile resolved',
  ];
  if (layoutAudits.length !== AUDIT_EXPECT.length) {
    failures.push(`layout audit count ${layoutAudits.length}, expected ${AUDIT_EXPECT.length}`);
  }
  for (const phase of AUDIT_EXPECT) {
    const audit = layoutAudits.find((a) => a.phase === phase);
    if (!audit) {
      failures.push(`layout audit missing: ${phase}`);
      continue;
    }
    if (audit.hasHScroll) failures.push(`${phase} horizontal scroll`);
    if (audit.issues.length) failures.push(`${phase} layout: ${audit.issues.join(';')}`);

    const visibleOf = (selector) =>
      audit.targets.find((t) => t.selector === selector)?.visible ?? 0;

    // 감사가 빈 화면을 통과시키지 않도록 각 상태의 필수 요소 표시를 확인한다
    if (visibleOf('.proposal-card') !== 3) {
      failures.push(`${phase} proposal cards visible ${visibleOf('.proposal-card')}, expected 3`);
    }
    if (visibleOf('.action-card') !== 5) {
      failures.push(`${phase} action cards visible ${visibleOf('.action-card')}, expected 5`);
    }
    if (phase.endsWith('review')) {
      if (audit.confirmVisible) failures.push(`${phase} confirm panel should be hidden`);
      if (audit.outcomeVisible) failures.push(`${phase} outcome panel should be hidden`);
    }
    if (phase.endsWith('decision')) {
      if (!audit.confirmVisible) failures.push(`${phase} confirm panel not visible`);
      if (audit.outcomeVisible) failures.push(`${phase} outcome panel should be hidden`);
      if (audit.groups['confirm-cancel']?.visible !== 2) {
        failures.push(`${phase} confirm/cancel buttons not both visible`);
      }
      if (visibleOf('#btn-confirm') !== 1 || visibleOf('#btn-cancel') !== 1) {
        failures.push(`${phase} confirm actions missing`);
      }
    }
    if (phase.endsWith('resolved')) {
      if (!audit.outcomeVisible) failures.push(`${phase} outcome panel not visible`);
      if (visibleOf('#btn-retry') !== 1) failures.push(`${phase} retry button not visible`);
      if (!(audit.groups['retry-vs-outcome']?.pairs >= 5)) {
        failures.push(
          `${phase} retry/outcome block pairs ${audit.groups['retry-vs-outcome']?.pairs}, expected >= 5`,
        );
      }
    }
  }
  for (let i = 0; i < candidateExpect.length; i++) {
    const exp = candidateExpect[i];
    const got = candidateClicks[i];
    if (got?.selected !== exp.name) failures.push(`select candidate ${exp.name}`);
    if (got?.detailTitle !== exp.name) failures.push(`detail title ${exp.name}`);
    if (got?.selectedCount !== 1) failures.push(`candidate single selection ${exp.name}`);
    if (!got?.hasClaim) failures.push(`candidate claim missing ${exp.name}`);
    if (!(got?.infoCount >= 1)) failures.push(`candidate info missing ${exp.name}`);
  }

  for (let i = 0; i < houseExpect.length; i++) {
    const exp = houseExpect[i];
    const got = houseClicks[i];
    if (got?.selected !== exp.name) failures.push(`select house ${exp.name}`);
    if (got?.title !== exp.name) failures.push(`house title ${exp.name}`);
    if (got?.selectedCount !== 1) failures.push(`house single selection ${exp.name}`);
    if (!got?.stance?.includes(exp.stanceHas)) {
      failures.push(`house stance ${exp.name} expected ${exp.stanceHas} got ${got?.stance}`);
    }
    if (!(got?.positiveCount >= 1)) failures.push(`house positive reasons ${exp.name}`);
    if (!(got?.negativeCount >= 1)) failures.push(`house negative reasons ${exp.name}`);
  }

  if (afterPlayer.relations !== 3) failures.push('player relations');
  if (afterPlayer.pressures !== 3) failures.push('player pressures');
  if (afterPlayer.expanded !== 'true') failures.push('player expand');

  // 키보드 후보: A → ArrowRight → B
  if (afterCandidateKey.selected !== '다리안 코르벤') {
    failures.push(`keyboard candidate select B got ${afterCandidateKey.selected}`);
  }
  if (afterCandidateKey.detailTitle !== '다리안 코르벤') {
    failures.push('keyboard candidate detail B');
  }
  if (!afterCandidateKey.activeIsCandidate) {
    failures.push('keyboard candidate focus after arrow');
  }
  if (afterCandidateKey2.selected !== '미레아 셀칸') {
    failures.push(`keyboard candidate select C got ${afterCandidateKey2.selected}`);
  }
  if (afterCandidateKey2.detailTitle !== '미레아 셀칸') {
    failures.push('keyboard candidate detail C');
  }

  // 키보드 가문: 아르덴 → ArrowRight → 바렌 → ArrowRight → 소렌
  if (afterHouseKey.selected !== '바렌 가문') {
    failures.push(`keyboard house select barren got ${afterHouseKey.selected}`);
  }
  if (afterHouseKey.title !== '바렌 가문') {
    failures.push('keyboard house detail barren');
  }
  if (!afterHouseKey.activeIsHouse) {
    failures.push('keyboard house focus after arrow');
  }
  if (!(afterHouseKey.positiveCount >= 1)) {
    failures.push('keyboard house reasons updated');
  }
  if (afterHouseKey2.selected !== '소렌 가문') {
    failures.push(`keyboard house select soren got ${afterHouseKey2.selected}`);
  }
  if (afterHouseKey2.title !== '소렌 가문') {
    failures.push('keyboard house detail soren');
  }

  if (consoleErrors.length) failures.push('console errors');

  // 터치 대상: 데스크톱·모바일 모두 최소 44px (너비·높이)
  if (desktop.minTouchH < 44) failures.push(`desktop touch height ${desktop.minTouchH}`);
  if (desktop.minTouchW < 44) failures.push(`desktop touch width ${desktop.minTouchW}`);
  if (mobile.minTouchH < 44) failures.push(`mobile touch height ${mobile.minTouchH}`);
  if (mobile.minTouchW < 44) failures.push(`mobile touch width ${mobile.minTouchW}`);

  if (desktop.layoutIssues?.length) {
    failures.push(`desktop layout: ${desktop.layoutIssues.join(';')}`);
  }
  if (mobile.layoutIssues?.length) {
    failures.push(`mobile layout: ${mobile.layoutIssues.join(';')}`);
  }

  if (desktop.ariaNames < 8) failures.push('missing aria names');
  if (mobile.ariaNames < 8) failures.push('mobile missing aria names');
} catch (err) {
  failures.push(`execution error: ${err?.message ?? err}`);
  console.error(err);
} finally {
  await cleanup();
}

if (failures.length) {
  console.error('FAILURES:', failures.join(', '));
  process.exit(1);
}
console.log('BROWSER_VERIFY_OK');
process.exit(0);
