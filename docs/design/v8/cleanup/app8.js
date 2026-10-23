// Seshi v8 · cleanup mock. Fresh layout: clear row states, subagent count only, grouped Actions, trimmed Settings. SAMPLE DATA ONLY.
import { Grid, trunc, segLen, L, mix } from './seshi-grid.js';

const TA = { green: '#86d97a', red: '#ff7b7b', cyan: '#7dcfff', blue: '#6cb6ff', yellow: '#e8c565', magenta: '#c4a7e7', gray: '#7f8290' };
export const THEMES = {
  seshi: { name: 'dark', label: 'Seshi', bg: '#272a31', fg: '#c9c9d1', card: '#302d3b', card2: '#3b3748', btn: '#3f3b4d', line: '#4a4656', dim: '#868997', text: '#c9c9d1', strong: '#eeecf5', acc: '#c4a7e7', accInk: '#1f1b29', work: '#ff9a55', needs: '#f5c542', ok: '#86d97a', err: '#ff6b6b', sky: '#6cb6ff', rev: '#b58cff', scrim: '#000000', a: TA },
  pixls: { name: 'dark', label: 'Pixls', bg: '#141413', fg: '#c9c5bb', card: '#1c1c1a', card2: '#262624', btn: '#2e2d2a', line: '#3a3934', dim: '#8a867c', text: '#c9c5bb', strong: '#ece9e1', acc: '#ff6a2f', accInk: '#141413', work: '#ff9a55', needs: '#f2c200', ok: '#3cc27e', err: '#ff5a52', sky: '#5aa9ff', rev: '#b58cff', scrim: '#000000', a: TA },
};
const bl = [0, 0x40, 0x44, 0x46, 0x47], br = [0, 0x80, 0xA0, 0xB0, 0xB8];
const BARS = [[1, 3, 2], [2, 4, 1], [3, 3, 3], [4, 2, 4], [3, 1, 3], [2, 2, 2], [1, 4, 3], [2, 3, 4], [4, 1, 2], [3, 2, 1]].map(([a, b, c]) => String.fromCharCode(0x2800 + bl[a] + br[b]) + String.fromCharCode(0x2800 + bl[c]));

const PROJ = [
  { name: 'AeVoxAffiliate', meta: '', list: [{ id: 'Stripe stuffs', agent: 'codex', age: '11m', k: 'seen' }] },
  { name: 'snipux', meta: '⎇ dev', list: [{ id: 'tall-crane', agent: 'claude', age: '43s', k: 'unseen' }] },
  { name: 'AeVoxPlatformDashboard', meta: '⎇ dev', list: [{ id: 'Make demo.aevoxdev.com', agent: 'codex', age: '2m', k: 'needs', q: 'asks: deploy to staging or prod?' }, { id: 'Auth page fixes', agent: 'claude', age: '9m', k: 'failed', q: 'failed: test · exit 1' }] },
  { name: 'seshi', meta: '⎇ dev', list: [{ id: 'swift-lynx', agent: 'claude', age: '6m', k: 'working', sub: 4 }] },
];
function projOf(s) {
  return PROJ.map(p => ({ ...p, list: p.list.map(q => q.id === 'swift-lynx' && s.helpers === 'ask' ? { ...q, k: 'needs', q: 'wants to start 4 helpers', sub: 0 } : q)
    .concat(p.name === 'seshi' && s.newS ? [{ id: s.newS, agent: s.newAgent || 'claude', age: 'now', k: 'new' }] : []) }));
}
const allOf = s => projOf(s).flatMap(p => p.list);
const NO = { a: ['claude', 'codex', 'gemini', 'shell'], w: ['new worktree', 'this folder'], p: ['seshi', 'snipux', 'AeVoxAffiliate', 'AeVoxPlatformDashboard'] };
const TASKS = ['Find and fix menu bug', 'Build 3-bar spinner and shimmer', 'Restore status colour chips', 'Allow rounded in Flat layout'];

export const PRESETS = [
  ['01 Overview', {}],
  ['02 Actions menu', { menu: 0, focus: 'menu' }],
  ['03 Message a session', { sel: 1, msg: 1, focus: 'msg' }],
  ['04 Inbox', { sheet: 'inbox', focus: 'sheet' }],
  ['05 Settings · General', { modal: 'settings', tab: 0, row: 3, focus: 'modal' }],
  ['06 Settings · Agents', { modal: 'settings', tab: 3, row: 3, focus: 'modal' }],
  ['07 Key map (Ctrl+Space)', { cmd: 'map', focus: 'modal' }],
  ['08 Narrow 100×30', { W: 100, H: 30 }],
  ['09 New session · instant (+)', { newS: 'bright-otter', sel: 5, paneNew: 1, focus: 'pane' }],
  ['10 New session with options (c)', { modal: 'newopt', no: { row: 0, a: 0, w: 0, p: 0 }, focus: 'modal' }],
  ['11 Session wants helpers', { helpers: 'ask', hpop: 1, sel: 4 }],
  ['12 Settings · Sessions', { modal: 'settings', tab: 1, row: 3, focus: 'modal' }],
  ['13 Sidebar collapsed (z)', { rail: 1, focus: 'pane' }],
  ['14 Collapsed · switch sessions', { rail: 1, focus: 'side', sel: 2 }],
  ['15 Search (type or Tab)', { cmd: 'search', q: 'spl', ci: 0, focus: 'modal' }],
  ['16 All keys (?)', { cmd: 'all', focus: 'modal' }],
  ['17 Copy · selecting', { copy: 'sel', focus: 'pane' }],
  ['18 Copy · copied', { copy: 'done', focus: 'pane' }],
];
export function preset(name, W, H, fl = {}) {
  const p = (PRESETS.find(x => x[0] === name) || PRESETS[0])[1];
  return { W, H, theme: fl.theme || 'seshi', motion: fl.motion !== false, f: 0, focus: 'side', sel: 4, menu: null, msg: 0, sheet: null, isel: 0, modal: null, tab: 0, row: 0, opt: {}, toast: null, leader: false, hide: false, subs: true, rail: 0, newS: null, paneNew: 0, helpers: 'ok', hpop: 0, no: null, cmd: null, q: '', ci: 0, copy: null, ...p };
}

function shimmer(t, s, T, extra = {}) {
  const a = Array.from(t), n = a.length, base = T.work, hi = mix(T.work, '#ffffff', 0.55), pos = s.motion ? (s.f % (n + 8)) - 4 : -99;
  return a.map((ch, i) => { const d = Math.abs(i - pos); return { t: ch, fg: d < 1 ? hi : d < 2.5 ? mix(hi, base, 0.5) : base, b: 1, ...extra }; });
}
const pad = (s, n) => { const a = Array.from(s); return a.length >= n ? trunc(s, n) : s + ' '.repeat(n - a.length); };
// Keys are drawn as keycaps: a raised chip per key, combos joined with a dim +.
const cap = (T, k, bg) => (k.length > 1 && k.includes('+') ? k.split('+') : [k]).flatMap((p, i) => [...(i ? [{ t: '+', fg: T.dim, bg }] : []), { t: ' ' + p + ' ', fg: T.strong, bg: T.btn, b: 1 }]);
const capLen = k => (k.length > 1 && k.includes('+') ? k.split('+') : [k]).reduce((n, p, i) => n + L(p) + 2 + (i ? 1 : 0), 0);
function actionBar(g, T, x, y, w, list, ground) { const bg = ground || T.card; g.fill(x, y, w, 1, bg); g.putSegs(x + 1, y, hints(T, list, bg), {}, x + w - 1); }
const hints = (T, list, bg) => list.flatMap((h, i) => [...(i ? [{ t: '   ', bg }] : []), ...cap(T, h[0], bg), { t: ' ' + h[1], fg: T.text, bg }]);
const chip = (T, label, on) => ({ t: ' ' + label + ' ', fg: on ? T.accInk : T.strong, bg: on ? T.acc : T.btn, b: 1 });
const btn = (g, R, x, y, T, label, key, act) => { const x2 = g.putSegs(x, y, [{ t: ' ' + label + ' ', bg: T.btn, fg: T.strong, b: 1 }, { t: key + ' ', bg: T.btn, fg: T.acc, b: 1 }]); R.push({ x, y, w: x2 - x, h: 1, act }); return x2 + 1; };
function glyphOf(T, q, s) {
  return { working: [{ t: BARS[s.motion ? s.f % BARS.length : 2], fg: T.work, b: 1 }, { t: ' ' }], needs: [{ t: '●  ', fg: T.needs, b: 1 }], failed: [{ t: '✕  ', fg: T.err, b: 1 }], unseen: [{ t: '✓  ', fg: T.ok, b: 1 }], seen: [{ t: '○  ', fg: T.dim }], new: [{ t: '○  ', fg: T.acc, b: 1 }] }[q.k];
}
function line2(T, q, s) {
  if (q.k === 'working') return [{ t: q.agent + ' · ', fg: T.dim }, ...shimmer('working', s, T), ...(s.subs && q.sub ? [{ t: ' · ' + q.sub + ' agents', fg: T.text }] : [])];
  if (q.k === 'needs') return [{ t: q.q, fg: T.needs }];
  if (q.k === 'failed') return [{ t: q.q, fg: T.err, b: 1 }];
  if (q.k === 'new') return [{ t: q.agent + ' · ', fg: T.dim }, { t: 'new · type to begin', fg: T.acc }];
  if (q.k === 'unseen') return [{ t: 'done · not seen', fg: T.ok, b: 1 }];
  return [{ t: 'done', fg: T.dim }];
}

