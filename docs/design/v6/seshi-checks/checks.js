// Seshi · checkpoints Seshi checks itself (design brief v6). Cell-grid spec. SAMPLE DATA ONLY.
import { Grid, trunc, segLen, L, mix, THEMES as GT } from '../seshi-v6/seshi-grid.js';
import { THEMES as PT } from '../seshi-v6/app.js';
const lt = GT.light;
export const THEMES = { pixls: PT.pixls, night: PT.night,
  paper: { name: 'light', label: 'Seshi Paper', bg: lt.bg, fg: lt.fg, card: lt.surf, card2: lt.surf2, btn: lt.btn, line: lt.line, dim: lt.dim, text: lt.text, strong: lt.strong, acc: lt.acc, accInk: '#ffffff', needs: lt.needs, work: lt.work, ok: lt.ok, err: lt.err, rev: lt.ws.violet, sky: lt.ws.sky, wait: lt.ws.sky, scrim: lt.scrim, a: lt.a } };

const bl = [0, 0x40, 0x44, 0x46, 0x47], brr = [0, 0x80, 0xA0, 0xB0, 0xB8];
const BARS = [[1, 3, 2], [2, 4, 1], [3, 3, 3], [4, 2, 4], [3, 1, 3], [2, 2, 2], [1, 4, 3], [2, 3, 4], [4, 1, 2], [3, 2, 1]].map(([a, b, c]) => String.fromCharCode(0x2800 + bl[a] + brr[b]) + String.fromCharCode(0x2800 + bl[c]));
const GL = {
  unicode: { work: BARS, check: ['◜', '◝', '◞', '◟'], needs: '●', done: '✓', failed: '✕', over: '⊘', couldnt: '?', stale: '◷', idle: '○', asleep: 'z', pend: '…', bar: '▎', branch: '⎇', close: '✕', fold: '▾', warn: '⚠', arrow: '›' },
  ascii: { work: ['. ', 'o ', 'O ', 'o '], check: ['-', '\\', '|', '/'], needs: '!', done: '+', failed: 'x', over: '/', couldnt: '?', stale: '~', idle: 'o', asleep: 'z', pend: '.', bar: '|', branch: 'git:', close: 'x', fold: 'v', warn: '!', arrow: '>' },
};
GL.nerd = { ...GL.unicode, needs: '\uf0f3', done: '\uf00c', failed: '\uf00d', over: '\uf05e', couldnt: '\uf128', stale: '\uf017', idle: '\uf10c', branch: '\ue0a0', close: '\uf00d', warn: '\uf071' };

// session state → glyph / colour / word. Shape + word always carry the meaning; colour only reinforces it.
const glyph = (G, k, s) => k === 'working' ? G.work[s.motion ? s.f % G.work.length : 2] : k === 'checking' ? G.check[s.motion ? (s.f >> 1) % 4 : 0] : ({ needs: G.needs, approve: G.needs, done: G.done, passed: G.done, stale: G.stale, failed: G.failed, over: G.over, couldnt: G.couldnt, idle: G.idle, asleep: G.asleep })[k] || '·';
const col = (T, k) => ({ working: T.acc, checking: T.sky, needs: T.needs, approve: T.needs, done: T.ok, passed: T.ok, stale: T.ok, failed: T.err, over: T.text, couldnt: T.rev, idle: T.dim, asleep: T.dim })[k] || T.text;
function shimmer(t, s, T, extra = {}) {
  const a = Array.from(t), n = a.length, base = mix(T.acc, T.bg, 0.2), hi = mix(T.acc, '#ffffff', 0.55), pos = s.motion ? (s.f % (n + 8)) - 4 : -99;
  return a.map((ch, i) => { const d = Math.abs(i - pos); return { t: ch, fg: d < 1 ? hi : d < 2.5 ? mix(hi, base, 0.5) : base, b: 1, ...extra }; });
}
const WORD = { working: 'working', checking: 'checking', needs: 'needs you', approve: 'needs you', done: 'done', passed: 'done', stale: 'done · stale', failed: 'failed', over: 'done · overridden', couldnt: 'not checked', idle: 'idle', asleep: 'asleep' };
const crG = (G, r, s) => r === 'pass' ? G.done : r === 'fail' ? G.failed : r === 'run' ? G.check[s.motion ? (s.f >> 1) % 4 : 0] : r === 'err' ? G.couldnt : r === 'stale' ? G.stale : r === 'over' ? G.over : G.pend;
const crC = (T, r) => ({ pass: T.ok, fail: T.err, run: T.sky, err: T.rev, stale: T.ok, over: T.text })[r] || T.dim;

