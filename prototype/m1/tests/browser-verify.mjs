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

  async key(key, code, windowsVirtualKeyCode) {
    await this.send('Input.dispatchKeyEvent', {
      type: 'keyDown',
      key,
      code,
      windowsVirtualKeyCode,
    });
    await this.send('Input.dispatchKeyEvent', {
      type: 'keyUp',
      key,
      code,
      windowsVirtualKeyCode,
    });
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
      touchTargets: [...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => {
          const r = el.getBoundingClientRect();
          return { w: r.width, h: r.height, tag: el.className };
        }),
      minTouchW: Math.min(...[...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => el.getBoundingClientRect().width)),
      minTouchH: Math.min(...[...document.querySelectorAll('.candidate-card,.house-card,#player-toggle')]
        .map(el => el.getBoundingClientRect().height)),
      layoutIssues: (() => {
        const issues = [];
        const vw = window.innerWidth;
        const core = [...document.querySelectorAll(
          '.candidate-card,.house-card,#player-toggle,#candidate-detail,#house-detail,.crisis-facts,h1,h2'
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

  const desktop = await measure(1280, 720);

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
  // 후보 A(세리아) 선택 후 포커스, ArrowRight → B
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

  // ArrowRight again → C
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

  const mobile = await measure(390, 664);

  const consoleErrors = cdp.console.filter(
    (c) => c.type === 'error' || c.type === 'exception' || c.exceptionDetails,
  );

  const result = {
    desktop,
    candidateClicks,
    houseClicks,
    afterPlayer,
    afterCandidateKey,
    afterCandidateKey2,
    afterHouseKey,
    afterHouseKey2,
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
  if (desktop.titles?.c2 !== '다리안 코르벤') failures.push('candidate B name');
  if (desktop.titles?.c3 !== '미레아 셀칸') failures.push('candidate C name');

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