// ---------- sidebar
function drawSide(g, R, T, s, w, H) {
  const lit = s.focus === 'side', tint = mix(T.bg, T.text, 0.12), wide = s.W >= 140, X = 1, RX = w - 3;
  const ALL = allOf(s), nN = ALL.filter(q => q.k === 'needs').length, nF = ALL.filter(q => q.k === 'failed').length;
  g.putSegs(1, 0, [chip(T, 'SESSIONS ' + ALL.length, lit)]);
  const cnt = [{ t: '●' + nN, fg: T.needs, b: 1 }, { t: ' ✕' + nF, fg: T.err, b: 1 }]; g.putSegs(w - 2 - segLen(cnt), 0, cnt.map(c => ({ ...c, bg: T.card })));
  let yy = 2, idx = 0;
  for (const p of projOf(s)) {
    const room = RX - X - 3 - L(p.name), meta = p.meta && room >= 6 ? p.meta : '';
    g.putSegs(X + 1, yy, [{ t: '▾ ', fg: T.acc }, { t: trunc(p.name, RX - X - 2), fg: T.strong, b: 1 }]); if (meta) g.put(RX + 1 - L(meta), yy, meta, { fg: T.dim }); yy++;
    for (const q of p.list) {
      const on = idx === s.sel, bg = on ? tint : undefined, n = wide || q.k !== 'seen' ? 2 : 1, hot = q.k !== 'seen';
      if (on) g.fill(X, yy, RX - X + 2, n, tint);
      if (on || q.k === 'failed') for (let j = 0; j < n; j++) g.set(X, yy + j, '▎', { fg: q.k === 'failed' ? T.err : lit ? T.acc : T.dim, bg, b: 1 });
      const nm = trunc(q.id, RX - X - 4 - L(q.age) - 1);
      g.putSegs(X + 1, yy, [...glyphOf(T, q, s).map(t => ({ ...t, bg })), ...(q.k === 'working' ? shimmer(nm, s, T, { bg }) : [{ t: nm, fg: hot || on ? T.strong : T.text, b: hot, bg }])]);
      g.put(RX + 1 - L(q.age), yy, q.age, { fg: T.dim, bg });
      if (n === 2) g.putSegs(X + 4, yy + 1, line2(T, q, s).map(t => ({ ...t, bg })), {}, RX + 2);
      R.push({ x: X, y: yy, w: RX - X + 2, h: n, act: { t: 'sel', i: idx } });
      yy += n; idx++;
    }
    yy++;
  }
  // footer: lifted one row with a divider above, no band background; two quiet buttons, key letter in accent
  g.hl(1, H - 3, w - 3, '─', { fg: T.line });
  const fb = (x, word, a) => { const segs = [{ t: ' ', bg: T.btn }, { t: word[0], fg: T.acc, b: 1, bg: T.btn }, { t: word.slice(1) + ' ', fg: T.text, bg: T.btn }]; const x2 = g.putSegs(x, H - 2, segs); R.push({ x, y: H - 2, w: x2 - x, h: 1, act: a }); return x2; };
  fb(2, 'new', { t: 'newNow' }); fb(w - 3 - 6, 'menu', { t: 'menu' });
}

// ---------- collapsed (z): sessions move into the top strip as chips; the pane gets the full width
function drawStrip(g, R, T, s, W) {
  const ALL = allOf(s), lit = s.focus === 'side';
  let x = g.putSegs(1, 0, [chip(T, '»', false)]) + 1; R.push({ x: 1, y: 0, w: 3, h: 1, act: { t: 'rail' } });
  for (let i = 0; i < ALL.length; i++) {
    const q = ALL[i], on = i === s.sel, bg = on ? (lit ? T.acc : T.btn) : T.card, ink = on && lit;
    const segs = [{ t: ' ', bg }, ...glyphOf(T, q, s).map(t => ({ ...t, fg: ink ? T.accInk : t.fg, bg })), { t: trunc(q.id, 16) + ' ', fg: ink ? T.accInk : q.k === 'seen' && !on ? T.text : T.strong, b: on || q.k !== 'seen', bg }];
    if (x + segLen(segs) > W - 26) { const m = ' +' + (ALL.length - i) + ' more '; g.putSegs(x, 0, [{ t: m, fg: T.dim, bg: T.card }]); R.push({ x, y: 0, w: L(m), h: 1, act: { t: 'rail' } }); break; }
    const x0 = x; x = g.putSegs(x, 0, segs) + 1; R.push({ x: x0, y: 0, w: x - x0 - 1, h: 1, act: { t: 'sel', i } });
  }
}