export const PRESETS = [
  ['01 No check set', { stage: 'none' }],
  ['02 Settings: empty', { stage: 'none', modal: 'settings', set: 'empty' }],
  ['03 Settings: never run', { stage: 'none', modal: 'settings', set: 'never' }],
  ['04 Approve repo command', { stage: 'approve', modal: 'approve' }],
  ['05 Checking', { stage: 'checking' }],
  ['06 Passed', { stage: 'passed' }],
  ['07 Failed', { stage: 'failed' }],
  ['08 Why? (i)', { stage: 'failed', pop: 1 }],
  ['09 Evidence', { stage: 'failed', sheet: 'evidence' }],
  ['10 Long output', { stage: 'failed', sheet: 'evidence', out: 'long' }],
  ['11 Printed nothing', { stage: 'passed', sheet: 'evidence', out: 'empty' }],
  ['12 Inbox', { stage: 'failed', sheet: 'inbox' }],
  ['13 Override?', { stage: 'failed', sheet: 'inbox', modal: 'override' }],
  ['14 Overridden', { stage: 'over', sheet: 'evidence' }],
  ['15 Could not run', { stage: 'couldnt', sheet: 'evidence' }],
  ['16 Settings: missing cmd', { stage: 'couldnt', modal: 'settings', set: 'missing' }],
  ['17 Stale result', { stage: 'stale', sheet: 'evidence' }],
  ['18 Room for more', { stage: 'multi', sheet: 'inbox' }],
];
export function preset(name, W, H, fl = {}) {
  const p = (PRESETS.find(x => x[0] === name) || PRESETS[0])[1];
  return { W, H, theme: fl.theme || 'pixls', glyphs: fl.glyphs || 'unicode', style: fl.style || 'flush', motion: fl.motion !== false, f: 0, stage: 'none', sheet: null, modal: null, set: null, out: 'normal', pop: 0, sel: 0, isel: 0, toast: null, leader: false, reason: 1, ...p, focus: p.modal ? 'modal' : p.sheet ? 'sheet' : 'side' };
}

function sessions(T, G, s) {
  const st = s.stage, C = (n, r, t) => ({ n, r, t });
  const ff = { id: 'fix-footer', agent: 'codex', age: '2m' };
  const set = (k, checks, sub) => Object.assign(ff, { k, checks, sub });
  if (st === 'none') set('done', [], [{ t: 'finished · not seen', fg: T.dim }]);
  if (st === 'approve') set('approve', [C('test', 'pend')], [{ t: 'approve check: npm test', fg: T.needs }]);
  if (st === 'checking') set('checking', [C('test', 'run', '6s')], [{ t: 'checking test · 6s', fg: T.sky }]);
  if (st === 'passed') set('passed', [C('test', 'pass', '14s')], [{ t: 'test ', fg: T.text }, { t: G.done, fg: T.ok, b: 1 }, { t: ' · 14.2s · 14:22', fg: T.dim }]);
  if (st === 'failed') set('failed', [C('test', 'fail', 'exit 1')], [{ t: 'failed: test · exit 1', fg: T.err, b: 1 }]);
  if (st === 'over') set('over', [C('test', 'over')], [{ t: 'test ' + G.failed + ' overridden by you', fg: T.text }]);
  if (st === 'couldnt') set('couldnt', [C('test', 'err')], [{ t: 'test could not run · not found', fg: T.rev }]);
  if (st === 'stale') set('stale', [C('test', 'stale')], [{ t: 'test ' + G.done + ' · stale: 3 files changed', fg: T.text }]);
  if (st === 'multi') set('failed', [C('build', 'pass'), C('test', 'fail'), C('review', 'pend')], null);
  const ap = st === 'multi' ? { id: 'add-payments', agent: 'claude', age: '40s', k: 'checking', checks: [C('build', 'pass'), C('test', 'run'), C('review', 'pend')] } : { id: 'add-payments', agent: 'claude', age: 'now', k: 'working', checks: [], sub: [{ t: 'claude · ', fg: T.dim }, ...shimmer('working', s, T)] };
  const hasCheck = st !== 'none';
  return [
    { name: 'affiliate-site', meta: G.branch + ' main' + (hasCheck ? (st === 'multi' ? ' · 3 checks' : ' · check: test') : ''), list: [ff, ap] },
    { name: 'pixls', meta: 'no git', list: [{ id: 'warm-finch', agent: 'codex', age: '3m', k: 'needs', checks: [], sub: [{ t: 'asks: use pnpm or npm?', fg: T.needs }] }] },
    { name: 'seshi', meta: G.branch + ' dev', list: [{ id: 'quick-fox', agent: 'claude', age: '9h', k: 'done', checks: [], sub: [{ t: 'finished', fg: T.dim }] }, { id: 'old-moth', agent: 'codex', age: '2d', k: 'asleep', checks: [], sub: [{ t: 'asleep zzZ', fg: T.dim }] }] },
  ];
}
function chips(T, G, s, checks, narrow) {
  if (narrow) { const bad = checks.find(c => c.r === 'fail' || c.r === 'err'); return [...checks.map(c => ({ t: crG(G, c.r, s), fg: crC(T, c.r), b: 1 })), ...(bad ? [{ t: ' ' + bad.n, fg: crC(T, bad.r) }] : [])]; }
  return checks.flatMap((c, i) => [...(i ? [{ t: ' · ', fg: T.dim }] : []), { t: c.n + ' ', fg: c.r === 'fail' ? T.err : T.text }, { t: crG(G, c.r, s), fg: crC(T, c.r), b: 1 }]);
}

