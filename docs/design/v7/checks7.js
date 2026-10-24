// Vibecasso · checkpoints in all three layouts (design brief v7). Cell-grid spec. SAMPLE DATA ONLY.
import { Grid, trunc, segLen, L, mix, THEMES as GT } from './seshi-grid.js';
const PT = {
  pixls: { name: 'dark', label: 'Vibecasso Pixls', bg: '#141413', fg: '#c9c5bb', card: '#1c1c1a', card2: '#262624', btn: '#2e2d2a', hov: '#3a3935', line: '#3a3934', dim: '#8a867c', text: '#c9c5bb', strong: '#ece9e1',
    acc: '#ff6a2f', accInk: '#141413', needs: '#f2c200', work: '#c9c5bb', ok: '#3cc27e', err: '#ff5a52', rev: '#b58cff', wait: '#8fa6ff', sky: '#5aa9ff', scrim: '#000000',
    a: { green: '#7fd962', red: '#ff6b6b', cyan: '#3dd6c0', blue: '#5aa9ff', yellow: '#e8c565', magenta: '#a593ff', gray: '#8a867c' } },
  night: { name: 'dark', label: 'Vibecasso Night', bg: '#070b10', fg: '#c9d1d9', card: '#0f1821', card2: '#18242f', btn: '#1d2a37', hov: '#2a3a4c', line: '#1f2c3a', dim: '#71808f', text: '#a7b4c2', strong: '#f2f6f8',
    acc: '#c3f53c', accInk: '#0a1204', needs: '#ffb547', work: '#a7b4c2', ok: '#7fd962', err: '#ff6b6b', rev: '#a593ff', wait: '#5aa9ff', sky: '#5aa9ff', scrim: '#000000',
    a: { green: '#7fd962', red: '#ff6b6b', cyan: '#3dd6c0', blue: '#5aa9ff', yellow: '#e8c565', magenta: '#a593ff', gray: '#71808f' } },
};
const lt = GT.light;
export const THEMES = { pixls: PT.pixls, night: PT.night,
  paper: { name: 'light', label: 'Vibecasso Paper', bg: lt.bg, fg: lt.fg, card: lt.surf, card2: lt.surf2, btn: lt.btn, line: lt.line, dim: lt.dim, text: lt.text, strong: lt.strong, acc: lt.acc, accInk: '#ffffff', needs: lt.needs, work: lt.work, ok: lt.ok, err: lt.err, rev: lt.ws.violet, sky: lt.ws.sky, wait: lt.ws.sky, scrim: lt.scrim, a: lt.a } };

const bl = [0, 0x40, 0x44, 0x46, 0x47], brr = [0, 0x80, 0xA0, 0xB0, 0xB8];
const BARS = [[1, 3, 2], [2, 4, 1], [3, 3, 3], [4, 2, 4], [3, 1, 3], [2, 2, 2], [1, 4, 3], [2, 3, 4], [4, 1, 2], [3, 2, 1]].map(([a, b, c]) => String.fromCharCode(0x2800 + bl[a] + brr[b]) + String.fromCharCode(0x2800 + bl[c]));
const GL = {
  unicode: { work: BARS, check: ['◜', '◝', '◞', '◟'], needs: '●', done: '✓', failed: '✕', over: '⊘', couldnt: '?', stale: '◷', idle: '○', asleep: 'z', pend: '…', bar: '▎', branch: '⎇', close: '✕', fold: '▾', warn: '⚠', arrow: '›' },
  ascii: { work: ['. ', 'o ', 'O ', 'o '], check: ['-', '\\', '|', '/'], needs: '!', done: '+', failed: 'x', over: '/', couldnt: '?', stale: '~', idle: 'o', asleep: 'z', pend: '.', bar: '|', branch: 'git:', close: 'x', fold: 'v', warn: '!', arrow: '>' },
};
GL.nerd = { ...GL.unicode, needs: '\uf0f3', done: '\uf00c', failed: '\uf00d', over: '\uf05e', couldnt: '\uf128', stale: '\uf017', idle: '\uf10c', branch: '\ue0a0', close: '\uf00d', warn: '\uf071' };

// session state → glyph / colour / word. Shape + word always carry the meaning; colour only reinforces it.
const glyph = (G, k, s) => k === 'working' ? G.work[s.motion ? s.f % G.work.length : 2] : k === 'checking' ? G.check[s.motion ? (s.f >> 1) % 4 : 0] : ({ never: G.couldnt, needs: G.needs, approve: G.needs, done: G.done, passed: G.done, stale: G.stale, failed: G.failed, over: G.over, couldnt: G.couldnt, idle: G.idle, asleep: G.asleep })[k] || '·';
const col = (T, k) => ({ working: T.acc, checking: T.sky, needs: T.needs, approve: T.needs, done: T.ok, passed: T.ok, stale: T.text, never: T.rev, failed: T.err, over: T.text, couldnt: T.rev, idle: T.dim, asleep: T.dim })[k] || T.text;
function shimmer(t, s, T, extra = {}) {
  const a = Array.from(t), n = a.length, base = mix(T.acc, T.bg, 0.2), hi = mix(T.acc, '#ffffff', 0.55), pos = s.motion ? (s.f % (n + 8)) - 4 : -99;
  return a.map((ch, i) => { const d = Math.abs(i - pos); return { t: ch, fg: d < 1 ? hi : d < 2.5 ? mix(hi, base, 0.5) : base, b: 1, ...extra }; });
}
const WORD = { working: 'working', checking: 'checking', needs: 'needs you', approve: 'needs you', done: 'done', passed: 'done', stale: 'done · stale', failed: 'failed', over: 'done · overridden', couldnt: 'not checked', never: 'not checked', idle: 'idle', asleep: 'asleep' };
const crG = (G, r, s) => r === 'pass' ? G.done : r === 'fail' ? G.failed : r === 'run' ? G.check[s.motion ? (s.f >> 1) % 4 : 0] : r === 'err' ? G.couldnt : r === 'stale' ? G.stale : r === 'over' ? G.over : r === 'need' ? G.needs : G.pend;
const crC = (T, r) => ({ pass: T.ok, fail: T.err, run: T.sky, err: T.rev, stale: T.dim, over: T.text, need: T.needs })[r] || T.dim;
const bordered = s => s.layout !== 'flat';
let CS = null; // current state, for modal corners

