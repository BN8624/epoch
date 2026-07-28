// 헤드리스 Chrome CDP로 레이아웃·상호작용·콘솔 검증 (의존성 없음)
import { spawn } from 'child_process';
import http from 'http';

const CHROME = 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe';
const PORT = 9223;
const URL = 'http://127.0.0.1:8765/';

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

function getJson(path) {
  return new Promise((resolve, reject) => {
    http
      .get(`http://127.0.0.1:${PORT}${path}`, (res) => {
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
    this.ws.close();
  }
}

const chrome = spawn(
  CHROME,
  [
    `--remote-debugging-port=${PORT}`,
    '--headless=new',
    '--disable-gpu',
    '--no-first-run',
    '--no-default-browser-check',
    '--window-size=1280,720',
    'about:blank',
  ],
  { stdio: 'ignore' },
);

try {
  await sleep(900);
  let pages;
  for (let i = 0; i < 25; i++) {
    try {
      pages = await getJson('/json/list');
      if (pages.length) break;
    } catch {
      /* retry */
    }
    await sleep(200);
  }
  if (!pages?.length) throw new Error('No CDP pages');

  const page = pages.find((p) => p.type === 'page') || pages[0];
  const cdp = new Cdp(page.webSocketDebuggerUrl);
  await cdp.ready();
  await cdp.send('Runtime.enable');
  await cdp.send('Page.enable');
  await cdp.send('Page.navigate', { url: URL });
  await sleep(1500);

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
      selectedCandidate: document.querySelector('.candidate-card.is-selected .card-name')?.textContent || null,
      selectedHouse: document.querySelector('.house-card.is-selected .card-name')?.textContent || null,
      titles: {
        ruler: document.querySelector('.crisis-facts dd')?.textContent,
        c1: document.querySelectorAll('.candidate-card .card-name')[0]?.textContent,
        c2: document.querySelectorAll('.candidate-card .card-name')[1]?.textContent,
        c3: document.querySelectorAll('.candidate-card .card-name')[2]?.textContent,
        player: document.querySelector('#player-panel h2')?.textContent,
      },
      hasPublicBadge: !!document.querySelector('.badge-public'),
      hasUnverifiedBadge: !!document.querySelector('.badge-unverified'),
      ariaNames: [...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => el.getAttribute('aria-label')).filter(Boolean).length,
      minTouch: Math.min(...[...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => el.getBoundingClientRect().height)),
    })`);
  }

  const desktop = await measure(1280, 720);

  await cdp.eval(`document.querySelectorAll('.candidate-card')[1].click()`);
  await sleep(250);
  const afterB = await cdp.eval(`({
    selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
    detailTitle: document.querySelector('#candidate-detail-title')?.textContent,
  })`);

  await cdp.eval(`document.querySelectorAll('.candidate-card')[2].click()`);
  await sleep(250);
  const afterC = await cdp.eval(`({
    selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
    detailTitle: document.querySelector('#candidate-detail-title')?.textContent,
  })`);

  await cdp.eval(`document.querySelector('[data-house-id="house-merova"]').click()`);
  await sleep(250);
  const afterHouse = await cdp.eval(`({
    selected: document.querySelector('.house-card.is-selected .card-name')?.textContent,
    detailTitle: document.querySelector('#house-detail-title')?.textContent,
    positiveCount: document.querySelectorAll('.reasons-positive li').length,
    negativeCount: document.querySelectorAll('.reasons-negative li').length,
  })`);

  // all five houses
  const houseIds = [
    'house-arden',
    'house-barren',
    'house-soren',
    'house-merova',
    'house-halbeck',
  ];
  const houseClicks = [];
  for (const id of houseIds) {
    await cdp.eval(`document.querySelector('[data-house-id="${id}"]').click()`);
    await sleep(150);
    houseClicks.push(
      await cdp.eval(`({
        id: '${id}',
        title: document.querySelector('#house-detail-title')?.textContent,
        stance: document.querySelector('.house-stance-large')?.textContent?.replace(/\\s+/g,' ').trim(),
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

  await cdp.eval(`document.querySelector('.candidate-card.is-selected')?.focus()`);
  await cdp.send('Input.dispatchKeyEvent', {
    type: 'keyDown',
    key: 'ArrowRight',
    code: 'ArrowRight',
    windowsVirtualKeyCode: 39,
  });
  await cdp.send('Input.dispatchKeyEvent', {
    type: 'keyUp',
    key: 'ArrowRight',
    code: 'ArrowRight',
    windowsVirtualKeyCode: 39,
  });
  await sleep(250);
  const afterKey = await cdp.eval(`({
    selected: document.querySelector('.candidate-card.is-selected .card-name')?.textContent,
    activeClass: document.activeElement?.className,
  })`);

  const mobile = await measure(390, 664);

  const consoleErrors = cdp.console.filter(
    (c) => c.type === 'error' || c.type === 'exception' || c.exceptionDetails,
  );

  const result = {
    desktop,
    afterB,
    afterC,
    afterHouse,
    houseClicks,
    afterPlayer,
    afterKey,
    mobile,
    consoleLogCount: cdp.console.length,
    consoleErrors,
  };

  console.log(JSON.stringify(result, null, 2));

  const failures = [];
  if (desktop.hasHScroll) failures.push('desktop horizontal scroll');
  if (mobile.hasHScroll) failures.push('mobile horizontal scroll');
  if (desktop.candidateCount !== 3) failures.push('candidate count');
  if (desktop.houseCount !== 5) failures.push('house count');
  if (desktop.titles?.c1 !== '세리아 아르케온') failures.push('candidate A name');
  if (afterB.selected !== '다리안 코르벤') failures.push('select B');
  if (afterC.selected !== '미레아 셀칸') failures.push('select C');
  if (afterHouse.selected !== '메로바 가문') failures.push('select merova');
  if (afterPlayer.relations !== 3) failures.push('player relations');
  if (afterPlayer.pressures !== 3) failures.push('player pressures');
  if (consoleErrors.length) failures.push('console errors');
  if (desktop.minTouch < 40) failures.push('touch target too small');
  if (desktop.ariaNames < 8) failures.push('missing aria names');

  cdp.close();
  chrome.kill();

  if (failures.length) {
    console.error('FAILURES:', failures.join(', '));
    process.exit(1);
  }
  console.log('BROWSER_VERIFY_OK');
  process.exit(0);
} catch (err) {
  console.error(err);
  chrome.kill();
  process.exit(1);
}