// ---------- panels: flush (shared edges), rounded cards, flat
function panel(g, T, s, x, y, w, h, lit, title, right, flatBg) {
  const cb = lit ? T.acc : T.btn, cf = lit ? T.accInk : T.strong;
  const chip = title ? [{ t: ' ', bg: cb }, ...title.map(t => ({ ...t, fg: lit ? cf : (t.fg === T.dim ? T.text : t.fg || T.strong), bg: cb })), { t: ' ', bg: cb }] : null;
  if (s.style === 'rounded') {
    const c = lit ? T.acc : T.line, S = { fg: c };
    g.set(x, y, '╭', S); g.set(x + w - 1, y, '╮', S); g.set(x, y + h - 1, '╰', S); g.set(x + w - 1, y + h - 1, '╯', S);
    g.hl(x + 1, y, w - 2, '─', S); g.hl(x + 1, y + h - 1, w - 2, '─', S); g.vl(x, y + 1, h - 2, '│', S); g.vl(x + w - 1, y + 1, h - 2, '│', S);
    if (chip) g.putSegs(x + 2, y, chip); if (right) g.putSegs(x + w - 2 - segLen(right) - 1, y, [{ t: ' ' }, ...right, { t: ' ' }]);
    return;
  }
  const hb = s.style === 'flat' ? mix(T.card, T.text, 0.05) : T.card;
  if (s.style === 'flat' && flatBg) g.fill(x, y + 1, w, h - 1, flatBg);
  g.fill(x, y, w, 1, hb);
  if (s.style === 'flush' && x > 0) { g.set(x, y, '│', { fg: T.line, bg: hb }); g.vl(x, y + 1, h - 1, '│', { fg: lit ? T.acc : T.line }); }
  if (chip) g.putSegs(x + 1, y, chip);
  if (right) { const rs = right.map(t => ({ ...t, bg: hb })); g.putSegs(x + w - 1 - segLen(rs), y, rs); }
}
const btn = (g, R, x, y, T, label, key, st, act) => { const P = st === 'primary' ? [T.acc, T.accInk, T.accInk] : st === 'danger' ? [T.btn, T.err, T.acc] : [T.btn, T.strong, T.acc]; const x2 = g.putSegs(x, y, [{ t: ' ' + label + ' ', bg: P[0], fg: P[1], b: 1 }, ...(key ? [{ t: key + ' ', bg: P[0], fg: P[2], b: 1 }] : [])]); R.push({ x, y, w: x2 - x, h: 1, act }); return x2 + 1; };
const hints = (T, list) => list.flatMap((h, i) => [...(i ? [{ t: '   ' }] : []), { t: h[0], fg: T.acc, b: 1 }, { t: ' ' + h[1], fg: T.text }]);
const pad = (s, n) => { const a = Array.from(s); return a.length >= n ? trunc(s, n) : s + ' '.repeat(n - a.length); };
function chipHead(g, y, x, T, label, c, n, ink) { return g.putSegs(x, y, [{ t: ' ' + label + ' ', fg: ink, bg: c, b: 1 }, ...(n != null ? [{ t: ' ' + n + ' ', fg: ink, bg: mix(c, T.bg, 0.25), b: 1 }] : [])]); }