// ---------- pane: agent output untouched
function drawPane(g, R, T, s, x, w, H) {
  const lit = s.focus === 'pane', A = T.a;
  if (s.paneNew && !s.rail) {
    let tx = g.putSegs(x + 1, 0, [{ t: ' 1 ', fg: T.dim, bg: T.btn, b: 1 }, { t: ' swift-lynx ', fg: T.text, bg: T.btn }]); R.push({ x: x + 1, y: 0, w: tx - x - 1, h: 1, act: { t: 'tab1' } });
    tx = g.putSegs(tx + 1, 0, [{ t: ' 2 ', fg: lit ? T.accInk : T.dim, bg: lit ? T.acc : T.btn, b: 1 }, { t: ' ' + s.newS + ' ', fg: lit ? T.accInk : T.strong, bg: lit ? T.acc : T.btn, b: 1 }]);
    g.putSegs(tx + 1, 0, [{ t: ' + ', fg: T.acc, bg: T.btn, b: 1 }]);
    const st = [{ t: '○ new  ', fg: T.acc, b: 1, bg: T.card }, { t: '✕ ', fg: T.dim, bg: T.card }]; g.putSegs(x + w - segLen(st), 0, st);
    const cx = x + 3, ag = s.newAgent || 'claude';
    g.putSegs(cx, 2, [{ t: '✻ ', fg: A.magenta, b: 1 }, { t: ag === 'codex' ? 'Codex' : ag === 'gemini' ? 'Gemini CLI' : ag === 'shell' ? 'bash' : 'Claude Code', fg: T.strong, b: 1 }]);
    g.put(cx + 2, 3, '~/personal/seshi-worktrees/' + s.newS, { fg: T.dim });
    g.put(cx + 2, 4, 'new worktree · branch seshi/' + s.newS + ' off dev', { fg: T.dim });
    g.hl(cx, H - 5, w - 5, '─', { fg: T.line }); g.putSegs(cx, H - 4, [{ t: '› ', fg: T.strong }, { t: '█', fg: T.text }, { t: ' Try "fix the menu bug"', fg: T.dim }]); g.hl(cx, H - 3, w - 5, '─', { fg: T.line });
    g.putSegs(cx, H - 2, [{ t: '~/personal/seshi-worktrees/' + s.newS, fg: A.red }, { t: ' │ ctx ', fg: T.dim }, { t: '0%', fg: A.cyan }]);
    R.push({ x: x + 1, y: 1, w: w - 1, h: H - 1, act: { t: 'focus', v: 'pane' } });
    return;
  }
  { const ty = s.rail ? 1 : 0; if (s.rail) g.fill(x, 1, w, 1, T.bg); let tx = g.putSegs(x + 1, ty, [{ t: ' 1 ', fg: lit ? T.accInk : T.dim, bg: lit ? T.acc : T.btn, b: 1 }, { t: ' swift-lynx ', fg: lit ? T.accInk : T.strong, bg: lit ? T.acc : T.btn, b: 1 }]);
  const px = tx + 1; tx = g.putSegs(px, ty, [{ t: ' + ', fg: T.acc, bg: T.btn, b: 1 }]); R.push({ x: px, y: ty, w: 3, h: 1, act: { t: 'toast', v: 'New tab' } }); }
  const st = [{ t: BARS[s.motion ? s.f % BARS.length : 2] + ' ', fg: T.work, b: 1, bg: T.card }, ...shimmer('working', s, T, { bg: T.card }), { t: '  ✕ ', fg: T.dim, bg: T.card }];
  g.putSegs(x + w - segLen(st), 0, st);
  const E = { segs: [] }, Ln = (segs, r) => ({ segs, r }), RULE = { rule: 1 };
  const tree = (c, t) => Ln([{ t: '  ' + c + ' ', fg: T.dim }, { t, fg: T.strong, b: 1 }]);
  const lines = [
    Ln([{ t: '● ', fg: T.strong }, { t: "I'll fan this out: four agents, each in its own worktree so edits don't collide.", fg: T.text }]), E,
    Ln([{ t: '● ', fg: A.blue }, { t: '4 background agents launched (↓ to manage)', fg: T.text }]),
    tree('├', 'Find and fix menu bug'), tree('├', 'Build 3-bar spinner and shimmer'), tree('├', 'Restore status colour chips'), tree('└', 'Allow rounded in Flat layout'), E,
    Ln([{ t: '● ', fg: T.strong }, { t: 'Now my part: the + chip and the reference frames it changes.', fg: T.text }]),
    Ln([{ t: '  Ran 6 shell commands', fg: T.dim }]), E,
    Ln([{ t: '● ', fg: T.strong }, { t: 'The + chip is done and the full gate passes: build, tests (752 passed), clippy.', fg: T.text }]),
    Ln([{ t: "  The four agents are still running, so I'll wait for them.", fg: T.text }]), E,
    Ln([{ t: '* ', fg: T.dim }, { t: 'Waiting for 4 background agents to finish', fg: T.text }]),
    RULE, Ln([{ t: '› ', fg: T.strong }, { t: '█', fg: T.text }]), RULE,
    Ln([{ t: '~/personal/seshi-worktrees/swift-lynx', fg: A.red }, { t: ' │ ctx ', fg: T.dim }, { t: '12%', fg: A.cyan }, { t: ' │ wk ', fg: T.dim }, { t: '56%', fg: A.yellow }]),
    Ln([{ t: '›› auto mode on', fg: A.yellow }, { t: ' · ← for agents', fg: T.text }]), E,
    Ln([{ t: '● ', fg: A.yellow }, { t: 'main', fg: A.yellow, b: 1 }]),
    ...[['Running flat_corners test filter', '121.4k'], ['Rereading unchanged test output file', '69.3k'], ['Running cargo test after updating expectations', '124.4k'], ['Editing theme.rs needs-amber helper', '133.0k']].map(([t, k]) => Ln([{ t: '○ ', fg: T.dim }, { t: 'general-purpose  ', fg: T.text }, { t, fg: T.text }], '5m 30s · ↓ ' + k + ' tokens')),
  ];
  const cx = x + 3, cw = w - 5, top = Math.max(2, H - 1 - lines.length), shown = lines.slice(Math.max(0, lines.length - (H - 3)));
  shown.forEach((l, i) => { const yy = top + i; if (l.rule) { g.hl(cx, yy, cw, '─', { fg: T.line }); return; } const rw = l.r ? L(l.r) + 2 : 0; g.putSegs(cx, yy, l.segs, {}, cx + cw - rw); if (l.r && cw > 70) g.put(cx + cw - L(l.r), yy, l.r, { fg: T.text }); });
  if (s.copy) {
    // selection = shown lines 2..6; partial last line. Selecting: sky tint. Copied: accent tint + a receipt chip at the end of the selection.
    const y0 = top + 2, y1 = top + 6, endX = cx + Math.min(36, cw - 30), tint = s.copy === 'sel' ? mix(T.bg, T.sky, 0.38) : mix(T.bg, T.acc, 0.34);
    for (let yy = y0; yy <= y1; yy++) { const xe = yy === y1 ? endX : cx + cw; for (let xx = cx; xx < xe; xx++) { const c = g.c[yy] && g.c[yy][xx]; if (c) { c.bg = tint; c.fg = T.strong; } } }
    if (s.copy === 'sel') g.putSegs(endX, y1, [{ t: '▏', fg: T.sky, b: 1 }]);

    const hb = s.copy === 'sel' ? [['y', 'copy'], ['Esc', 'cancel']] : null;
    if (hb) g.putSegs(endX + 2, y1, hints(T, hb, T.card));
  }
  R.push({ x: x + 1, y: 1, w: w - 1, h: H - 1, act: { t: 'focus', v: 'pane' } });
}

// ---------- inbox sheet
function drawInbox(g, R, T, s, x, w, H) {
  const lit = s.focus === 'sheet', X = x + 2, iw = w - 4, tint = mix(T.bg, T.text, 0.12);
  g.putSegs(x + 1, 0, [chip(T, 'INBOX 2', lit)]); g.putSegs(x + w - 3, 0, [{ t: '✕ ', fg: T.dim, bg: T.card }]); R.push({ x: x + w - 3, y: 0, w: 2, h: 1, act: { t: 'sheet', v: null } });
  const groups = [
    ['NEEDS YOU', T.needs, [{ id: 'Make demo.aevoxdev.com', p: 'AeVoxPlatformDashboard', age: '2m', g: '●', c: T.needs, l2: [{ t: 'asks: deploy to staging or prod?', fg: T.needs }], acts: [['Reply', 'r', { t: 'msgFor', i: 2 }], ['Open', '⏎', { t: 'toast', v: 'Opens the session' }]] }]],
    ['FAILED CHECKS', T.err, [{ id: 'Auth page fixes', p: 'AeVoxPlatformDashboard', age: '9m', g: '✕', c: T.err, l2: [{ t: 'failed: test · exit 1 · 14:22', fg: T.err, b: 1 }], l3: '✗ auth.test.ts › redirects after sign-in', acts: [['Output', '⏎', { t: 'toast', v: 'Opens the check evidence' }], ['Re-run', 'r', { t: 'toast', v: 'Re-running npm test…' }]] }]],
    ['NOT SEEN', T.ok, [{ id: 'tall-crane', p: 'snipux', age: '43s', g: '✓', c: T.ok, l2: [{ t: 'done · not seen', fg: T.ok, b: 1 }], acts: [['Open', '⏎', { t: 'toast', v: 'Opens the session; it becomes seen' }]] }]],
  ];
  let yy = 2, i = 0;
  for (const [label, c, list] of groups) {
    g.putSegs(X, yy++, [{ t: ' ' + label + ' ', fg: T.bg, bg: c, b: 1 }, { t: ' ' + list.length + ' ', fg: T.bg, bg: mix(c, T.bg, 0.25), b: 1 }]);
    for (const q of list) {
      const on = s.isel === i, n = 2 + (q.l3 ? 1 : 0) + (on ? 1 : 0), bg = on ? tint : undefined;
      if (on) { g.fill(x + 1, yy, w - 2, n, tint); for (let j = 0; j < n; j++) g.set(x + 1, yy + j, '▎', { fg: lit ? T.acc : T.dim, bg: tint }); }
      g.putSegs(X, yy, [{ t: q.g + ' ', fg: q.c, b: 1, bg }, { t: q.id, fg: T.strong, b: 1, bg }, { t: ' · ' + q.p, fg: T.dim, bg }], {}, X + iw - 5); g.put(X + iw - L(q.age), yy, q.age, { fg: T.dim, bg });
      g.putSegs(X + 2, yy + 1, q.l2.map(t => ({ ...t, bg })), {}, X + iw);
      if (q.l3) g.putSegs(X + 2, yy + 2, [{ t: q.l3, fg: T.dim, bg }], {}, X + iw);
      R.push({ x: x + 1, y: yy, w: w - 2, h: n, act: { t: 'isel', i } });
      if (on) { let bx = X + 2; q.acts.forEach(([l, k, a]) => { bx = btn(g, R, bx, yy + n - 1, T, l, k, a); }); }
      yy += n + 1; i++;
    }
  }
  actionBar(g, T, x + 1, H - 1, w - 1, [['↑↓', 'select'], ['⏎', 'open'], ['r', 'reply'], ['Esc', 'close']]);
}