export const PRESETS = [
  ['01 No check set', { stage: 'none' }],
  ['02 Row · checking', { stage: 'checking' }],
  ['03 Row · passed', { stage: 'passed' }],
  ['04 Row · failed', { stage: 'failed' }],
  ['05 Row · overridden', { stage: 'over' }],
  ['06 Row · could not run', { stage: 'couldnt' }],
  ['07 Evidence · short output', { stage: 'failed', sheet: 'evidence' }],
  ['08 Evidence · long output', { stage: 'failed', sheet: 'evidence', out: 'long' }],
  ['09 Evidence · no output', { stage: 'failed', sheet: 'evidence', out: 'empty' }],
  ['10 Evidence · printed nothing', { stage: 'passed', sheet: 'evidence', out: 'empty' }],
  ['11 Inbox', { stage: 'failed', sheet: 'inbox', isel: 1 }],
  ['12 Override?', { stage: 'failed', sheet: 'evidence', modal: 'override' }],
  ['13 Why? (i)', { stage: 'failed', pop: 1 }],
  ['14 Settings · empty', { stage: 'none', modal: 'settings', set: 'empty' }],
  ['15 Settings · set', { stage: 'passed', modal: 'settings', set: 'set' }],
  ['16 Settings · never run', { stage: 'none', modal: 'settings', set: 'never' }],
  ['17 Settings · command missing', { stage: 'couldnt', modal: 'settings', set: 'missing' }],
  ['A1 Repo command waits', { stage: 'approve', sheet: 'inbox', isel: 1 }],
  ['A2 Approve · read first', { stage: 'approve', modal: 'approve', apv: 'new' }],
  ['A3 Approve · confirm always', { stage: 'approve', modal: 'approve', apv: 'confirm' }],
  ['A4 Approve · command changed', { stage: 'approve', modal: 'approve', apv: 'changed' }],
  ['A5 Never for this repo', { stage: 'never', sheet: 'evidence' }],
  ['B1 Stale · row', { stage: 'stale' }],
  ['B2 Stale · evidence', { stage: 'stale', sheet: 'evidence' }],
  ['C1 Several checkpoints · rows', { stage: 'multi' }],
  ['C2 Several checkpoints · Inbox', { stage: 'multi', sheet: 'inbox' }],
];
export function preset(name, W, H, fl = {}) {
  const p = (PRESETS.find(x => x[0] === name) || PRESETS[0])[1];
  return { W, H, theme: fl.theme || 'pixls', glyphs: fl.glyphs || 'unicode', layout: fl.layout || 'flat', corners: fl.corners || 'square', motion: fl.motion !== false, apv: 'new', f: 0, stage: 'none', sheet: null, modal: null, set: null, out: 'normal', pop: 0, sel: 0, isel: 0, toast: null, leader: false, reason: 1, ...p, focus: p.modal ? 'modal' : p.sheet ? 'sheet' : 'side' };
}

function sessions(T, G, s) {
  const st = s.stage, C = (n, r, t) => ({ n, r, t });
  const ff = { id: 'fix-footer', agent: 'codex', age: '2m' };
  const set = (k, checks, sub) => Object.assign(ff, { k, checks, sub });
  if (st === 'none') set('done', [], [{ t: 'finished · not seen', fg: T.dim }]);
  if (st === 'approve') set('approve', [C('test', 'pend')], [{ t: 'approve check: npm test', fg: T.needs }]);
  if (st === 'checking') set('checking', [C('test', 'run', '6s')], [{ t: 'checking test · 6s', fg: T.sky }]);
  if (st === 'passed') set('passed', [C('test', 'pass', '14s')], [{ t: 'passed: test', fg: T.ok, b: 1 }, { t: ' · 14.2s · 14:22', fg: T.dim }]);
  if (st === 'failed') set('failed', [C('test', 'fail', 'exit 1')], [{ t: 'failed: test · exit 1', fg: T.err, b: 1 }]); ff.l3 = [{ t: 'footer shows current year', fg: T.dim }];
  if (st === 'over') set('over', [C('test', 'over')], [{ t: 'failed, overridden: test', fg: T.text, b: 1 }]);
  if (st === 'couldnt') set('couldnt', [C('test', 'err')], [{ t: 'test could not run · not found', fg: T.rev }]);
  if (st === 'stale') set('stale', [C('test', 'stale')], [{ t: 'stale pass: test', fg: T.text, b: 1 }, { t: ' · 3 files changed', fg: T.dim }]);
  if (st === 'never') set('never', [C('test', 'err')], [{ t: 'not checked: never for this repo', fg: T.rev }]);
  if (st === 'multi') set('failed', [C('commits', 'pass'), C('test', 'fail'), C('approval', 'pend'), C('review', 'pend')], null);
  ff.keep = st !== 'none';
  const ap = st === 'multi' ? { id: 'add-payments', agent: 'claude', age: '40s', k: 'approve', keep: 1, checks: [C('commits', 'pass'), C('test', 'pass'), C('approval', 'need'), C('review', 'pend')] } : { id: 'add-payments', agent: 'claude', age: 'now', k: 'working', checks: [], sub: [{ t: 'claude · ', fg: T.dim }, ...shimmer('working', s, T)] };
  const hasCheck = st !== 'none';
  return [
    { name: 'affiliate-site', meta: G.branch + ' main' + (hasCheck ? (st === 'multi' ? ' · 4 checkpoints' : ' · check: test') : ''), list: [ff, ap] },
    { name: 'pixls', meta: 'no git', list: [{ id: 'warm-finch', agent: 'codex', age: '3m', k: 'needs', keep: 1, checks: [], sub: [{ t: 'asks: use pnpm or npm?', fg: T.needs }] }] },
    { name: 'orbit', meta: G.branch + ' dev', list: [{ id: 'quick-fox', agent: 'claude', age: '9h', k: 'done', checks: [], sub: [{ t: 'finished', fg: T.dim }] }, { id: 'old-moth', agent: 'codex', age: '2d', k: 'asleep', checks: [], sub: [{ t: 'asleep zzZ', fg: T.dim }] }] },
  ];
}
function chips(T, G, s, checks, narrow) {
  if (narrow) {
    const bad = checks.find(c => c.r === 'fail' || c.r === 'err'), need = checks.find(c => c.r === 'need'), pend = checks.find(c => c.r === 'pend' || c.r === 'run');
    const word = bad ? { t: ' failed: ' + bad.n, fg: crC(T, bad.r), b: 1 } : need ? { t: ' needs you: ' + need.n, fg: T.needs, b: 1 } : pend ? { t: ' next: ' + pend.n, fg: T.dim } : { t: ' all passed', fg: T.ok, b: 1 };
    return [{ ...word, t: word.t.trim() + ' ' }, ...checks.map(c => ({ t: crG(G, c.r, s), fg: crC(T, c.r), b: 1 }))];
  }
  return checks.flatMap((c, i) => [...(i ? [{ t: ' · ', fg: T.dim }] : []), { t: c.n + ' ', fg: c.r === 'fail' ? T.err : T.text }, { t: crG(G, c.r, s), fg: crC(T, c.r), b: 1 }]);
}