// ---------- sidebar
function drawSide(g, R, T, G, s, F, x, y, w, h) {
  const lit = s.focus === 'side', tint = mix(T.bg, T.text, 0.12), bodyBg = s.style === 'flat' ? T.card : undefined;
  const all = F.flatMap(f => f.list), nNeeds = all.filter(q => q.k === 'needs' || q.k === 'approve').length, nFail = all.filter(q => q.k === 'failed').length;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'SESSIONS', b: 1 }, { t: ' ' + all.length, fg: T.dim }], [...(nNeeds ? [{ t: G.needs + nNeeds, fg: T.needs, b: 1 }] : []), ...(nFail ? [{ t: (nNeeds ? ' ' : '') + G.failed + nFail, fg: T.err, b: 1 }] : [])], bodyBg);
  let yy = y + 2, idx = 0; const narrow = w < 32, X = x + (s.style === 'rounded' ? 2 : 1);
  for (const f of F) {
    const room = w - (X - x) - 6 - L(f.name), meta = room >= 6 ? trunc(f.meta, room) : '';
    g.putSegs(X + 1, yy, [{ t: G.fold + ' ', fg: T.acc }, { t: trunc(f.name, w - 6), fg: T.strong, b: 1 }]); if (meta) g.put(x + w - 2 - L(meta), yy, meta, { fg: T.dim }); yy++;
    for (const q of f.list) {
      const on = idx === s.sel, bg = on ? tint : bodyBg, fail = q.k === 'failed';
      if (on) g.fill(X, yy, w - (X - x) - 1, 2, tint);
      if (on || fail) for (let j = 0; j < 2; j++) g.set(X, yy + j, G.bar, { fg: fail ? T.err : lit ? T.acc : T.dim, bg, b: 1 });
      const nm = trunc(q.id, w - 10 - L(q.age));
      g.putSegs(X + 2, yy, [{ t: glyph(G, q.k, s) + ' ', fg: col(T, q.k), bg, b: fail || q.k === 'needs' || q.k === 'approve' || q.k === 'working' }, ...(q.k === 'working' ? shimmer(nm, s, T, { bg }) : [{ t: nm, fg: on || fail ? T.strong : T.text, b: 1, bg }])]);
      g.put(x + w - 2 - L(q.age), yy, q.age, { fg: T.dim, bg });
      const full = q.checks.length > 1 ? [{ t: q.agent + ' · ', fg: T.dim }, ...chips(T, G, s, q.checks, false)] : null;
      const sub = full ? (segLen(full) <= w - (X - x) - 6 ? full : chips(T, G, s, q.checks, true)) : q.sub || [];
      g.putSegs(X + 4, yy + 1, sub.map(t => ({ ...t, bg })), {}, x + w - 2);
      R.push({ x: X, y: yy, w: w - (X - x) - 1, h: 2, act: { t: 'selSess', i: idx } });
      yy += 2; idx++;
    }
    yy++;
  }
  const by = y + h - (s.style === 'rounded' ? 2 : 1);
  let bx = X + 1; bx = g.putSegs(bx, by, [{ t: '+', fg: T.acc, b: 1 }, { t: ' new', fg: T.text }]) + 3;
  const ib = bx; bx = g.putSegs(bx, by, [{ t: 'j', fg: T.acc, b: 1 }, { t: ' inbox', fg: T.text }]); R.push({ x: ib, y: by, w: bx - ib, h: 1, act: { t: 'sheet', v: 'inbox' } }); bx += 3;
  if (bx + 10 < x + w) { const sb = bx; bx = g.putSegs(bx, by, [{ t: ',', fg: T.acc, b: 1 }, { t: ' settings', fg: T.text }]); R.push({ x: sb, y: by, w: bx - sb, h: 1, act: { t: 'modal', v: 'settings', set: s.stage === 'none' ? 'empty' : s.stage === 'couldnt' ? 'missing' : 'never' } }); }
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
  const lit = s.focus === 'sheet', X = x + 2, iw = w - 4, st = s.stage, bodyBg = s.style === 'flat' ? T.card : undefined;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'CHECK', b: 1 }, { t: ' · fix-footer › test', fg: T.dim }], [{ t: G.close + ' ', fg: T.dim }], bodyBg);
  R.push({ x: x + w - 3, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 2;
  const kv = (k, v) => { g.putSegs(X, yy++, [{ t: pad(k, 9), fg: T.dim }, ...v], {}, X + iw); };
  const res = { failed: [G.failed + ' failed', T.err, 'exit 1'], over: [G.over + ' failed · overridden', T.text, 'exit 1'], couldnt: [G.couldnt + ' could not run', T.rev, 'exit 127'], passed: [G.done + ' passed', T.ok, 'exit 0'], stale: [G.done + ' passed · ' + G.stale + ' stale', T.ok, 'exit 0'], multi: [G.failed + ' failed', T.err, 'exit 1'] }[st] || [G.done + ' passed', T.ok, 'exit 0'];
  g.putSegs(X, yy++, [{ t: ' ' + res[0] + ' ', fg: T.bg, bg: res[1], b: 1 }, { t: '  ' + res[2], fg: T.strong, b: 1 }, { t: st === 'couldnt' ? '  command not found' : '', fg: T.rev }]); yy++;
  kv('Command', [{ t: st === 'couldnt' ? 'pnpm test' : 'npm test', fg: T.strong, b: 1 }]);
  kv('Folder', [{ t: '~\\dev\\affiliate-site-worktrees\\fix-footer', fg: T.text }]);
  kv('Ran', [{ t: '14:22:05', fg: T.text }, { t: ' · took ', fg: T.dim }, { t: st === 'couldnt' ? '0.1s' : '14.2s', fg: T.text }, { t: ' · after turn 3 ended 14:21:51', fg: T.dim }]);
  kv('Seen by', [{ t: 'Seshi ran this itself. The agent said "tests pass".', fg: T.dim }]);
  yy++;
  if (st === 'over') { g.fill(X, yy, iw, 2, mix(T.bg, T.text, 0.1)); g.putSegs(X + 1, yy++, [{ t: G.over + ' Overridden by you 14:30', fg: T.strong, b: 1 }, { t: '  · u undo', fg: T.dim }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'reason: flaky legal test, tracked in #212 · next run replaces this', fg: T.text }], {}, X + iw); yy++; }
  if (st === 'stale') { g.fill(X, yy, iw, 2, mix(T.bg, T.text, 0.1)); g.putSegs(X + 1, yy++, [{ t: G.stale + ' Stale', fg: T.strong, b: 1 }, { t: ': the agent changed 3 files after this ran (14:24)', fg: T.text }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'This result is about older code. r re-runs; the row says stale until then.', fg: T.dim }], {}, X + iw); yy++; }
  if (st === 'couldnt') { g.fill(X, yy, iw, 2, mix(T.bg, T.rev, 0.12)); g.putSegs(X + 1, yy++, [{ t: G.couldnt + ' Not a test failure.', fg: T.rev, b: 1 }, { t: ' The work hasn\'t been checked.', fg: T.text }], {}, X + iw); g.putSegs(X + 3, yy++, [{ t: 'Fix the command in settings (,) or install pnpm, then r.', fg: T.dim }], {}, X + iw); yy++; }
  const out = st === 'couldnt' ? [['sh: 1: pnpm: not found', 'err']] : s.out === 'empty' ? [] : st === 'passed' || st === 'stale' ? OUT_PASS : OUT_FAIL;
  const total = s.out === 'long' ? 1284 : out.length, room = Math.max(3, y + h - 4 - yy - 1);
  chipHead(g, yy, X, T, 'OUTPUT', T.btn, null, T.strong);
  g.putSegs(X + 9, yy++, [{ t: s.out === 'empty' ? 'empty' : out.length > room || s.out === 'long' ? 'last ' + Math.min(room, out.length) + ' of ' + total.toLocaleString() + ' lines' : 'all ' + out.length + ' lines', fg: T.dim }, ...(s.out === 'long' ? [{ t: '  f full output', fg: T.dim }] : [])]);
  const ob = mix(T.bg, T.scrim, 0.25), oh = Math.min(room, Math.max(out.length, 3));
  g.fill(X, yy, iw, oh, ob);
  if (!out.length) { g.put(X + 2, yy + 1, '(the command printed nothing)', { fg: T.dim, bg: ob, i: 1 }); g.put(X + 2, yy + 2, trunc('exit 0 is what passed it.', iw - 3), { fg: T.dim, bg: ob }); }
  const shown = out.slice(Math.max(0, out.length - oh)), first = total - shown.length + 1;
  shown.forEach(([t, k], i) => { g.putSegs(X, yy + i, [{ t: pad(String(first + i), 5), fg: mix(T.dim, ob, 0.35), bg: ob }, { t: trunc(t, iw - 6), fg: k === 'err' ? T.err : k === 'ok' ? T.ok : k === 'd' ? T.dim : T.text, bg: ob }], {}, X + iw); });
  if (s.out === 'long') g.set(X + iw - 1, yy, '▲', { fg: T.dim, bg: ob });
  const by = y + h - 3; let bx = X;
  bx = btn(g, R, bx, by, T, 'Re-run', 'r', 'normal', { t: 'rerun' });
  if (st === 'failed' || st === 'multi') bx = btn(g, R, bx, by, T, 'Override…', 'O', 'danger', { t: 'modal', v: 'override' });
  if (st === 'over') bx = btn(g, R, bx, by, T, 'Undo override', 'u', 'normal', { t: 'go', p: '09 Evidence' });
  if (bx + 18 < X + iw) bx = btn(g, R, bx, by, T, 'Open session', 'o', 'normal', { t: 'sheet', v: null });
  g.putSegs(X, y + h - 2, hints(T, [['Esc', 'back'], ['r', 're-run'], ['f', 'full output'], ['i', 'why']]), {}, X + iw);
}