// ---------- floating panels
function float(g, T, x, y, w, h, title, right) {
  g.fill(x, y, w, h, T.card); g.fill(x, y, w, 1, T.acc);
  g.putSegs(x + 2, y, [{ t: title, fg: T.accInk, bg: T.acc, b: 1 }]); if (right) g.putSegs(x + w - L(right) - 2, y, [{ t: right, fg: T.accInk, bg: T.acc }]);
}
const MENU = [['NEW'], ['Session', 'n', { t: 'newNow' }], ['Session with options…', 'c', { t: 'modal', v: 'newopt' }], ['Shell here', 's', 'New shell in this session\'s folder'], ['Pane beside this', 'p', 'Split'], ['Tab', 't', 'New tab'], ['GO TO'], ['Inbox', 'j', { t: 'sheet', v: 'inbox' }], ['Search commands', 'Space', { t: 'cmd', v: 'search' }], ['All keys', '?', { t: 'cmd', v: 'all' }], ['VIEW'], ['Collapse sidebar', 'z', { t: 'rail' }], ['Hide sidebar', 'Z', { t: 'hide' }], ['SET UP'], ['Settings', ',', { t: 'modal', v: 'settings' }]];
const MITEMS = MENU.filter(m => m[1]);
const menuAct = m => typeof m[2] === 'string' ? { t: 'toast', v: m[2] } : m[2];
function drawMenu(g, R, T, s, H) {
  const w = 42, h = MENU.length + 5, x = 1, y = H - 2 - h, tint = mix(T.card, T.text, 0.12);
  float(g, T, x, y, w, h, 'ACTIONS', 'Esc');
  let yy = y + 2, i = 0;
  MENU.forEach(m => {
    if (!m[1]) { g.put(x + 2, yy++, m[0], { fg: T.dim, bg: T.card, b: 1 }); return; }
    const on = s.menu === i, bg = on ? tint : T.card;
    g.fill(x + 1, yy, w - 2, 1, bg); if (on) g.set(x + 1, yy, '▎', { fg: T.acc, bg });
    g.put(x + 4, yy, m[0], { fg: on ? T.strong : T.text, bg, b: on }); g.put(x + w - 3 - L(m[1]), yy, m[1], { fg: T.acc, bg, b: 1 });
    R.push({ x: x + 1, y: yy, w: w - 2, h: 1, act: { t: 'menuRun', i } }); yy++; i++;
  });
  g.putSegs(x + 2, y + h - 2, [{ t: 'Ctrl+Space', fg: T.acc, b: 1, bg: T.card }, { t: ' then a key works anywhere', fg: T.dim, bg: T.card }]);
}
function drawMsg(g, R, T, s, px, pw) {
  const q = allOf(s)[s.sel], w = Math.min(78, pw - 4), x = px + 2, y = 2, h = 10;
  float(g, T, x, y, w, h, 'MESSAGE ' + q.id + ' · ' + q.agent, 'Esc');
  g.putSegs(x + 2, y + 2, [{ t: 'replying to  ', fg: T.dim, bg: T.card }, { t: trunc('"Slowness is already measurable: my notes show open-to-visible dropped…"', w - 17), fg: T.text, bg: T.card, i: 1 }]);
  g.fill(x + 2, y + 4, w - 4, 3, T.card2); g.putSegs(x + 3, y + 4, [{ t: 'Next prompt for ' + q.agent + '…', fg: T.dim, bg: T.card2, i: 1 }, { t: ' █', fg: T.acc, bg: T.card2 }]);
  let bx = x + 2; bx = btn(g, R, bx, y + h - 2, T, 'Send', 'Enter', { t: 'send' }); bx = btn(g, R, bx, y + h - 2, T, 'New line', 'Shift+Enter', { t: 'toast', v: 'New line' }); btn(g, R, bx, y + h - 2, T, 'Cancel', 'Esc', { t: 'close' });
}
const TABS = ['General', 'Sessions', 'Appearance', 'Agents', 'Check', 'Keys'];
const SET = {
  0: [{ h: 'INPUT' }, { l: 'Leader key', o: ['Ctrl+Space'], x: 'Press it, let go, then a key. To change it: Enter, then press the new leader.' }, { l: 'Mouse', o: ['on', 'off'], x: 'Click, scroll and drag in Seshi\'s panels. Agent panes still get their own mouse events.' }, { l: 'Alt+arrows move between panes', o: ['on', 'off'], x: 'Alt+←↑↓→ moves focus without the leader.' }, { l: 'Leader waits for next key', o: ['on', 'off'], x: 'After the leader, Seshi waits for your next key instead of timing out. Was "Keys after a pause".' },
    { h: 'LAYOUT' }, { l: 'Layout', o: ['cards', 'flat'], d: 1, x: 'cards: a border around each panel. flat: one header strip and thin rules.' }, { l: 'Background', o: ['terminal', 'solid'], x: 'terminal: your terminal\'s background shows through. solid: Seshi paints its own.' }, { l: 'Sidebar side', o: ['left', 'right'], x: 'Which side the sessions list sits on.' }, { l: 'Motion', o: ['on', 'off'], x: 'Spinners and shimmer. Off freezes them; nothing depends on motion.' },
    { h: 'SHELL' }, { l: 'Start folder', o: ['(default)'], x: 'Where new shells open.' }, { l: 'Editor', o: ['default', 'nvim'], x: 'Opens files from Seshi.' }, { l: 'Shell', o: ['default', 'bash'], x: 'Shell for new panes.' }],
  1: [{ h: 'NEW SESSIONS' }, { l: '+ starts', o: ['instantly', 'with options'], x: 'instantly: + opens a session with the defaults below. c always asks first.' }, { l: 'Agent', o: ['last used', 'claude', 'codex', 'gemini'], x: 'Which agent + starts. "last used" follows your last pick in c.' }, { l: 'Where', o: ['new worktree', 'this folder'], x: 'new worktree: its own branch, so sessions running side by side don\'t collide.' },
    { h: 'HELPERS' }, { l: 'Sessions may start helpers', o: ['ask', 'always', 'never'], x: 'ask: the first time a session wants helpers, its row asks you once (1 allow · 2 always here · 3 no).' }, { l: 'Helpers at once', o: ['2', '3', '4'], d: 1, x: 'Per session. Extra helpers wait their turn.' }],
  3: [{ h: 'AGENT STATUS' }, { l: 'Claude', o: ['✓ hooks installed'], ok: 1, x: 'Seshi reads working / needs you / done from Claude\'s hooks.' }, { l: 'Codex', o: ['✓ read from the process'], ok: 1, x: 'No hooks needed; Seshi watches the process.' }, { l: 'Set up another agent…', o: ['Enter'], x: 'Moved here from the Actions menu ("Set up agent status").' },
    { h: 'SUBAGENTS' }, { l: 'Count on the session row', o: ['on', 'off'], x: 'Shows "· 4 agents" only when the agent reports subagents. Claude does; Codex doesn\'t, so Codex rows never show one. The list stays in the agent\'s own pane.' }],
};
const rowsOf = tab => (SET[tab] || [{ note: 'Unchanged in this mock.' }]);
const selRows = tab => rowsOf(tab).filter(r => r.o);
function drawSettings(g, R, T, s, W, H) {
  const w = Math.min(W - 4, 112), h = Math.min(H - 2, 40), x = Math.floor((W - w) / 2), y = Math.max(1, Math.floor((H - h) / 2)), X = x + 3, tight = H < 40;
  g.dimAll(); float(g, T, x, y, w, h, 'SETTINGS', 'Esc close');
  let tx = X; TABS.forEach((t, i) => { const x0 = tx; tx = g.putSegs(tx, y + 2, [{ t: ' ' + t + ' ', fg: i === s.tab ? T.accInk : T.text, bg: i === s.tab ? T.acc : T.card, b: i === s.tab }]) + 2; R.push({ x: x0, y: y + 2, w: tx - x0 - 2, h: 1, act: { t: 'tab', v: i } }); });
  g.put(x + w - 26, y + 2, 'Team tab removed', { fg: T.dim, bg: T.card, i: 1 });
  g.hl(X, y + 3, w - 6, '─', { fg: T.line, bg: T.card });
  let yy = y + (tight ? 4 : 5), si = 0, cur = null; const maxY = y + h - 6, tint = mix(T.card, T.text, 0.1);
  for (const r of rowsOf(s.tab)) {
    if (yy > maxY) break;
    if (r.note) { g.put(X + 1, yy++, r.note, { fg: T.dim, bg: T.card }); continue; }
    if (r.h) { if (!tight && yy > y + 5) yy++; g.put(X + 1, yy++, r.h, { fg: T.dim, bg: T.card, b: 1 }); continue; }
    const on = si === s.row, bg = on ? tint : T.card, val = s.opt[s.tab + r.l] ?? (r.d || 0); if (on) cur = r;
    g.fill(X, yy, w - 6, 1, bg); if (on) g.put(X + 1, yy, '›', { fg: T.acc, bg, b: 1 });
    g.put(X + 3, yy, r.l, { fg: on ? T.strong : T.text, bg, b: on });
    let ox = X + 36; r.o.forEach((o, k) => { const sel = r.o.length > 1 ? k === val : true, x0 = ox; ox = g.putSegs(ox, yy, [{ t: ' ' + o + ' ', fg: r.ok ? T.ok : sel ? (r.o.length > 1 ? T.accInk : T.strong) : T.dim, bg: r.ok ? bg : sel && r.o.length > 1 ? T.acc : bg, b: sel }]) + 1; if (r.o.length > 1) R.push({ x: x0, y: yy, w: ox - x0 - 1, h: 1, act: { t: 'opt', row: si, v: k } }); });
    R.push({ x: X, y: yy, w: 34, h: 1, act: { t: 'row', v: si } });
    yy++; si++;
  }
  const hy = y + h - 5; g.hl(X, hy, w - 6, '─', { fg: T.line, bg: T.card });
  if (cur) {
    const w1 = w - 12 - L(cur.l), w2 = w - 8, words = (cur.x || '').split(' '); let l1 = '', l2 = '';
    for (const wd of words) { if (!l2 && L(l1 ? l1 + ' ' + wd : wd) <= w1) l1 = l1 ? l1 + ' ' + wd : wd; else l2 = l2 ? l2 + ' ' + wd : wd; }
    g.putSegs(X + 1, hy + 1, [{ t: cur.l + '  ', fg: T.strong, b: 1, bg: T.card }, { t: l1, fg: T.text, bg: T.card }], {}, x + w - 3);
    if (l2) g.put(X + 1, hy + 2, trunc(l2, w2), { fg: T.text, bg: T.card });
  }
  actionBar(g, T, x, y + h - 2, w, [['↑↓', 'move'], ['←→', 'change'], ['Tab', 'next section'], ['Esc', 'close']], T.card2);
  g.putSegs(x + w - 22, y + h - 2, [{ t: 'config.toml  ', fg: T.dim, bg: T.card2 }, ...cap(T, 'o', T.card2), { t: ' open', fg: T.text, bg: T.card2 }]);
}
function keepRow(g, R, i, fn) {
  const anc = R.find(r => r.act && r.act.t === 'sel' && r.act.i === i); const keep = [];
  if (anc) for (let y = anc.y; y < anc.y + anc.h; y++) for (let x = anc.x; x < anc.x + anc.w; x++) keep.push([x, y, { ...g.c[y][x] }]);
  g.dimAll(); keep.forEach(([x, y, c]) => { g.c[y][x] = c; }); return anc;
}
function drawHelpers(g, R, T, s, px, pw, H) {
  const anc = keepRow(g, R, 4); if (!anc) return;
  const w = Math.min(64, pw - 3), x = px + 1, h = 13; let y = Math.max(1, anc.y - 1); if (y + h > H) y = Math.max(1, H - h);
  float(g, T, x, y, w, h, 'START HELPERS? · swift-lynx', 'Esc later');
  const P = (yy, segs) => g.putSegs(x + 2, yy, segs.map(t => ({ bg: T.card, ...t })), {}, x + w - 2);
  P(y + 2, [{ t: 'claude wants to split this into ', fg: T.text }, { t: '4 helpers', fg: T.strong, b: 1 }, { t: ', each in its', fg: T.text }]);
  P(y + 3, [{ t: 'own worktree off dev:', fg: T.text }]);
  TASKS.forEach((t, i) => P(y + 4 + i, [{ t: '  · ', fg: T.dim }, { t, fg: T.strong }]));
  P(y + 9, [{ t: 'Their permission prompts come to your Inbox. Nothing merges.', fg: T.dim }]);
  let bx = x + 2; bx = btn(g, R, bx, y + 11, T, 'Allow', '1', { t: 'hAns', v: 1 }); bx = btn(g, R, bx, y + 11, T, 'Always in seshi', '2', { t: 'hAns', v: 2 }); btn(g, R, bx, y + 11, T, 'No, do it alone', '3', { t: 'hAns', v: 3 });
}
function drawNewOpt(g, R, T, s, W, H) {
  const w = Math.min(W - 4, 72), h = 13, x = Math.floor((W - w) / 2), y = Math.max(1, Math.floor((H - h) / 2)), no = s.no || { row: 0, a: 0, w: 0, p: 0 }, tint = mix(T.card, T.text, 0.1);
  g.dimAll(); float(g, T, x, y, w, h, 'NEW SESSION', 'Esc');
  const rows = [['Agent', 'a', NO.a], ['Where', 'w', NO.w], ['Project', 'p', NO.p]];
  rows.forEach(([l, k, opts], i) => {
    const yy = y + 2 + i * 2, on = no.row === i, bg = on ? tint : T.card;
    g.fill(x + 2, yy, w - 4, 1, bg); if (on) g.put(x + 2, yy, '›', { fg: T.acc, bg, b: 1 });
    g.put(x + 4, yy, l, { fg: on ? T.strong : T.dim, bg, b: on });
    let ox = x + 14;
    if (k === 'p') { g.putSegs(ox, yy, [{ t: '‹ ', fg: T.dim, bg }, { t: ' ' + opts[no.p] + ' ', fg: T.accInk, bg: T.acc, b: 1 }, { t: ' ›', fg: T.dim, bg }, { t: '   ' + (no.p + 1) + ' of ' + opts.length, fg: T.dim, bg }]); R.push({ x: ox, y: yy, w: 20, h: 1, act: { t: 'noSet', row: i, v: (no.p + 1) % opts.length } }); }
    else opts.forEach((o, j) => { const sel = no[k] === j, x0 = ox; ox = g.putSegs(ox, yy, [{ t: ' ' + o + ' ', fg: sel ? T.accInk : T.text, bg: sel ? T.acc : bg, b: sel }]) + 1; R.push({ x: x0, y: yy, w: ox - x0 - 1, h: 1, act: { t: 'noSet', row: i, v: j } }); });
  });
  const ag = NO.a[no.a], wh = NO.w[no.w], pr = NO.p[no.p];
  g.putSegs(x + 4, y + 8, [{ t: 'Opens ', fg: T.text, bg: T.card }, { t: ag, fg: T.strong, b: 1, bg: T.card }, { t: wh === 'new worktree' ? ' in a new worktree of ' : ' in the folder of ', fg: T.text, bg: T.card }, { t: pr, fg: T.strong, b: 1, bg: T.card }, { t: '. You type the task there.', fg: T.text, bg: T.card }], {}, x + w - 2);
  let bx = x + 4; bx = btn(g, R, bx, y + 10, T, 'Start', 'Enter', { t: 'newNow' }); btn(g, R, bx, y + 10, T, 'Cancel', 'Esc', { t: 'close' });
  g.putSegs(x + w - 26, y + 10, hints(T, [['↑↓', 'rows'], ['←→', 'pick']], T.card));
}
const CMDS = [
  ['Sessions', 'New session', 'n', { t: 'newNow' }], ['Sessions', 'New session…', 'c', { t: 'modal', v: 'newopt' }], ['Sessions', 'Reply to this session', 'r', { t: 'msgOpen' }], ['Sessions', 'Menu', 'm', { t: 'menu' }], ['Sessions', 'Inbox', 'j', { t: 'sheet', v: 'inbox' }], ['Sessions', 'Why this status', 'i', 'Explains the selected row'],
  ['Panes', 'Split right', 'p', 'Split right'], ['Panes', 'Split down', '-', 'Split down'], ['Panes', 'Close pane', 'x', 'Close pane'], ['Panes', 'Move focus', '←↑↓→', null], ['Panes', 'Collapse sidebar', 'z', { t: 'rail' }], ['Panes', 'Hide sidebar', 'Z', { t: 'hide' }],
  ['Tabs', 'New tab', 't', 'New tab'], ['Tabs', 'Next tab', ']', 'Next tab'], ['Tabs', 'Previous tab', '[', 'Previous tab'], ['Tabs', 'Rename tab', 'R', 'Rename tab'],
  ['Project', 'Worktrees ›', 'w', 'Worktrees (second step)'], ['Project', 'Files', 'f', 'Files'], ['Project', 'Changes', 'd', 'Changes'],
  ['Seshi', 'Settings', ',', { t: 'modal', v: 'settings' }], ['Seshi', 'All keys', '?', { t: 'cmd', v: 'all' }], ['Seshi', 'Quit · agents keep going', 'q', 'Quit (mock)'],
  ['No leader', 'Go to tab 1–9', 'Alt+1–9', null], ['No leader', 'Move focus', 'Alt+arrows', null],
  ['Search only', 'Stop agent', '', 'Stops the agent (Ctrl+C)'], ['Search only', 'Clean up worktrees', '', 'Clean up worktrees'], ['Search only', 'Remove worktree', '', 'Remove worktree'], ['Search only', 'Check for updates', '', 'Up to date'], ['Search only', 'Pane info', '', 'Pane info'], ['Search only', 'Search history', '', 'Search history'], ['Search only', 'Set up agent status', '', { t: 'modal', v: 'settings' }],
];
const cmdAct = c => c[3] == null ? null : typeof c[3] === 'string' ? { t: 'toast', v: c[3] } : c[3];
const RECENT = ['Inbox', 'Reply to this session', 'Split right', 'Collapse sidebar'].map(l => CMDS.findIndex(c => c[1] === l));
const found = s => s.q ? CMDS.map((c, i) => i).filter(i => CMDS[i][1].toLowerCase().includes(s.q.toLowerCase()) || CMDS[i][0].toLowerCase().includes(s.q.toLowerCase())) : RECENT;
function drawCmd(g, R, T, s, W, H) {
  const all = s.cmd === 'all', srch = s.cmd === 'search', w = Math.min(W - 4, 112), h = Math.min(H - 2, all ? 33 : 25), x = Math.floor((W - w) / 2), y = Math.max(1, Math.floor((H - h) / 2)), C = T.card, tint = mix(C, T.text, 0.1);
  g.dimAll(); float(g, T, x, y, w, h, all ? 'ALL KEYS' : srch ? 'SEARCH COMMANDS' : 'Ctrl+Space', 'Esc close');
  g.fill(x + 2, y + 2, w - 4, 1, T.bg);
  g.putSegs(x + 3, y + 2, [{ t: '› ', fg: T.acc, b: 1, bg: T.bg }, ...(srch ? [{ t: s.q, fg: T.strong, b: 1, bg: T.bg }, { t: '█', fg: T.acc, bg: T.bg }] : [{ t: '█', fg: T.acc, bg: T.bg }, { t: all ? ' type to search' : ' press a key, or type to search', fg: T.dim, bg: T.bg }])]);
  if (srch) {
    const list = found(s); let yy = y + 4;
    g.put(x + 3, yy++, s.q ? list.length + ' match' + (list.length === 1 ? '' : 'es') : 'RECENT', { fg: T.dim, bg: C, b: 1 });
    list.slice(0, h - 8).forEach((ci, i) => {
      const c = CMDS[ci], on = i === s.ci, bg = on ? tint : C; g.fill(x + 2, yy, w - 4, 1, bg); if (on) g.set(x + 2, yy, '▎', { fg: T.acc, bg });
      const lab = c[1], at = s.q ? lab.toLowerCase().indexOf(s.q.toLowerCase()) : -1;
      const segs = at >= 0 ? [{ t: lab.slice(0, at), fg: T.strong, bg }, { t: lab.slice(at, at + s.q.length), fg: T.acc, b: 1, bg }, { t: lab.slice(at + s.q.length), fg: T.strong, bg }] : [{ t: lab, fg: T.strong, bg }];
      g.putSegs(x + 4, yy, segs); g.put(x + w - 30, yy, c[0], { fg: T.dim, bg }); if (c[2]) g.putSegs(x + w - 4 - capLen(c[2]), yy, cap(T, c[2], bg));
      R.push({ x: x + 2, y: yy, w: w - 4, h: 1, act: { t: 'cmdRun', i: ci } }); yy++;
    });
    if (!list.length) g.put(x + 4, yy, 'Nothing matches "' + s.q + '".', { fg: T.dim, bg: C });
    actionBar(g, T, x, y + h - 2, w, [['↑↓', 'move'], ['Enter', 'run'], ['Tab', 'key map'], ['Esc', 'close']], T.card2);
    return;
  }
  const three = w >= 100, cols = three ? [['Sessions', 'Project'], ['Panes', 'Seshi'], ['Tabs', ...(all ? ['No leader', 'Search only'] : [])]] : [['Sessions', 'Project', 'Seshi', ...(all ? ['No leader'] : [])], ['Panes', 'Tabs']];
  const cw = Math.floor((w - 6) / cols.length), maxY = y + h - 4;
  g.putSegs(x + 3, y + 4, [{ t: 'Every key below goes after the leader:  ', fg: T.text, bg: C }, ...cap(T, 'Ctrl+Space', C), { t: '  then  ', fg: T.dim, bg: C }, ...cap(T, 'p', C), { t: '  = split right', fg: T.dim, bg: C }], {}, x + w - 3);
  g.hl(x + 3, y + 5, w - 6, '─', { fg: T.line, bg: C });
  cols.forEach((col, ci) => { let yy = y + 7; const X = x + 3 + ci * cw;
    col.forEach(gname => { if (yy > maxY) return; const nl = gname === 'No leader'; g.putSegs(X, yy++, [{ t: nl ? 'WITHOUT THE LEADER' : gname.toUpperCase(), fg: gname === 'Search only' || nl ? T.dim : T.strong, bg: C, b: 1 }, ...(nl ? [{ t: '  press directly', fg: T.dim, bg: C }] : [])]);
      CMDS.forEach((c, i) => { if (c[0] !== gname || yy > maxY) return; const k = c[2] || '—';
        const kw = Math.max(...CMDS.filter(cc => cc[0] === gname).map(cc => capLen(cc[2] || '—'))) + 2;
        const ks = c[2] ? cap(T, c[2], C) : [{ t: ' — ', fg: T.dim, bg: C }], kl = c[2] ? capLen(c[2]) : 3;
        g.putSegs(X, yy, [...ks, { t: ' '.repeat(Math.max(1, kw - kl)), bg: C }, { t: trunc(c[1], cw - kw - 2), fg: T.text, bg: C }]); if (c[3]) R.push({ x: X, y: yy, w: cw - 2, h: 1, act: { t: 'cmdRun', i } }); yy++; });
      yy++; }); });
  const foot = all ? (three ? 'Type to search. Search-only commands have no key.' : 'Type to search. + 7 search-only commands.') : 'Keys with › open a second step.';
  g.put(x + 3, y + h - 3, foot, { fg: T.dim, bg: C });
  actionBar(g, T, x, y + h - 2, w, all ? [['Tab', 'search'], ['Esc', 'close']] : [['key', 'run it'], ['Tab', 'search'], ['?', 'show all'], ['Esc', 'close']], T.card2);
}
// One toast for everything Seshi tells you: a real card at the bottom-right of the screen, above the agent's input area.
function drawToast(g, R, T, W, H, t) {
  const lines = [t.title, ...(t.detail ? [t.detail] : []), ...(t.preview ? [t.preview] : [])];
  const w = Math.min(W - 4, Math.max(40, ...lines.map(l => L(l) + 8))), h = lines.length + 2, x = W - w - 2, y = H - h - 5;
  g.fill(x, y, w, h, T.card);
  const S = { fg: T.line, bg: T.card }; g.hl(x + 1, y, w - 2, '─', S); g.hl(x + 1, y + h - 1, w - 2, '─', S); g.set(x + w - 1, y, '┐', S); g.set(x + w - 1, y + h - 1, '┘', S); g.vl(x + w - 1, y + 1, h - 2, '│', S);
  for (let j = 0; j < h; j++) g.set(x, y + j, '▌', { fg: t.c || T.acc, bg: T.card });
  g.putSegs(x + 2, y + 1, [{ t: (t.glyph || '●') + ' ', fg: t.c || T.acc, b: 1, bg: T.card }, { t: trunc(t.title, w - 10), fg: T.strong, b: 1, bg: T.card }]);
  g.putSegs(x + w - 4, y + 1, [{ t: '✕', fg: T.dim, bg: T.card }]); R.push({ x: x + w - 4, y: y + 1, w: 2, h: 1, act: { t: 'toastClose' } });
  if (t.detail) g.put(x + 4, y + 2, trunc(t.detail, w - 6), { fg: T.text, bg: T.card });
  if (t.preview) g.put(x + 4, y + 3, trunc(t.preview, w - 6), { fg: T.dim, bg: T.card, i: 1 });
}
function drawKeys(g, R, T, s, W, H) {
  const w = Math.min(W - 4, 96), h = Math.min(H - 2, 24), x = Math.floor((W - w) / 2), y = Math.max(1, Math.floor((H - h) / 2));
  g.dimAll(); float(g, T, x, y, w, h, 'ALL KEYS', 'Esc close');
  const cols = [[['SESSIONS', [['↑ ↓', 'select a session'], ['Enter', 'open it'], ['m', 'message it'], ['+', 'new session, instantly'], ['c', 'new session with options']]], ['PANELS', [['Alt+arrows', 'move between panes'], ['z', 'collapse sidebar'], ['Z', 'hide sidebar'], ['Esc', 'close a panel']]]],
    [['GO TO', [['j', 'Inbox'], ['a', 'Actions menu'], [',', 'Settings'], ['Space', 'Command palette']]], ['ANYWHERE', [['Ctrl+Space', 'then any key above'], ['', 'works even inside an agent pane']]]]];
  const cw = Math.floor((w - 6) / 2);
  cols.forEach((col, ci) => { let yy = y + 2; const X = x + 3 + ci * cw; col.forEach(([hd, ks]) => { g.put(X, yy++, hd, { fg: T.dim, bg: T.card, b: 1 }); ks.forEach(([k, d]) => { g.putSegs(X, yy++, [{ t: pad(k, 12), fg: T.acc, b: 1, bg: T.card }, { t: d, fg: T.text, bg: T.card }], {}, X + cw - 2); }); yy++; }); });
  g.put(x + 3, y + h - 2, 'The footer always shows + new · j inbox · a actions · ? keys.', { fg: T.dim, bg: T.card });
}

