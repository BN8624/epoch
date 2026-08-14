// Chrome 실행 파일 탐색 규칙만 담은 순수 helper (주입 가능한 env·platform·파일 검사)
import fs from 'fs';
import path from 'path';

export function defaultIsFile(p) {
  try {
    return fs.statSync(p).isFile();
  } catch {
    return false;
  }
}

function commonInstallCandidates(env, platform) {
  if (platform === 'win32') {
    return [env['ProgramFiles'], env['ProgramFiles(x86)'], env['LocalAppData']]
      .filter((base) => typeof base === 'string' && base.length > 0)
      .map((base) => path.win32.join(base, 'Google', 'Chrome', 'Application', 'chrome.exe'));
  }
  if (platform === 'darwin') {
    return ['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'];
  }
  return ['/usr/bin/google-chrome', '/usr/bin/google-chrome-stable', '/opt/google/chrome/chrome'];
}

function pathLookupCandidates(env, platform) {
  const names =
    platform === 'win32'
      ? ['chrome.exe', 'google-chrome.exe']
      : ['google-chrome', 'google-chrome-stable', 'chrome'];
  const sep = platform === 'win32' ? ';' : ':';
  const join = platform === 'win32' ? path.win32.join : path.posix.join;
  const dirs = String(env.PATH ?? '')
    .split(sep)
    .filter(Boolean);
  const out = [];
  for (const dir of dirs) {
    for (const name of names) out.push(join(dir, name));
  }
  return out;
}

// 우선순위: EPOCH_CHROME_PATH → 운영체제 일반 설치 위치 → PATH 탐색
export function resolveChromePath(options = {}) {
  const env = options.env ?? process.env;
  const platform = options.platform ?? process.platform;
  const isFile = options.isFile ?? defaultIsFile;
  const tried = [];

  const explicit = env.EPOCH_CHROME_PATH;
  if (typeof explicit === 'string' && explicit.length > 0) {
    tried.push(explicit);
    if (isFile(explicit)) return { path: explicit, source: 'EPOCH_CHROME_PATH', tried };
  }

  for (const candidate of commonInstallCandidates(env, platform)) {
    tried.push(candidate);
    if (isFile(candidate)) return { path: candidate, source: 'common-install-path', tried };
  }

  for (const candidate of pathLookupCandidates(env, platform)) {
    tried.push(candidate);
    if (isFile(candidate)) return { path: candidate, source: 'PATH', tried };
  }

  return { path: null, source: null, tried };
}

export function chromeNotFoundMessage(tried = []) {
  return [
    'Chrome executable was not found.',
    'Set EPOCH_CHROME_PATH to the Chrome executable.',
    `Checked ${tried.length} location(s).`,
    ...tried.slice(0, 8).map((t) => `  - ${t}`),
  ].join('\n');
}