// ---------- inbox (sheet): needs you first, then failed checks, then finished
function inboxItems(T, G, s) {
  const st = s.stage, it = [];
  it.push({ grp: 'needs', id: 'warm-finch', proj: 'pixls', age: '3m', k: 'needs', l2: [{ t: 'Use pnpm or npm for the workspace?', fg: T.needs }], acts: [['Reply', 'm', { t: 'toast', v: 'Follow-up popover (existing m)' }], ['Open', 'o', { t: 'sheet', v: null }]] });
  if (st === 'approve') it.push({ grp: 'needs', id: 'fix-footer', proj: 'affiliate-site', age: '1m', k: 'approve', l2: [{ t: 'approve check command: npm test', fg: T.needs }], acts: [['Review', '⏎', { t: 'modal', v: 'approve' }]] });
  if (st === 'failed' || st === 'multi') it.push({ grp: 'failed', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: 'failed', l2: st === 'multi' ? [{ t: 'failed: test · ', fg: T.err, b: 1 }, ...chips(T, G, s, [{ n: 'build', r: 'pass' }, { n: 'test', r: 'fail' }, { n: 'review', r: 'pend' }])] : [{ t: 'failed: test · exit 1 · 14:22', fg: T.err, b: 1 }], l3: [{ t: '✗ legal.test.tsx › footer shows current year', fg: T.dim }], acts: [['Output', '⏎', { t: 'sheet', v: 'evidence' }], ['Re-run', 'r', { t: 'rerun' }], ['Override…', 'O', { t: 'modal', v: 'override' }]] });
  if (st === 'couldnt') it.push({ grp: 'couldnt', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: 'couldnt', l2: [{ t: 'test could not run: pnpm not found', fg: T.rev }], acts: [['Output', '⏎', { t: 'sheet', v: 'evidence' }], ['Settings', ',', { t: 'modal', v: 'settings', set: 'missing' }]] });
  if (st === 'passed' || st === 'stale' || st === 'over' || st === 'none') it.push({ grp: 'done', id: 'fix-footer', proj: 'affiliate-site', age: '2m', k: st === 'none' ? 'done' : st, l2: st === 'over' ? [{ t: G.over + ' done · test failed, overridden', fg: T.text }] : st === 'stale' ? [{ t: 'test ' + G.done + ' · ' + G.stale + ' stale', fg: T.text }] : st === 'none' ? [{ t: 'finished', fg: T.dim }] : [{ t: 'test ' + G.done + ' 14.2s', fg: T.text }], acts: [] });
  it.push({ grp: 'done', id: 'quick-fox', proj: 'seshi', age: '9h', k: 'done', l2: [{ t: 'finished · not seen', fg: T.dim }], acts: [] });
  return it;
}
function drawInbox(g, R, T, G, s, x, y, w, h) {
  const lit = s.focus === 'sheet', X = x + 2, iw = w - 4, items = inboxItems(T, G, s), tint = mix(T.bg, T.text, 0.12), bodyBg = s.style === 'flat' ? T.card : undefined;
  panel(g, T, s, x, y, w, h, lit, [{ t: 'INBOX', b: 1 }, { t: ' ' + items.filter(i => i.grp !== 'done').length, fg: T.dim }], [{ t: G.close + ' ', fg: T.dim }], bodyBg);
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
  if (s.stage === 'multi' && yy < y + h - 6) { g.putSegs(X, yy++, [{ t: 'Several checkpoints read as one line: the first failing one is named.', fg: T.dim }], {}, X + iw); }
  g.putSegs(X, y + h - 2, hints(T, [['↑↓', 'select'], ['⏎', 'open'], ['r', 're-run'], ['O', 'override'], ['Esc', 'close']]), {}, X + iw);
}