export function render(s) {
  const T = THEMES[s.theme] || THEMES.seshi, W = s.W, H = s.H, g = new Grid(W, H, T), R = [];
  g.fill(0, 0, W, 1, T.card);
  const SW = s.hide || s.rail ? 0 : W >= 140 ? 36 : 28;
  const SW2 = s.rail ? 0 : SW; if (SW2) drawSide(g, R, T, s, SW2, H);
  const px = SW ? SW - 1 : 0, beside = s.sheet && W >= 140, SHW = 70, pw = beside ? W - px - SHW + 1 : W - px;
  const rule = (x, lit) => { if (x <= 0) return; g.set(x, 0, '│', { fg: T.line, bg: T.card }); g.vl(x, 1, H - 1, '│', { fg: lit ? T.acc : T.line }); };
  if (!(s.sheet && !beside)) { drawPane(g, R, T, s, px, pw, H); rule(px, s.focus === 'pane'); }
  if (s.sheet) { const sx = beside ? px + pw - 1 : px; drawInbox(g, R, T, s, sx, beside ? SHW : W - px, H); rule(sx, s.focus === 'sheet'); }
  if (s.rail) drawStrip(g, R, T, s, W);
  if (s.hpop) drawHelpers(g, R, T, s, px, pw, H);
  if (s.modal === 'newopt') drawNewOpt(g, R, T, s, W, H);
  if (s.menu != null) drawMenu(g, R, T, s, H);
  if (s.msg) drawMsg(g, R, T, s, px, pw);
  if (s.modal === 'settings') drawSettings(g, R, T, s, W, H);
  if (s.cmd) drawCmd(g, R, T, s, W, H);
  if (s.copy === 'done') drawToast(g, R, T, W, H, { glyph: '✓', c: T.ok, title: 'Copied to clipboard', detail: '214 characters · 5 lines from swift-lynx', preview: '"● 4 background agents launched (↓ to manage) ├ Find and fix…"' });
  else if (s.toast) drawToast(g, R, T, W, H, { title: s.toast });
  if (s.leader) { const lb = ' Ctrl+Space … '; g.putSegs(W - L(lb) - 1, 0, [{ t: lb, fg: T.accInk, bg: T.acc, b: 1 }]); }
  if (s.hx != null) for (let k = R.length - 1; k >= 0; k--) { const q = R[k]; if (s.hx >= q.x && s.hx < q.x + q.w && s.hy >= q.y && s.hy < q.y + q.h) { for (let yy = q.y; yy < q.y + q.h; yy++) for (let xx = q.x; xx < q.x + q.w; xx++) { const c = g.c[yy] && g.c[yy][xx]; if (c) c.bg = mix(c.bg, T.text, 0.1); } break; } }
  return { rows: g.rows(), regions: R, W, H };
}