// ---------- panels: Cards floating (border + 1-col gap), Cards tiled (border, edge to edge), Flat (no borders, thin rules)
function panel(g, T, s, x, y, w, h, lit, title, right) {
  const cb = lit ? T.acc : T.btn, cf = lit ? T.accInk : T.strong;
  const chip = title ? [{ t: ' ', bg: cb }, ...title.map(t => ({ ...t, fg: lit ? cf : (t.fg === T.dim ? T.text : t.fg || T.strong), bg: cb })), { t: ' ', bg: cb }] : null;
  if (bordered(s)) {
    const S = { fg: lit ? T.acc : T.line }, K = s.corners === 'rounded' ? '╭╮╰╯' : '┌┐└┘';
    g.set(x, y, K[0], S); g.set(x + w - 1, y, K[1], S); g.set(x, y + h - 1, K[2], S); g.set(x + w - 1, y + h - 1, K[3], S);
    g.hl(x + 1, y, w - 2, '─', S); g.hl(x + 1, y + h - 1, w - 2, '─', S); g.vl(x, y + 1, h - 2, '│', S); g.vl(x + w - 1, y + 1, h - 2, '│', S);
    if (chip) g.putSegs(x + 2, y, chip); if (right) g.putSegs(x + w - 3 - segLen(right), y, [{ t: ' ' }, ...right, { t: ' ' }]);
    return;
  }
  g.fill(x, y, w, 1, T.card);
  if (x > 0) { g.set(x, y, '│', { fg: T.line, bg: T.card }); g.vl(x, y + 1, h - 1, '│', { fg: lit ? T.acc : T.line }); }
  if (chip) g.putSegs(x + 1, y, chip);
  if (right) { const rs = right.map(t => ({ ...t, bg: T.card })); g.putSegs(x + w - 1 - segLen(rs), y, rs); }
}
const btn = (g, R, x, y, T, label, key, st, act) => { const P = st === 'primary' ? [T.acc, T.accInk, T.accInk] : st === 'danger' ? [T.btn, T.err, T.acc] : [T.btn, T.strong, T.acc]; const x2 = g.putSegs(x, y, [{ t: ' ' + label + ' ', bg: P[0], fg: P[1], b: 1 }, ...(key ? [{ t: key + ' ', bg: P[0], fg: P[2], b: 1 }] : [])]); R.push({ x, y, w: x2 - x, h: 1, act }); return x2 + 1; };
const hints = (T, list) => list.flatMap((h, i) => [...(i ? [{ t: '   ' }] : []), { t: h[0], fg: T.acc, b: 1 }, { t: ' ' + h[1], fg: T.text }]);
const pad = (s, n) => { const a = Array.from(s); return a.length >= n ? trunc(s, n) : s + ' '.repeat(n - a.length); };
function chipHead(g, y, x, T, label, c, n, ink) { return g.putSegs(x, y, [{ t: ' ' + label + ' ', fg: ink, bg: c, b: 1 }, ...(n != null ? [{ t: ' ' + n + ' ', fg: ink, bg: mix(c, T.bg, 0.25), b: 1 }] : [])]); }

// ---------- sidebar. Below 140 cols a row is one line unless line 2 is a check result or a question; below 100 the age drops.
function drawSide(g, R, T, G, s, F, x, y, w, h) {
  const lit = s.focus === 'side', tint = mix(T.bg, T.text, 0.12), B = bordered(s);
  const all = F.flatMap(f => f.list), nNeeds = all.filter(q => q.k === 'needs' || q.k === 'approve').length, nFail = all.filter(q => q.k === 'failed').length;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'SESSIONS', b: 1 }, { t: ' ' + all.length, fg: T.dim }], [...(nNeeds ? [{ t: G.needs + nNeeds, fg: T.needs, b: 1 }] : []), ...(nFail ? [{ t: (nNeeds ? ' ' : '') + G.failed + nFail, fg: T.err, b: 1 }] : [])]);
  let yy = y + 2, idx = 0; const X = x + (B ? 2 : 1), RX = x + w - (B ? 3 : 2), wide = s.W >= 140, showAge = s.W >= 100;
  for (const f of F) {
    const room = RX - X - 3 - L(f.name), meta = room >= 6 ? trunc(f.meta, room) : '';
    g.putSegs(X + 1, yy, [{ t: G.fold + ' ', fg: T.acc }, { t: trunc(f.name, RX - X - 2), fg: T.strong, b: 1 }]); if (meta) g.put(RX + 1 - L(meta), yy, meta, { fg: T.dim }); yy++;
    for (const q of f.list) {
      const on = idx === s.sel, bg = on ? tint : undefined, fail = q.k === 'failed', multi = q.checks.length > 1;
      const l2 = multi ? chips(T, G, s, q.checks, true) : (wide || q.keep) ? q.sub || [] : [];
      const l3 = wide ? (multi ? chips(T, G, s, q.checks, false) : q.l3) : null, n = 1 + (l2.length ? 1 : 0) + (l3 ? 1 : 0);
      if (on) g.fill(X, yy, RX - X + 2, n, tint);
      if (on || fail) for (let j = 0; j < n; j++) g.set(X, yy + j, G.bar, { fg: fail ? T.err : lit ? T.acc : T.dim, bg, b: 1 });
      const age = showAge ? q.age : '', nm = trunc(q.id, RX - X - 4 - L(age));
      g.putSegs(X + 2, yy, [{ t: glyph(G, q.k, s) + ' ', fg: col(T, q.k), bg, b: fail || q.k === 'needs' || q.k === 'approve' || q.k === 'working' }, ...(q.k === 'working' ? shimmer(nm, s, T, { bg }) : [{ t: nm, fg: on || fail ? T.strong : T.text, b: 1, bg }])]);
      if (age) g.put(RX + 1 - L(age), yy, age, { fg: T.dim, bg });
      if (l2.length) g.putSegs(X + 4, yy + 1, l2.map(t => ({ ...t, bg })), {}, RX + 2);
      if (l3) g.putSegs(X + 4, yy + 1 + (l2.length ? 1 : 0), l3.map(t => ({ ...t, bg })), {}, RX + 2);
      R.push({ x: X, y: yy, w: RX - X + 2, h: n, act: { t: 'selSess', i: idx } });
      yy += n; idx++;
    }
    yy++;
  }
  const by = y + h - (B ? 2 : 1);
  g.putSegs(X + 1, by, [{ t: '+', fg: T.acc, b: 1 }, { t: ' new', fg: T.text }]);
  const ac = 'a actions'; g.putSegs(RX + 1 - L(ac), by, [{ t: 'a', fg: T.acc, b: 1 }, { t: ' actions', fg: T.text }]);
  R.push({ x: RX + 1 - L(ac), y: by, w: L(ac), h: 1, act: { t: 'toast', v: 'Row menu (built): Re-run check · Override… · Undo override' } });
}

// ---------- terminal (agent output as-is)
function drawTerm(g, T, x, y, w, h) {
  const a = T.a, E = [], lines = [
    [{ t: '› ', fg: T.dim }, { t: 'rename the footer copyright to 2026', fg: T.strong }], E,
    [{ t: '• ', fg: a.green }, { t: 'Edit ', b: 1 }, { t: 'src/components/Footer.tsx', fg: a.cyan }],
    [{ t: '  - ', fg: a.red }, { t: '© 2025 Affiliate Co.', fg: a.red }], [{ t: '  + ', fg: a.green }, { t: '© 2026 Affiliate Co.', fg: a.green }], E,
    [{ t: '• ', fg: a.green }, { t: 'Ran ' }, { t: 'npm test -- Footer', fg: a.blue }], [{ t: '  └ 3 passed', fg: T.dim }], E,
    [{ t: '• ', fg: a.green }, { t: 'Done. Footer updated and its tests pass.' }], E, [{ t: 'Worked for 48s', fg: T.dim }]];
  const top = y + Math.max(1, h - 3 - lines.length);
  lines.forEach((l, i) => g.putSegs(x, top + i, l, {}, x + w));
  g.putSegs(x, y + h - 2, [{ t: '› ', fg: T.strong }, { t: 'Ask Codex to do anything', fg: T.dim }], {}, x + w);
}