// ---------- modals (Pixls: hairline, accent title strip, hard shadow)
function modal(g, T, W, H, w, h, title) { const x = Math.max(0, Math.floor((W - w) / 2)), y = Math.max(1, Math.floor((H - h) / 2)); g.dimAll(); g.fill(x + 1, y + 1, w, h, '#000000'); g.fill(x, y, w, h, T.card); g.hl(x, y + h - 1, w, '─', { fg: T.line, bg: T.card }); g.vl(x, y + 1, h - 2, '│', { fg: T.line, bg: T.card }); g.vl(x + w - 1, y + 1, h - 2, '│', { fg: T.line, bg: T.card }); g.fill(x, y, w, 1, T.acc); g.putSegs(x + 2, y, [{ t: title, fg: T.accInk, bg: T.acc, b: 1 }]); g.putSegs(x + w - 11, y, [{ t: 'Esc close', fg: T.accInk, bg: T.acc }]); return { x, y }; }
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
    const w = Math.min(W - 2, 74), h = 16, p = { ...modal(g, T, W, H, w, h, 'RUN A CHECK FROM THIS REPO?'), w }; let y = p.y + 2;
    P(p, [{ t: 'This command came from the repo, not from you:', fg: T.text }], y++); y++;
    field(g, p.x + 2, y++, w - 4, T, '$ npm test'); y++;
    P(p, [{ t: pad('In', 9), fg: T.dim }, { t: '~\\dev\\affiliate-site-worktrees\\fix-footer', fg: T.strong }], y++);
    P(p, [{ t: pad('From', 9), fg: T.dim }, { t: '.seshi/checks.toml · added in a1b2c3d', fg: T.text }], y++);
    P(p, [{ t: pad('When', 9), fg: T.dim }, { t: 'after each agent turn ends, in that session\'s worktree', fg: T.text }], y++); y++;
    P(p, [{ t: 'Seshi never runs a repo\'s command without you.', fg: T.dim }], y++);
    P(p, [{ t: 'If the command changes, it asks again.', fg: T.dim }], y++);
    let bx = p.x + 2; bx = btn(g, R, bx, p.y + h - 2, T, 'Allow this command', 'a', 'primary', { t: 'approve' }); bx = btn(g, R, bx, p.y + h - 2, T, 'Run once', '1', 'normal', { t: 'approve' }); btn(g, R, bx, p.y + h - 2, T, "Don't run", 'Esc', 'normal', { t: 'modal', v: null });
  }
  if (s.modal === 'settings') {
    const w = Math.min(W - 2, 80), h = Math.min(H - 2, 20), p = { ...modal(g, T, W, H, w, h, 'PROJECT · affiliate-site'), w }; let y = p.y + 2;
    chipHead(g, y++, p.x + 2, T, 'CHECK', T.btn, null, T.strong); y++;
    if (s.set === 'empty') {
      P(p, [{ t: 'No check set.', fg: T.strong, b: 1 }, { t: ' Seshi can run one command itself when an agent\'s', fg: T.text }], y++);
      P(p, [{ t: 'turn ends; the session isn\'t done until it passes. Agents keep working.', fg: T.text }], y++); y++;
      P(p, [{ t: pad('Command', 10), fg: T.dim }], y); field(g, p.x + 12, y++, w - 14, T, '', 'e.g. npm test'); y++;
      P(p, [{ t: 'Projects without a check look exactly as they do today.', fg: T.dim }], y++);
    } else {
      const miss = s.set === 'missing';
      P(p, [{ t: pad('Command', 10), fg: T.dim }], y); field(g, p.x + 12, y++, w - 14, T, miss ? 'pnpm test' : 'npm test'); y++;
      P(p, [{ t: pad('Runs', 10), fg: T.dim }, { t: 'after each turn ends · in the session\'s worktree', fg: T.text }], y++);
      P(p, [{ t: pad('Timeout', 10), fg: T.dim }, { t: '10 min', fg: T.text }, { t: '  ‹ ›', fg: T.dim }], y++);
      P(p, [{ t: pad('Status', 10), fg: T.dim }, ...(miss ? [{ t: G.couldnt + ' could not run on fix-footer 14:22 · pnpm: not found (exit 127)', fg: T.rev, b: 1 }] : [{ t: G.pend + ' never run', fg: T.strong, b: 1 }, { t: ' · runs after the next turn ends', fg: T.dim }])], y++);
      if (miss) P(p, [{ t: pad('', 10) }, { t: 'Not on the worktree\'s PATH. Install it or change the command.', fg: T.dim }], y++);
      P(p, [{ t: pad('Source', 10), fg: T.dim }, { t: 'you, in settings · no approval needed', fg: T.text }], y++);
    }
    if (s.set !== 'empty') { y++; P(p, [{ t: 'CHECKPOINTS  ', fg: T.dim, b: 1 }, { t: '1 of 1 · room for more kinds later', fg: T.dim }], y++); }
    let bx = p.x + 2, by = p.y + h - 2;
    bx = btn(g, R, bx, by, T, 'Save', 'Ctrl+S', 'primary', { t: 'saveSet' });
    if (s.set !== 'empty') { bx = btn(g, R, bx, by, T, 'Run now', 'r', 'normal', { t: 'rerun' }); bx = btn(g, R, bx, by, T, 'Remove', 'Del', 'danger', { t: 'go', p: '02 Settings: empty' }); }
    btn(g, R, bx, by, T, 'Cancel', 'Esc', 'normal', { t: 'modal', v: null });
  }
}
function drawWhy(g, R, T, G, s, x, y, W) {
  const w = Math.min(60, W - x - 2), h = 8, bg = T.card;
  g.fill(x + 1, y + 1, w, h, '#000000'); g.fill(x, y, w, h, bg); g.fill(x, y, w, 1, T.btn);
  g.putSegs(x + 1, y, [{ t: ' WHY · fix-footer reads failed ', fg: T.strong, bg: T.btn, b: 1 }]);
  const P = (yy, segs) => g.putSegs(x + 2, yy, segs.map(t => ({ bg, ...t })), {}, x + w - 1);
  P(y + 2, [{ t: '14:21:51 ', fg: T.dim }, { t: 'agent\'s turn ended · it said "tests pass"', fg: T.text }]);
  P(y + 3, [{ t: '14:22:05 ', fg: T.dim }, { t: 'Seshi ran ', fg: T.text }, { t: 'npm test', fg: T.strong, b: 1 }, { t: ' in its worktree', fg: T.text }]);
  P(y + 4, [{ t: '14:22:19 ', fg: T.dim }, { t: G.failed + ' exit 1 after 14.2s · 1 of 59 tests failed', fg: T.err, b: 1 }]);
  P(y + 6, hints(T, [['⏎', 'evidence'], ['r', 're-run'], ['O', 'override'], ['Esc', 'close']]));
  R.push({ x, y, w, h, act: { t: 'sheet', v: 'evidence' } });
}