export function act(s, a) {
  const S = p => ({ state: { ...s, toast: null, ...p } });
  const toast = v => ({ state: { ...s, menu: null, toast: v }, fx: [{ after: 2400, act: { t: 'clearToast' } }] });
  const base = { menu: null, msg: 0, modal: null, hpop: 0, cmd: null }, ALL = allOf(s);
  switch (a.t) {
    case 'clearToast': return S({});
    case 'copyClear': return S({ copy: null });
    case 'toastClose': return S({ copy: s.copy === 'done' ? null : s.copy, toast: null });
    case 'toast': return toast(a.v);
    case 'leader': return S({ ...base, cmd: s.cmd ? null : 'map', q: '', ci: 0, focus: s.cmd ? 'side' : 'modal' });
    case 'cmd': return S({ ...base, cmd: a.v, q: '', ci: 0, focus: a.v ? 'modal' : 'side' });
    case 'cmdRun': { const c = CMDS[a.i], next = cmdAct(c); const closed = { ...s, cmd: null, focus: 'side' }; return next ? act(closed, next) : S({ cmd: null, focus: 'side' }); }
    case 'msgOpen': return S({ ...base, msg: 1, focus: 'msg' });
    case 'sel': return S({ ...base, sel: a.i, focus: 'side' });
    case 'focus': return S({ ...base, focus: a.v });
    case 'sheet': return S({ ...base, sheet: a.v, isel: 0, focus: a.v ? 'sheet' : 'side' });
    case 'isel': return S({ isel: a.i, focus: 'sheet' });
    case 'menu': return S({ ...base, menu: 0, focus: 'menu' });
    case 'menuRun': return act({ ...s, menu: null, focus: 'side' }, menuAct(MITEMS[a.i]));
    case 'modal': return S({ ...base, modal: a.v, tab: 0, row: 0, focus: 'modal', ...(a.v === 'newopt' ? { no: { row: 0, a: 0, w: 0, p: 0 } } : {}) });
    case 'hide': return S({ ...base, hide: !s.hide, rail: 0, focus: s.hide ? 'side' : 'pane', toast: s.hide ? null : 'Sidebar hidden · Z brings it back' });
    case 'rail': return S({ ...base, rail: s.rail ? 0 : 1, hide: false, focus: s.rail ? 'side' : 'pane' });
    case 'tab1': return S({ paneNew: 0, focus: 'pane' });
    case 'newNow': { const ag = s.no ? NO.a[s.no.a] : 'claude'; return S({ ...base, newS: 'bright-otter', newAgent: ag, sel: 5, paneNew: 1, focus: 'pane', no: null }); }
    case 'noSet': { const no = { ...(s.no || { row: 0, a: 0, w: 0, p: 0 }) }; no.row = a.row; no[['a', 'w', 'p'][a.row]] = a.v; return S({ no }); }
    case 'hAns': return S({ hpop: 0, helpers: a.v === 3 ? 'no' : 'ok', toast: a.v === 3 ? 'swift-lynx will do it alone' : a.v === 2 ? 'Always for seshi · helpers starting' : null });
    case 'msgFor': return S({ ...base, sel: a.i, msg: 1, focus: 'msg' });
    case 'send': return { state: { ...s, msg: 0, focus: 'side', toast: 'Sent to ' + allOf(s)[s.sel].id }, fx: [{ after: 2400, act: { t: 'clearToast' } }] };
    case 'close': return S({ ...base, focus: s.sheet ? 'sheet' : 'side' });
    case 'tab': return S({ tab: a.v, row: 0 });
    case 'row': return S({ row: a.v });
    case 'opt': { const r = selRows(s.tab)[a.row]; return S({ row: a.row, opt: { ...s.opt, [s.tab + r.l]: a.v }, ...(r.l === 'Count on the session row' ? { subs: a.v === 0 } : {}) }); }
    case 'key': {
      const k = a.key;
      if (s.rail && s.focus === 'side' && (k === 'ArrowRight' || k === 'ArrowLeft')) return S({ sel: Math.max(0, Math.min(ALL.length - 1, s.sel + (k === 'ArrowRight' ? 1 : -1))) });
      if (s.modal === 'settings') {
        const rows = selRows(s.tab);
        if (k === 'Escape') return act(s, { t: 'close' });
        if (k === 'Tab') return S({ tab: (s.tab + 1) % TABS.length, row: 0 });
        if (k === 'ArrowDown') return S({ row: Math.min(rows.length - 1, s.row + 1) });
        if (k === 'ArrowUp') return S({ row: Math.max(0, s.row - 1) });
        if ((k === 'ArrowLeft' || k === 'ArrowRight') && rows[s.row] && rows[s.row].o.length > 1) { const r = rows[s.row], cur = s.opt[s.tab + r.l] ?? (r.d || 0), v = (cur + (k === 'ArrowRight' ? 1 : r.o.length - 1)) % r.o.length; return act(s, { t: 'opt', row: s.row, v }); }
        return null;
      }
      if (s.cmd) {
        if (k === 'Escape') return act(s, { t: 'cmd', v: null });
        if (s.cmd === 'search') {
          const list = found(s);
          if (k === 'Tab') return S({ cmd: 'map' }); if (k === 'ArrowDown') return S({ ci: Math.min(list.length - 1, s.ci + 1) }); if (k === 'ArrowUp') return S({ ci: Math.max(0, s.ci - 1) });
          if (k === 'Enter') return list[s.ci] != null ? act(s, { t: 'cmdRun', i: list[s.ci] }) : null;
          if (k === 'Backspace') return S({ q: Array.from(s.q).slice(0, -1).join(''), ci: 0 });
          if (Array.from(k).length === 1) return S({ q: s.q + k, ci: 0 }); return null;
        }
        if (k === 'Tab') return S({ cmd: 'search', q: '', ci: 0 }); if (k === '?' && s.cmd === 'map') return S({ cmd: 'all' });
        const i = CMDS.findIndex(c => c[2] === k); if (i >= 0) return act(s, { t: 'cmdRun', i });
        if (Array.from(k).length === 1 && k !== ' ') return S({ cmd: 'search', q: k, ci: 0 }); return null;
      }
      if (s.modal === 'newopt') {
        const no = s.no || { row: 0, a: 0, w: 0, p: 0 }, key = ['a', 'w', 'p'][no.row], n = NO[key].length;
        if (k === 'Escape') return act(s, { t: 'close' }); if (k === 'Enter') return act(s, { t: 'newNow' });
        if (k === 'ArrowDown') return S({ no: { ...no, row: Math.min(2, no.row + 1) } }); if (k === 'ArrowUp') return S({ no: { ...no, row: Math.max(0, no.row - 1) } });
        if (k === 'ArrowRight' || k === 'ArrowLeft') return act(s, { t: 'noSet', row: no.row, v: (no[key] + (k === 'ArrowRight' ? 1 : n - 1)) % n });
        return null;
      }
      if (s.hpop) { if (k === 'Escape') return S({ hpop: 0 }); if (['1', '2', '3'].includes(k)) return act(s, { t: 'hAns', v: +k }); return null; }
      if (s.modal === 'keys') return k === 'Escape' || k === '?' ? act(s, { t: 'close' }) : null;
      if (s.msg) { if (k === 'Escape') return act(s, { t: 'close' }); if (k === 'Enter') return act(s, { t: 'send' }); return null; }
      if (s.menu != null) {
        if (k === 'Escape') return S({ menu: null, focus: 'side' });
        if (k === 'ArrowDown' || k === 'j') return S({ menu: Math.min(MITEMS.length - 1, s.menu + 1) });
        if (k === 'ArrowUp' || k === 'k') return S({ menu: Math.max(0, s.menu - 1) });
        if (k === 'Enter') return act(s, { t: 'menuRun', i: s.menu });
        const i = MITEMS.findIndex(m => m[1] === k || (k === ' ' && m[1] === 'Space')); return i >= 0 ? act(s, { t: 'menuRun', i }) : null;
      }
      if (s.focus === 'sheet') {
        if (k === 'Escape') return act(s, { t: 'sheet', v: null });
        if (k === 'ArrowDown') return S({ isel: Math.min(2, s.isel + 1) }); if (k === 'ArrowUp') return S({ isel: Math.max(0, s.isel - 1) });
        if (k === 'r') return act(s, { t: 'msgFor', i: 2 }); if (k === 'Enter') return toast('Opens the item');
      }
      if (k === 'm') return act(s, { t: 'menu' }); if (k === 'n') return act(s, { t: 'newNow' }); if (k === 'j' && s.focus !== 'side') return act(s, { t: 'sheet', v: 'inbox' });
      if (k === '?') return act(s, { t: 'cmd', v: 'all' }); if (k === ' ' && s.focus === 'side') return act(s, { t: 'cmd', v: 'search' }); if (k === ',') return act(s, { t: 'modal', v: 'settings' });
      if (k === 'z') return act(s, { t: 'rail' }); if (k === 'Z') return act(s, { t: 'hide' }); if (k === 'r') return S({ msg: 1, focus: 'msg' });
      if (k === '+') return act(s, { t: 'newNow' }); if (k === 'c') return act(s, { t: 'modal', v: 'newopt' });
      if (s.focus === 'side' && /^[1-9]$/.test(k)) { const q = ALL[s.sel]; if (q && q.id === 'swift-lynx' && s.helpers === 'ask' && +k <= 3) return act(s, { t: 'hAns', v: +k }); if (s.rail && +k <= ALL.length) return S({ sel: +k - 1 }); }
      if (s.focus === 'side' && k === 'Enter' && ALL[s.sel] && ALL[s.sel].id === 'swift-lynx' && s.helpers === 'ask') return S({ hpop: 1 });
      if (s.focus === 'side') {
        if (k === 'ArrowDown' || k === 'j') return S({ sel: Math.min(ALL.length - 1, s.sel + 1) }); if (k === 'ArrowUp' || k === 'k') return S({ sel: Math.max(0, s.sel - 1) });
        if (k === 'Enter') return S({ focus: 'pane' }); if (k === 'i') return act(s, { t: 'sheet', v: 'inbox' });
      }
      if (s.focus === 'pane' && k === 'y') return s.copy === 'sel' ? { state: { ...s, copy: 'done' }, fx: [{ after: 2400, act: { t: 'copyClear' } }] } : S({ copy: 'sel' });
      if (s.focus === 'pane' && k === 'Escape' && s.copy) return S({ copy: null });
      if (s.focus === 'pane' && k === 'Escape') return S({ focus: 'side' });
      return null;
    }
  }
  return null;
}