// ---------- evidence (sheet)
const OUT_FAIL = [['> affiliate-site@1.4.0 test', 'd'], ['> vitest run', 'd'], ['', ''], [' ✓ src/components/Footer.test.tsx (3)', 'ok'], [' ✓ src/lib/price.test.ts (9)', 'ok'], [' ✗ src/pages/legal.test.tsx > footer shows current year', 'err'], ["   AssertionError: expected '© 2025 Affiliate Co.'", 'err'], ["                   to equal '© 2026 Affiliate Co.'", 'err'], ['    ❯ src/pages/legal.test.tsx:18:32', 'd'], ['', ''], [' Test Files  1 failed | 11 passed (12)', 'err'], ['      Tests  1 failed | 58 passed (59)', 'err'], ['   Duration  14.21s', 'd']];
const OUT_PASS = [['> vitest run', 'd'], [' ✓ 12 test files · 59 tests', 'ok'], ['   Duration  14.02s', 'd']];
function drawEvidence(g, R, T, G, s, x, y, w, h) {
  const lit = s.focus === 'sheet', X = x + 2, iw = w - 4, st = s.stage;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'CHECK', b: 1 }, { t: ' · fix-footer › test', fg: T.dim }], [{ t: G.close + ' ', fg: T.dim }]);
  R.push({ x: x + w - 3, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 2;
  const kv = (k, v) => { g.putSegs(X, yy++, [{ t: pad(k, 9), fg: T.dim }, ...v], {}, X + iw); };
  const res = { failed: [G.failed + ' failed', T.err, 'exit 1'], over: [G.over + ' failed · overridden', T.text, 'exit 1'], couldnt: [G.couldnt + ' could not run', T.rev, 'exit 127'], passed: [G.done + ' passed', T.ok, 'exit 0'], stale: [G.stale + ' stale pass', T.btn, 'exit 0 on older code'], never: [G.couldnt + ' not checked', T.rev, 'never for this repo'], multi: [G.failed + ' failed', T.err, 'exit 1'] }[st] || [G.done + ' passed', T.ok, 'exit 0'];
  g.putSegs(X, yy++, [{ t: ' ' + res[0] + ' ', fg: st === 'stale' ? T.strong : T.bg, bg: res[1], b: 1 }, { t: '  ' + res[2], fg: T.strong, b: 1 }, { t: st === 'couldnt' ? '  command not found' : '', fg: T.rev }]); yy++;
  kv('Command', [{ t: st === 'couldnt' ? 'pnpm test' : 'npm test', fg: T.strong, b: 1 }]);
  kv('Folder', [{ t: '~\\dev\\affiliate-site-worktrees\\fix-footer', fg: T.text }]);
  kv('Ran', [{ t: '14:22:05', fg: T.text }, { t: ' · took ', fg: T.dim }, { t: st === 'couldnt' ? '0.1s' : '14.2s', fg: T.text }, { t: ' · after turn 3 ended 14:21:51', fg: T.dim }]);
  if (st === 'never') kv('Source', [{ t: '.vibecasso/checks.toml · you chose never 14:31', fg: T.text }]); else kv('Seen by', [{ t: 'Vibecasso ran this itself. The agent said "tests pass".', fg: T.dim }]);
  yy++;
  if (st === 'over') { g.fill(X, yy, iw, 2, mix(T.bg, T.text, 0.1)); g.putSegs(X + 1, yy++, [{ t: G.over + ' Overridden by you 14:30', fg: T.strong, b: 1 }, { t: '  · u undo', fg: T.dim }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'reason: flaky legal test, tracked in #212 · next run replaces this', fg: T.text }], {}, X + iw); yy++; }
  if (st === 'stale') { const sb = mix(T.bg, T.text, 0.1); g.fill(X, yy, iw, 5, sb);
    g.putSegs(X + 1, yy++, [{ t: G.stale + ' Stale.', fg: T.strong, b: 1, bg: sb }, { t: ' The agent changed 3 files after this ran (14:24).', fg: T.text, bg: sb }], {}, X + iw);
    g.putSegs(X + 3, yy++, [{ t: 'This pass is for older code, not for what is there now.', fg: T.text, bg: sb }], {}, X + iw);
    [['M', 'src/components/Footer.tsx'], ['M', 'src/pages/legal.tsx'], ['A', 'tests/legal.test.tsx']].forEach(([m, f]) => g.putSegs(X + 3, yy++, [{ t: m + ' ', fg: T.dim, bg: sb }, { t: f, fg: T.text, bg: sb }], {}, X + iw)); yy++; }
  if (st === 'never') { const sb = mix(T.bg, T.rev, 0.12); g.fill(X, yy, iw, 2, sb); g.putSegs(X + 1, yy++, [{ t: G.couldnt + ' Not checked.', fg: T.rev, b: 1, bg: sb }, { t: ' You chose never for this repo\'s command.', fg: T.text, bg: sb }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'Done means the agent said so. Change it in Settings › Check.', fg: T.dim, bg: sb }], {}, X + iw); yy++; }
  if (st === 'couldnt') { g.fill(X, yy, iw, 2, mix(T.bg, T.rev, 0.12)); g.putSegs(X + 1, yy++, [{ t: G.couldnt + ' Not a test failure.', fg: T.rev, b: 1 }, { t: ' The work hasn\'t been checked.', fg: T.text }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'Fix the command in settings (,) or install pnpm, then r.', fg: T.dim }], {}, X + iw); yy++; }
  const out = st === 'couldnt' ? [['sh: 1: pnpm: not found', 'err']] : s.out === 'empty' || st === 'never' ? [] : st === 'passed' || st === 'stale' ? OUT_PASS : OUT_FAIL;
  const total = s.out === 'long' ? 1284 : out.length, room = Math.max(3, y + h - 4 - yy - 1);
  chipHead(g, yy, X, T, 'OUTPUT', T.btn, null, T.strong);
  g.putSegs(X + 9, yy++, [{ t: s.out === 'empty' ? 'empty' : out.length > room || s.out === 'long' ? 'last ' + Math.min(room, out.length) + ' of ' + total.toLocaleString() + ' lines' : 'all ' + out.length + ' lines', fg: T.dim }, ...(s.out === 'long' ? [{ t: '  f full output', fg: T.dim }] : [])]);
  const ob = mix(T.bg, T.scrim, 0.25), oh = Math.min(room, Math.max(out.length, 3));
  g.fill(X, yy, iw, oh, ob);
  if (!out.length) { const E = st === 'never' ? ['(nothing ran)', 'There is no output because Vibecasso did not run it.'] : st === 'failed' ? ['(no output)', 'It exited 1 without printing. Run it in the session to see more.'] : ['(the command printed nothing)', 'exit 0 is what passed it.']; g.put(X + 2, yy + 1, E[0], { fg: T.dim, bg: ob, i: 1 }); g.put(X + 2, yy + 2, trunc(E[1], iw - 3), { fg: T.dim, bg: ob }); }
  const shown = out.slice(Math.max(0, out.length - oh)), first = total - shown.length + 1;
  shown.forEach(([t, k], i) => { g.putSegs(X, yy + i, [{ t: pad(String(first + i), 5), fg: mix(T.dim, ob, 0.35), bg: ob }, { t: trunc(t, iw - 6), fg: k === 'err' ? T.err : k === 'ok' ? T.ok : k === 'd' ? T.dim : T.text, bg: ob }], {}, X + iw); });
  if (s.out === 'long') g.set(X + iw - 1, yy, '▲', { fg: T.dim, bg: ob });
  const by = y + h - 3; let bx = X;
  if (st === 'never') bx = btn(g, R, bx, by, T, 'Check settings', ',', 'normal', { t: 'modal', v: 'settings', set: 'set' }); else bx = btn(g, R, bx, by, T, 'Re-run', 'r', st === 'stale' ? 'primary' : 'normal', { t: 'rerun' });
  if (st === 'failed' || st === 'multi') bx = btn(g, R, bx, by, T, 'Override…', 'O', 'danger', { t: 'modal', v: 'override' });
  if (st === 'over') bx = btn(g, R, bx, by, T, 'Undo override', 'u', 'normal', { t: 'go', p: '07 Evidence · short output' });
  if (bx + 18 < X + iw) bx = btn(g, R, bx, by, T, 'Open session', 'o', 'normal', { t: 'sheet', v: null });
  g.putSegs(X, y + h - 2, hints(T, [['Esc', 'back'], ['r', 're-run'], ['f', 'full output'], ['i', 'why']]), {}, X + iw);
}