// ---------- main render
export function render(s) {
  const T = THEMES[s.theme] || THEMES.pixls, G = GL[s.glyphs] || GL.unicode, W = s.W, H = s.H, g = new Grid(W, H, T), R = [];
  const F = sessions(T, G, s), all = F.flatMap(f => f.list), cur = all[0];
  const rounded = s.style === 'rounded', gut = rounded ? 1 : 0, ov = s.style === 'flush' ? 1 : 0;
  const SW = W >= 200 ? 40 : W >= 140 ? 34 : 26, sheetOn = !!s.sheet, beside = sheetOn && W >= 140, SHW = 72;
  drawSide(g, R, T, G, s, F, 0, 0, SW, H);
  const px = SW + gut - ov, pw = W - px, paneW = beside ? pw - SHW - gut + ov : pw, sx = beside ? px + paneW + gut - ov : px;
  if (!(sheetOn && !beside)) {
    const lit = s.focus === 'pane', k = cur.k;
    panel(g, T, s, px, 0, paneW, H, lit, null, [{ t: glyph(G, k, s) + ' ' + WORD[k], fg: col(T, k), b: 1 }, { t: '  ' + G.close + ' ', fg: T.dim }]);
    let tx = px + (rounded ? 2 : 1); tx = g.putSegs(tx, 0, [{ t: ' 1 ', fg: T.accInk, bg: T.acc, b: 1 }, { t: ' fix-footer ', fg: lit ? T.accInk : T.strong, bg: lit ? T.acc : T.btn, b: 1 }]); g.putSegs(tx, 0, [{ t: ' 2 ', fg: T.text, bg: T.btn }, { t: ' add-payments ', fg: T.text, bg: T.card }]);
    drawTerm(g, T, px + 3, 2, paneW - 6, H - 3);
    R.push({ x: px + 1, y: 1, w: paneW - 2, h: H - 2, act: { t: 'focus', v: 'pane' } });
  }
  if (s.sheet === 'evidence') drawEvidence(g, R, T, G, s, sx, 0, beside ? SHW : pw, H);
  if (s.sheet === 'inbox') drawInbox(g, R, T, G, s, sx, 0, beside ? SHW : pw, H);
  if (s.pop) drawWhy(g, R, T, G, s, Math.min(SW + 2, W - 40), 5, W);
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
    case 'modal': return S({ modal: a.v, set: a.set || s.set, focus: a.v ? 'modal' : s.sheet ? 'sheet' : 'side', pop: 0 });
    case 'doOverride': return S({ modal: null, stage: 'over', sheet: 'evidence', focus: 'sheet', toast: 'Overridden. fix-footer reads done (overridden); the failure stays in its evidence.' });
    case 'approve': return { state: { ...s, modal: null, stage: 'checking', focus: 'side', toast: 'Allowed npm test for this repo. Checking fix-footer…' }, fx: [{ after: 2600, act: { t: 'result', v: 'failed' } }] };
    case 'rerun': return { state: { ...s, modal: null, pop: 0, stage: 'checking', sheet: null, focus: 'side', toast: 'Re-running npm test in fix-footer\'s worktree…' }, fx: [{ after: 2600, act: { t: 'result', v: 'passed' } }] };
    case 'result': return S({ stage: a.v, toast: a.v === 'passed' ? 'test passed (exit 0, 13.9s). fix-footer reads done.' : 'test failed (exit 1). fix-footer is in the Inbox.' });
    case 'saveSet': return S({ modal: null, stage: s.stage === 'none' ? 'none' : s.stage, focus: 'side', toast: s.set === 'empty' ? 'Saved. Runs after the next turn ends.' : 'Saved.' });
    case 'key': {
      const k = a.key;
      if (s.leader) { const p = { leader: false }; if (k === 'j') return act({ ...s, ...p }, { t: 'sheet', v: 'inbox' }); if (k === ',') return act({ ...s, ...p }, { t: 'modal', v: 'settings', set: s.stage === 'none' ? 'empty' : s.stage === 'couldnt' ? 'missing' : 'never' }); return S(p); }
      if (s.modal) { if (k === 'Escape') return act(s, { t: 'modal', v: null }); if (s.modal === 'override' && k === 'O') return act(s, { t: 'doOverride' }); if (s.modal === 'approve' && (k === 'a' || k === '1')) return act(s, { t: 'approve' }); if (s.modal === 'settings' && k === 'r' && s.set !== 'empty') return act(s, { t: 'rerun' }); return null; }
      if (s.pop) { if (k === 'Escape') return S({ pop: 0 }); if (k === 'Enter') return act(s, { t: 'sheet', v: 'evidence' }); if (k === 'r') return act(s, { t: 'rerun' }); if (k === 'O') return act(s, { t: 'modal', v: 'override' }); return null; }
      const failed = s.stage === 'failed' || s.stage === 'multi';
      if (s.focus === 'sheet') {
        if (k === 'Escape') return act(s, { t: 'sheet', v: null });
        if (k === 'r') return act(s, { t: 'rerun' });
        if (k === 'O' && failed) return act(s, { t: 'modal', v: 'override' });
        if (k === 'u' && s.stage === 'over') return act(s, { t: 'go', p: '09 Evidence' });
        if (k === 'f') return toast('Full output opens in the pager (1,284 lines)');
        if (s.sheet === 'inbox') { const n = inboxItems(THEMES[s.theme], GL[s.glyphs], s).length; if (k === 'ArrowDown' || k === 'j') return S({ isel: Math.min(n - 1, s.isel + 1) }); if (k === 'ArrowUp' || k === 'k') return S({ isel: Math.max(0, s.isel - 1) }); if (k === 'Enter') return act(s, { t: 'sheet', v: 'evidence' }); }
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