// ---------- inbox (sheet): needs you first, then failed checks, then finished
function inboxItems(T, G, s) {
  const st = s.stage, it = [];
  it.push({ grp: 'needs', id: 'warm-finch', proj: 'pixls', age: '3m', k: 'needs', l2: [{ t: 'Use pnpm or npm for the workspace?', fg: T.needs }], acts: [['Reply', 'm', { t: 'toast', v: 'Follow-up popover (existing m)' }], ['Open', 'o', { t: 'sheet', v: null }]] });
  if (st === 'approve') it.push({ grp: 'needs', id: 'fix-footer', proj: 'affiliate-site', age: '1m', k: 'approve', l2: [{ t: 'approve check command: npm test', fg: T.needs }], acts: [['Read the command', '⏎', { t: 'modal', v: 'approve', apv: 'new' }]] });
  if (st === 'multi') it.push({ grp: 'needs', id: 'add-payments', proj: 'affiliate-site', age: '40s', k: 'approve', mc: [{ n: 'commits', r: 'pass' }, { n: 'test', r: 'pass' }, { n: 'approval', r: 'need' }, { n: 'review', r: 'pend' }], l2: [], acts: [['Open plan', '⏎', { t: 'toast', v: 'Approval checkpoint: shape only, screen not designed' }]] });
  if (st === 'failed' || st === 'multi') it.push({ grp: 'failed', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: 'failed', mc: st === 'multi' ? [{ n: 'commits', r: 'pass' }, { n: 'test', r: 'fail' }, { n: 'approval', r: 'pend' }, { n: 'review', r: 'pend' }] : null, l2: [{ t: 'failed: test · exit 1 · 14:22', fg: T.err, b: 1 }], l3: [{ t: '✗ legal.test.tsx › footer shows current year', fg: T.dim }], acts: [['Output', '⏎', { t: 'sheet', v: 'evidence' }], ['Re-run', 'r', { t: 'rerun' }], ['Override…', 'O', { t: 'modal', v: 'override' }]] });
  if (st === 'couldnt') it.push({ grp: 'couldnt', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: 'couldnt', l2: [{ t: 'test could not run: pnpm not found', fg: T.rev }], acts: [['Output', '⏎', { t: 'sheet', v: 'evidence' }], ['Settings', ',', { t: 'modal', v: 'settings', set: 'missing' }]] });
  if (st === 'passed' || st === 'stale' || st === 'over' || st === 'none' || st === 'never') it.push({ grp: 'done', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: st === 'none' ? 'done' : st, l2: st === 'over' ? [{ t: 'failed, overridden: test', fg: T.text, b: 1 }] : st === 'stale' ? [{ t: 'stale pass: test', fg: T.text, b: 1 }, { t: ' · 3 files changed since', fg: T.dim }] : st === 'never' ? [{ t: 'not checked: never for this repo', fg: T.rev }] : st === 'none' ? [{ t: 'finished', fg: T.dim }] : [{ t: 'passed: test', fg: T.ok, b: 1 }, { t: ' · 14.2s', fg: T.dim }], acts: st === 'stale' ? [['Re-run', 'r', { t: 'rerun' }], ['Output', '⏎', { t: 'sheet', v: 'evidence' }]] : [] });
  it.push({ grp: 'done', id: 'quick-fox', proj: 'orbit', age: '9h', k: 'done', l2: [{ t: 'finished · not seen', fg: T.dim }], acts: [] });
  return it;
}
function drawInbox(g, R, T, G, s, x, y, w, h) {
  const lit = s.focus === 'sheet', X = x + 2, iw = w - 4, items = inboxItems(T, G, s), tint = mix(T.bg, T.text, 0.12), bodyBg = undefined;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'INBOX', b: 1 }, { t: ' ' + items.filter(i => i.grp !== 'done').length, fg: T.dim }], [{ t: G.close + ' ', fg: T.dim }]);
  items.forEach(q => { if (q.mc) { const nr = chips(T, G, s, q.mc, true), full = [{ ...nr[0], t: nr[0].t.trim() }, { t: ' · ', fg: T.dim }, ...chips(T, G, s, q.mc, false)]; q.l2 = segLen(full) <= iw - 2 ? full : nr; } });
  R.push({ x: x + w - 3, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 2, i = 0;
  for (const [grp, label, c] of [['needs', 'NEEDS YOU', T.needs], ['failed', 'FAILED CHECKS', T.err], ['couldnt', 'COULD NOT RUN', T.rev], ['done', 'FINISHED', T.ok]]) {
    const list = items.filter(q => q.grp === grp); if (!list.length) continue;
    chipHead(g, yy++, X, T, label, c, list.length, T.bg);
    for (const q of list) {
      if (yy > y + h - 5) break;
      const on = s.isel === i && lit, n = 2 + (q.l3 ? 1 : 0) + (q.acts.length && on ? 1 : 0), bg = on ? tint : bodyBg;
      if (on) { g.fill(x + 1, yy, w - 2, n, tint); for (let j = 0; j < n; j++) g.set(x + 1, yy + j, G.bar, { fg: q.k === 'failed' ? T.err : T.acc, bg: tint }); }
      else if (q.k === 'failed') for (let j = 0; j < n; j++) g.set(x + 1, yy + j, G.bar, { fg: T.err });
      g.putSegs(X, yy, [{ t: glyph(G, q.k, s) + ' ', fg: col(T, q.k), b: 1, bg }, { t: q.id, fg: T.strong, b: 1, bg }, { t: ' · ' + q.proj, fg: T.dim, bg }], {}, X + iw - 4); g.put(X + iw - L(q.age), yy, q.age, { fg: T.dim, bg });
      g.putSegs(X + 2, yy + 1, q.l2.map(t => ({ ...t, bg })), {}, X + iw);
      if (q.l3) g.putSegs(X + 2, yy + 2, q.l3.map(t => ({ ...t, bg })), {}, X + iw);
      R.push({ x: x + 1, y: yy, w: w - 2, h: n, act: { t: 'isel', i } });
      yy += n - (on && q.acts.length ? 1 : 0);
      if (on && q.acts.length) { let bx = X + 2; q.acts.forEach(([l, k, a]) => { if (bx + L(l) + 6 < X + iw) bx = btn(g, R, bx, yy, T, l, k, l.startsWith('Override') ? 'danger' : 'normal', a); }); yy++; }
      yy++; i++;
    }
  }
  if (s.stage === 'multi' && yy < y + h - 6) { g.putSegs(X, yy++, [{ t: 'Several checkpoints read as one line: failed, then needs you, then next.', fg: T.dim }], {}, X + iw); }
  g.putSegs(X, y + h - 2, hints(T, [['↑↓', 'select'], ['⏎', 'open'], ['r', 're-run'], ['O', 'override'], ['Esc', 'close']]), {}, X + iw);
}

// ---------- modals (Pixls: hairline, accent title strip, hard shadow)
function modal(g, T, W, H, w, h, title) { const x = Math.max(0, Math.floor((W - w) / 2)), y = Math.max(0, Math.floor((H - h) / 2)), K = CS && CS.corners === 'rounded' ? '╰╯' : '└┘'; g.dimAll(); g.fill(x + 1, y + 1, w, h, '#000000'); g.fill(x, y, w, h, T.card); g.hl(x, y + h - 1, w, '─', { fg: T.line, bg: T.card }); g.vl(x, y + 1, h - 2, '│', { fg: T.line, bg: T.card }); g.vl(x + w - 1, y + 1, h - 2, '│', { fg: T.line, bg: T.card }); g.set(x, y + h - 1, K[0], { fg: T.line, bg: T.card }); g.set(x + w - 1, y + h - 1, K[1], { fg: T.line, bg: T.card }); g.fill(x, y, w, 1, T.acc); g.putSegs(x + 2, y, [{ t: title, fg: T.accInk, bg: T.acc, b: 1 }]); g.putSegs(x + w - 11, y, [{ t: 'Esc close', fg: T.accInk, bg: T.acc }]); return { x, y }; }
function field(g, x, y, w, T, v, ph) { g.fill(x, y, w, 1, T.card2); g.putSegs(x + 1, y, [v ? { t: v, fg: T.strong, b: 1, bg: T.card2 } : { t: ph, fg: T.dim, bg: T.card2, i: 1 }, ...(v ? [] : [])], {}, x + w); }
function drawModal(g, R, T, G, s, W, H) {
  const B = T.card, P = (p, segs, y) => g.putSegs(p.x + 2, y, segs.map(t => ({ bg: B, ...t })), {}, p.x + p.w - 2);
  if (s.modal === 'override') {
    const w = Math.min(W - 2, 70), h = 15, p = { ...modal(g, T, W, H, w, h, 'OVERRIDE A FAILED CHECK'), w }; let y = p.y + 2;
    P(p, [{ t: 'fix-footer will read ', fg: T.text }, { t: 'done (overridden)', fg: T.strong, b: 1 }, { t: ' though ', fg: T.text }, { t: 'test failed', fg: T.err, b: 1 }], y++);
    P(p, [{ t: '(exit 1 at 14:22).', fg: T.text }], y++); y++;
    P(p, [{ t: 'This hides a failure. It stays visible:', fg: T.strong, b: 1 }], y++);
    P(p, [{ t: '  ' + G.over + ' on the row, in the Inbox history and in the evidence', fg: T.text }], y++);
    P(p, [{ t: '  for this result only; the next run replaces it', fg: T.text }], y++); y++;
    P(p, [{ t: 'Reason (optional)', fg: T.dim }], y++); field(g, p.x + 2, y++, w - 4, T, s.reason ? 'flaky legal test, tracked in #212' : '', 'why is this OK?'); y++;
    let bx = p.x + 2; bx = btn(g, R, bx, p.y + h - 2, T, 'Override', 'O', 'danger', { t: 'doOverride' }); btn(g, R, bx, p.y + h - 2, T, 'Cancel', 'Esc', 'normal', { t: 'modal', v: null });
  }
  if (s.modal === 'approve') {
    const ch = s.apv === 'changed', cf = s.apv === 'confirm', w = Math.min(W - 2, 84), short = w < 80, h = Math.min(H - 1, 20) - (H < 30 ? 1 : 0);
    const p = { ...modal(g, T, W, H, w, h, ch ? 'THIS REPO CHANGED ITS CHECK' : 'A REPO WANTS TO RUN A COMMAND'), w }; let y = p.y + 2;
    P(p, [{ t: ch ? 'The command changed since you said always:' : 'affiliate-site ships its own check. It has never run here.', fg: T.strong, b: 1 }], y++); y++;
    const cmd = (sign, c, t) => { g.fill(p.x + 2, y, w - 4, 1, T.card2); g.putSegs(p.x + 3, y++, [{ t: sign + ' ', fg: c, b: 1, bg: T.card2 }, { t, fg: T.strong, b: 1, bg: T.card2 }], {}, p.x + w - 2); };
    if (ch) { cmd('-', T.err, 'npm test'); cmd('+', T.ok, 'npm test && node scripts/seed.js'); } else cmd('$', T.dim, 'npm test');
    if (H >= 30) y++;
    P(p, [{ t: pad('Runs in', 9), fg: T.dim }, { t: '~\\dev\\affiliate-site-worktrees\\fix-footer', fg: T.strong }], y++);
    P(p, [{ t: pad('From', 9), fg: T.dim }, { t: '.vibecasso/checks.toml', fg: T.strong }, { t: ch ? ' · changed in 9f3c2a1, 2h ago' : ' · added in a1b2c3d, 3 days ago', fg: T.text }], y++);
    P(p, [{ t: pad('When', 9), fg: T.dim }, { t: 'after every agent turn, with your permissions', fg: T.text }], y++); y++;
    P(p, [{ t: 'Read it. It can do anything you can in that folder.', fg: T.needs, b: 1 }], y++);
    P(p, [{ t: 'Enter and y do nothing here. Choose with 1, 2 or 3.', fg: T.dim }], y++);
    if (cf) { const cb = mix(T.card, T.needs, 0.18); g.fill(p.x + 2, y, w - 4, 1, cb); g.putSegs(p.x + 3, y++, [{ t: 'Press 2 again', fg: T.strong, b: 1, bg: cb }, { t: ' to always run this exact command for affiliate-site.', fg: T.strong, bg: cb }], {}, p.x + w - 2); }
    else y++;
    let bx = p.x + 2, by = p.y + h - 2;
    bx = btn(g, R, bx, by, T, 'Run once', '1', 'normal', { t: 'apv', v: 'once' });
    bx = btn(g, R, bx, by, T, short ? 'Always' : 'Always for this repo', cf ? '2 again' : '2', cf ? 'primary' : 'normal', { t: 'apv', v: 'always' });
    bx = btn(g, R, bx, by, T, short ? 'Never' : 'Never for this repo', '3', 'danger', { t: 'apv', v: 'never' });
    btn(g, R, bx, by, T, 'Not now', 'Esc', 'normal', { t: 'modal', v: null });
  }
  if (s.modal === 'settings') {
    const w = Math.min(W - 2, 80), h = Math.min(H - 2, 20), p = { ...modal(g, T, W, H, w, h, 'PROJECT · affiliate-site'), w }; let y = p.y + 2;
    chipHead(g, y++, p.x + 2, T, 'CHECK', T.btn, null, T.strong); y++;
    if (s.set === 'empty') {
      P(p, [{ t: 'No check set.', fg: T.strong, b: 1 }, { t: ' Vibecasso can run one command itself when an agent\'s', fg: T.text }], y++);
      P(p, [{ t: 'turn ends; the session isn\'t done until it passes. Agents keep working.', fg: T.text }], y++); y++;
      P(p, [{ t: pad('Command', 10), fg: T.dim }], y); field(g, p.x + 12, y++, w - 14, T, '', 'e.g. npm test'); y++;
      P(p, [{ t: 'Projects without a check look exactly as they do today.', fg: T.dim }], y++);
    } else {
      const miss = s.set === 'missing';
      P(p, [{ t: pad('Command', 10), fg: T.dim }], y); field(g, p.x + 12, y++, w - 14, T, miss ? 'pnpm test' : 'npm test'); y++;
      P(p, [{ t: pad('Runs', 10), fg: T.dim }, { t: 'after each turn ends · in the session\'s worktree', fg: T.text }], y++);
      P(p, [{ t: pad('Timeout', 10), fg: T.dim }, { t: '10 min', fg: T.text }, { t: '  ‹ ›', fg: T.dim }], y++);
      if (s.set === 'set') P(p, [{ t: pad('Status', 10), fg: T.dim }, { t: G.done + ' passed on fix-footer 14:22 · 14.2s', fg: T.ok, b: 1 }], y++); else P(p, [{ t: pad('Status', 10), fg: T.dim }, ...(miss ? [{ t: G.couldnt + ' could not run on fix-footer 14:22 · pnpm: not found (exit 127)', fg: T.rev, b: 1 }] : [{ t: G.pend + ' never run', fg: T.strong, b: 1 }, { t: ' · runs after the next turn ends', fg: T.dim }])], y++);
      if (miss) P(p, [{ t: pad('', 10) }, { t: 'Not on the worktree\'s PATH. Install it or change the command.', fg: T.dim }], y++);
      P(p, [{ t: pad('Source', 10), fg: T.dim }, { t: 'you, in settings · no approval needed', fg: T.text }], y++);
    }
    if (s.set !== 'empty') { y++; P(p, [{ t: 'CHECKPOINTS  ', fg: T.dim, b: 1 }, { t: '1 of 1 · room for more kinds later', fg: T.dim }], y++); }
    let bx = p.x + 2, by = p.y + h - 2;
    bx = btn(g, R, bx, by, T, 'Save', 'Ctrl+S', 'primary', { t: 'saveSet' });
    if (s.set !== 'empty') { bx = btn(g, R, bx, by, T, 'Run now', 'r', 'normal', { t: 'rerun' }); bx = btn(g, R, bx, by, T, 'Remove', 'Del', 'danger', { t: 'go', p: '14 Settings · empty' }); }
    btn(g, R, bx, by, T, 'Cancel', 'Esc', 'normal', { t: 'modal', v: null });
  }
}
function drawWhy(g, R, T, G, s, x, y, W) {
  const w = Math.min(60, W - x - 2), h = 8, bg = T.card;
  g.fill(x + 1, y + 1, w, h, '#000000'); g.fill(x, y, w, h, bg); g.fill(x, y, w, 1, T.btn);
  g.putSegs(x + 1, y, [{ t: ' WHY · fix-footer reads failed ', fg: T.strong, bg: T.btn, b: 1 }]);
  const P = (yy, segs) => g.putSegs(x + 2, yy, segs.map(t => ({ bg, ...t })), {}, x + w - 1);
  P(y + 2, [{ t: '14:21:51 ', fg: T.dim }, { t: 'agent\'s turn ended · it said "tests pass"', fg: T.text }]);
  P(y + 3, [{ t: '14:22:05 ', fg: T.dim }, { t: 'Vibecasso ran ', fg: T.text }, { t: 'npm test', fg: T.strong, b: 1 }, { t: ' in its worktree', fg: T.text }]);
  P(y + 4, [{ t: '14:22:19 ', fg: T.dim }, { t: G.failed + ' exit 1 after 14.2s · 1 of 59 tests failed', fg: T.err, b: 1 }]);
  P(y + 6, hints(T, [['⏎', 'evidence'], ['r', 're-run'], ['O', 'override'], ['Esc', 'close']]));
  R.push({ x, y, w, h, act: { t: 'sheet', v: 'evidence' } });
}

// ---------- main render
export function render(s) {
  CS = s; const T = THEMES[s.theme] || THEMES.pixls, G = GL[s.glyphs] || GL.unicode, W = s.W, H = s.H, g = new Grid(W, H, T), R = [];
  const F = sessions(T, G, s), all = F.flatMap(f => f.list), cur = all[0];
  const B = bordered(s), gut = s.layout === 'floating' ? 1 : 0, ov = B ? 0 : 1;
  const SW = W >= 200 ? 40 : W >= 140 ? 34 : 26, sheetOn = !!s.sheet, beside = sheetOn && W >= 140, SHW = 72;
  drawSide(g, R, T, G, s, F, 0, 0, SW, H);
  const px = SW + gut - ov, pw = W - px, paneW = beside ? pw - SHW - gut + ov : pw, sx = beside ? px + paneW + gut - ov : px;
  if (!(sheetOn && !beside)) {
    const lit = s.focus === 'pane', k = cur.k;
    panel(g, T, s, px, 0, paneW, H, lit, null, [{ t: glyph(G, k, s) + ' ' + WORD[k], fg: col(T, k), b: 1 }, { t: '  ' + G.close + ' ', fg: T.dim }]);
    let tx = px + (B ? 2 : 1); tx = g.putSegs(tx, 0, [{ t: ' 1 ', fg: T.accInk, bg: T.acc, b: 1 }, { t: ' fix-footer ', fg: lit ? T.accInk : T.strong, bg: lit ? T.acc : T.btn, b: 1 }]); g.putSegs(tx, 0, [{ t: ' 2 ', fg: T.text, bg: T.btn }, { t: ' add-payments ', fg: T.text, bg: T.card }]);
    drawTerm(g, T, px + 3, 2, paneW - 6, H - 3);
    R.push({ x: px + 1, y: 1, w: paneW - 2, h: H - 2, act: { t: 'focus', v: 'pane' } });
  }
  if (s.sheet === 'evidence') drawEvidence(g, R, T, G, s, sx, 0, beside ? SHW : pw, H);
  if (s.sheet === 'inbox') drawInbox(g, R, T, G, s, sx, 0, beside ? SHW : pw, H);
  if (s.pop) drawWhy(g, R, T, G, s, Math.min(SW + 2, W - 40), H < 30 ? 3 : 5, W);
  if (s.modal) drawModal(g, R, T, G, s, W, H);
  if (s.toast) { const t = trunc(s.toast, W - 6), x = Math.max(1, W - L(t) - 4); g.fill(x, H - 2, L(t) + 2, 1, T.btn); g.put(x + 1, H - 2, t, { fg: T.strong, bg: T.btn }); }
  if (s.leader) { const lb = ' Ctrl+Space '; g.putSegs(W - L(lb) - 1, 0, [{ t: lb, fg: T.bg, bg: T.sky, b: 1 }]); }
  // hover: tint the region under the pointer (every chip/row is clickable)
  if (s.hx != null) { for (let k = R.length - 1; k >= 0; k--) { const q = R[k]; if (s.hx >= q.x && s.hx < q.x + q.w && s.hy >= q.y && s.hy < q.y + q.h) { for (let yy = q.y; yy < q.y + q.h; yy++) for (let xx = q.x; xx < q.x + q.w; xx++) { const c = g.c[yy] && g.c[yy][xx]; if (c) c.bg = mix(c.bg, T.text, 0.1); } break; } } }
  return { rows: g.rows(), regions: R, W, H };
}

// ---------- actions
export function act(s, a) {
  const S = (p, fx) => ({ state: { ...s, toast: null, ...p }, fx });
  const toast = v => ({ state: { ...s, toast: v }, fx: [{ after: 2600, act: { t: 'clearToast' } }] });
  switch (a.t) {
    case 'clearToast': return S({});
    case 'toast': return toast(a.v);
    case 'go': return S(preset(a.p, s.W, s.H, s));
    case 'leader': return S({ leader: !s.leader });
    case 'selSess': return S({ sel: a.i, focus: 'side', pop: 0 });
    case 'focus': return S({ focus: a.v, pop: 0 });
    case 'sheet': return S({ sheet: a.v, focus: a.v ? 'sheet' : 'side', pop: 0, modal: null, isel: 0 });
    case 'isel': return S({ isel: a.i, focus: 'sheet' });
    case 'apv': {
      if (a.v === 'always' && s.apv !== 'confirm') return S({ apv: 'confirm' });
      if (a.v === 'never') return S({ modal: null, stage: 'never', sheet: 'evidence', focus: 'sheet', toast: 'Never: this repo\'s check will not run. Change it in Settings › Check.' });
      return { state: { ...s, modal: null, sheet: null, stage: 'checking', focus: 'side', toast: a.v === 'always' ? 'Always for affiliate-site. Asks again if the command changes.' : 'Running once. Asks again next turn.' }, fx: [{ after: 2600, act: { t: 'result', v: 'failed' } }] };
    }
    case 'modal': return S({ modal: a.v, apv: a.apv || s.apv, set: a.set || s.set, focus: a.v ? 'modal' : s.sheet ? 'sheet' : 'side', pop: 0 });
    case 'doOverride': return S({ modal: null, stage: 'over', sheet: 'evidence', focus: 'sheet', toast: 'Overridden. fix-footer reads done (overridden); the failure stays in its evidence.' });
    case 'approve': return { state: { ...s, modal: null, stage: 'checking', focus: 'side', toast: 'Allowed npm test for this repo. Checking fix-footer…' }, fx: [{ after: 2600, act: { t: 'result', v: 'failed' } }] };
    case 'rerun': return { state: { ...s, modal: null, pop: 0, stage: 'checking', sheet: null, focus: 'side', toast: 'Re-running npm test in fix-footer\'s worktree…' }, fx: [{ after: 2600, act: { t: 'result', v: 'passed' } }] };
    case 'result': return S({ stage: a.v, toast: a.v === 'passed' ? 'test passed (exit 0, 13.9s). fix-footer reads done.' : 'test failed (exit 1). fix-footer is in the Inbox.' });
    case 'saveSet': return S({ modal: null, stage: s.stage === 'none' ? 'none' : s.stage, focus: 'side', toast: s.set === 'empty' ? 'Saved. Runs after the next turn ends.' : 'Saved.' });
    case 'key': {
      const k = a.key;
      if (k === 'a' && s.focus === 'side' && !s.leader && !s.modal && !s.pop) return toast('Row menu (built): Re-run check · Override… · Undo override');
      if (s.leader) { const p = { leader: false }; if (k === 'j') return act({ ...s, ...p }, { t: 'sheet', v: 'inbox' }); if (k === ',') return act({ ...s, ...p }, { t: 'modal', v: 'settings', set: s.stage === 'none' ? 'empty' : s.stage === 'couldnt' ? 'missing' : 'never' }); return S(p); }
      if (s.modal) { if (k === 'Escape') return act(s, { t: 'modal', v: null }); if (s.modal === 'override' && k === 'O') return act(s, { t: 'doOverride' }); if (s.modal === 'approve') { if (k === '1') return act(s, { t: 'apv', v: 'once' }); if (k === '2') return act(s, { t: 'apv', v: 'always' }); if (k === '3') return act(s, { t: 'apv', v: 'never' }); if (k === 'Enter' || k === 'y' || k === ' ') return { state: { ...s, toast: 'Enter and y do nothing here. Choose 1, 2 or 3.' }, fx: [{ after: 2200, act: { t: 'clearToast' } }] }; return null; } if (s.modal === 'settings' && k === 'r' && s.set !== 'empty') return act(s, { t: 'rerun' }); return null; }
      if (s.pop) { if (k === 'Escape') return S({ pop: 0 }); if (k === 'Enter') return act(s, { t: 'sheet', v: 'evidence' }); if (k === 'r') return act(s, { t: 'rerun' }); if (k === 'O') return act(s, { t: 'modal', v: 'override' }); return null; }
      const failed = s.stage === 'failed' || s.stage === 'multi';
      if (s.focus === 'sheet') {
        if (k === 'Escape') return act(s, { t: 'sheet', v: null });
        if (k === 'r') return act(s, { t: 'rerun' });
        if (k === 'O' && failed) return act(s, { t: 'modal', v: 'override' });
        if (k === 'u' && s.stage === 'over') return act(s, { t: 'go', p: '07 Evidence · short output' });
        if (k === 'f') return toast('Full output opens in the pager (1,284 lines)');
        if (s.sheet === 'inbox') { const n = inboxItems(THEMES[s.theme], GL[s.glyphs], s).length; if (k === 'ArrowDown' || k === 'j') return S({ isel: Math.min(n - 1, s.isel + 1) }); if (k === 'ArrowUp' || k === 'k') return S({ isel: Math.max(0, s.isel - 1) }); if (k === 'Enter') { const it = inboxItems(THEMES[s.theme], GL[s.glyphs], s)[s.isel]; return it && it.acts[0] ? act(s, it.acts[0][2]) : null; } }
        return null;
      }
      if (s.focus === 'side') {
        if (k === 'ArrowDown' || k === 'j') return S({ sel: Math.min(4, s.sel + 1) }); if (k === 'ArrowUp' || k === 'k') return S({ sel: Math.max(0, s.sel - 1) });
        if (k === 'i' && s.sel === 0 && s.stage !== 'none') return S({ pop: 1 });
        if (k === 'Enter') return s.sel === 0 && s.stage !== 'none' && s.stage !== 'checking' ? act(s, { t: 'sheet', v: 'evidence' }) : S({ focus: 'pane' });
        if (k === 'r' && s.sel === 0 && s.stage !== 'none') return act(s, { t: 'rerun' });
        if (k === 'O' && s.sel === 0 && failed) return act(s, { t: 'modal', v: 'override' });
      }
      return null;
    }
  }
  return null;
}
