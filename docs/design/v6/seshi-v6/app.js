// Seshi v6 · sessions that call on specialists. Cell-grid spec. SAMPLE DATA ONLY.
import { Grid, trunc, segLen, L, mix } from './seshi-grid.js';

export const THEMES = {
  pixls: { name: 'dark', label: 'Seshi Pixls', bg: '#141413', fg: '#c9c5bb', card: '#1c1c1a', card2: '#262624', btn: '#2e2d2a', hov: '#3a3935', line: '#3a3934', dim: '#8a867c', text: '#c9c5bb', strong: '#ece9e1',
    acc: '#ff6a2f', accInk: '#141413', needs: '#f2c200', work: '#c9c5bb', ok: '#3cc27e', err: '#ff5a52', rev: '#b58cff', wait: '#8fa6ff', sky: '#5aa9ff', scrim: '#000000',
    a: { green: '#7fd962', red: '#ff6b6b', cyan: '#3dd6c0', blue: '#5aa9ff', yellow: '#e8c565', magenta: '#a593ff', gray: '#8a867c' } },
  night: { name: 'dark', label: 'Seshi Night', bg: '#070b10', fg: '#c9d1d9', card: '#0f1821', card2: '#18242f', btn: '#1d2a37', hov: '#2a3a4c', line: '#1f2c3a', dim: '#71808f', text: '#a7b4c2', strong: '#f2f6f8',
    acc: '#c3f53c', accInk: '#0a1204', needs: '#ffb547', work: '#a7b4c2', ok: '#7fd962', err: '#ff6b6b', rev: '#a593ff', wait: '#5aa9ff', sky: '#5aa9ff', scrim: '#000000',
    a: { green: '#7fd962', red: '#ff6b6b', cyan: '#3dd6c0', blue: '#5aa9ff', yellow: '#e8c565', magenta: '#a593ff', gray: '#71808f' } },
};
const GL = {
  unicode: { planned: '○', queued: '…', work: ["⣰⡄","⣼⡀","⣶⡆","⣧⡇","⣆⡆","⣤⡄","⣸⡆","⣴⡇","⣇⡄","⣦⡀"], waiting: '↳', needs: '●', reviewing: '◎', changes: '↺', done: '✓', failed: '✕', stopped: '■', interrupted: '?', idle: '○', verified: '◆', down: '↓', up: '↑', branch: '⎇', bar: '▎', back: '‹', close: '✕', fold: '▾', unfold: '▸', warn: '⚠' },
  ascii: { planned: 'o', queued: '.', work: ['.', 'o', 'O', 'o'], waiting: '>', needs: '!', reviewing: '@', changes: '~', done: '+', failed: 'x', stopped: '#', interrupted: '?', idle: 'o', verified: '*', down: 'v', up: '^', branch: 'git:', bar: '|', back: '<', close: 'x', fold: 'v', unfold: '>', warn: '!' },
};
GL.nerd = { ...GL.unicode, planned: '\uf10c', queued: '\uf141', waiting: '\uf017', needs: '\uf0f3', reviewing: '\uf06e', changes: '\uf021', done: '\uf00c', failed: '\uf00d', stopped: '\uf04d', interrupted: '\uf128', idle: '\uf10c', verified: '\uf132', branch: '\ue0a0', back: '\uf104', close: '\uf00d', fold: '\uf107', unfold: '\uf105', warn: '\uf071' };
const SPEC = {
  forge: { n: 'Forge', k: 'F', c: '#5aa9ff', role: 'backend', model: 'codex · gpt-5-codex · high', prov: 'codex', desc: 'Server APIs, database, payments. Writes tests for every endpoint.', refs: 'docs/API_PATTERNS.md, docs/DATABASE.md', lim: { edit: 'worktree', run: 'tests, lint, build', net: 'ask', commit: 'task branch', merge: 'never', publish: 'never' }, auto: true },
  canvas: { n: 'Canvas', k: 'C', c: '#ff7ab6', role: 'frontend', model: 'codex · gpt-5-codex · medium', prov: 'codex', desc: 'Pages and components. Builds against the agreed interface.', refs: 'docs/UI.md', lim: { edit: 'worktree', run: 'tests, lint, build', net: 'ask', commit: 'task branch', merge: 'never', publish: 'never' }, auto: true },
  lens: { n: 'Lens', k: 'L', c: '#3dd6c0', role: 'reviewer', model: 'codex · gpt-5-codex · high', prov: 'codex', desc: 'Reviews the exact combined code and runs its checks. Never edits.', refs: 'docs/TESTING.md', lim: { edit: 'none', run: 'tests, e2e', net: 'never', commit: 'never', merge: 'never', publish: 'never' }, auto: true },
  scribe: { n: 'Scribe', k: 'S', c: '#a593ff', role: 'docs', model: 'claude · sonnet', prov: 'claude', desc: 'Writes and updates docs from merged changes.', refs: 'README.md', lim: { edit: 'worktree', run: 'none', net: 'never', commit: 'task branch', merge: 'never', publish: 'never' }, auto: false },
};
const TASK = {
  T1: { title: 'checkout session endpoint', sp: 'forge', deps: [], ac: 'AC1', tree: 't1-checkout' },
  T2: { title: 'webhook + commission ledger', sp: 'forge', deps: ['T1'], ac: 'AC2 AC3', tree: 't2-webhook' },
  T3: { title: 'buy button + redirect', sp: 'canvas', deps: [], ac: 'AC1', tree: 't3-buy-button' },
  T4: { title: 'order status page', sp: 'canvas', deps: ['T3'], ac: 'AC4', tree: 't4-order-page' },
  R: { title: 'review the combined result', sp: 'lens', deps: ['T1', 'T2', 'T3', 'T4'], ac: 'AC1–AC5', tree: 'add-payments' },
};
const STAGES = ['solo', 'plan', 'missing', 'work', 'needs', 'restart', 'failed', 'review', 'delivered'];
function tasksFor(st, s) {
  const m = {}, set = (id, k, note, x) => { m[id] = { k, note, ...(x || {}) }; };
  if (st === 'solo' || st === 'empty') return m;
  if (st === 'plan' || st === 'missing') { Object.keys(TASK).forEach(id => set(id, 'planned', 'in plan v2 · not approved')); if (st === 'missing') { set('T3', 'planned', 'no compatible specialist', { warn: 1 }); set('T4', 'planned', 'no compatible specialist', { warn: 1 }); } return m; }
  if (st === 'work') { set('T1', s.stopped === 'T1' ? 'stopped' : 'working', s.stopped === 'T1' ? 'stopped by you · worktree kept' : 'running npm test -- checkout', { ago: '12s' }); set('T3', 'working', 'editing components/BuyButton.tsx', { ago: '4s' }); set('T2', 'waiting', 'after T1'); set('T4', 'waiting', 'after T3'); set('R', 'queued', 'after T1–T4'); return m; }
  if (st === 'needs') { set('T1', 'done', 'checked · review passed 14:19'); set('T2', s.answered ? 'working' : 'needs', s.answered ? 'installing stripe@16' : 'asks: npm install stripe@16 (network)', { ago: s.answered ? 'now' : '3m' }); set('T3', 'done', 'checked · review passed 14:36'); set('T4', 'working', 'editing pages/order/[id].tsx', { ago: '8s' }); set('R', 'queued', 'after T2, T4'); return m; }
  if (st === 'restart') { set('T1', 'done', 'review passed'); set('T2', 'working', 'writing tests/webhook.test.ts', { ago: '20s' }); set('T3', 'done', 'review passed');
    set('T4', s.t4 === 'retry' ? 'working' : s.t4 === 'stopped' ? 'stopped' : 'interrupted', s.t4 === 'retry' ? 'attempt 2 · keeps attempt 1 changes' : s.t4 === 'stopped' ? 'stopped by you · worktree kept' : 'attempt 1 · outcome unknown since 14:58'); set('R', 'queued', 'after T2, T4'); return m; }
  if (st === 'failed') { set('T1', 'done', 'review passed'); set('T2', 'done', 'review passed 15:08'); set('T3', 'done', 'review passed'); set('T4', s.t4 === 'retry' ? 'working' : 'failed', s.t4 === 'retry' ? 'attempt 3 · fixing 2 tsc errors' : 'attempt 2 · tsc: 2 errors'); set('R', 'waiting', 'after T4'); return m; }
  if (st === 'review') { ['T1', 'T3'].forEach(id => set(id, 'done', 'review passed')); set('T2', 'done', 'repair for F1 verified'); set('T4', 'changes', 'repair attempt · editing pages/order/[id].tsx', { ago: '6s', live: 1 }); set('R', 'changes', 'review #1: 2 findings'); return m; }
  ['T1', 'T2', 'T3', 'T4'].forEach(id => set(id, 'done', 'review passed')); set('R', 'done', 'review #2 passed 15:32'); return m;
}
const KL = { planned: 'planned', queued: 'queued', working: 'working', waiting: 'waiting', needs: 'needs you', reviewing: 'reviewing', changes: 'changes requested', done: 'completed', failed: 'failed', stopped: 'stopped', interrupted: 'interrupted' };
const GROUP = [['needs', 'NEEDS YOU'], ['interrupted', 'INTERRUPTED · OUTCOME UNKNOWN'], ['failed', 'FAILED'], ['changes', 'CHANGES REQUESTED'], ['working', 'WORKING'], ['reviewing', 'REVIEWING'], ['waiting', 'WAITING ON ANOTHER TASK'], ['queued', 'QUEUED'], ['stopped', 'STOPPED'], ['planned', 'PLANNED · NOT APPROVED'], ['done', 'COMPLETED']];
const stColor = (T, k) => ({ needs: T.needs, interrupted: T.err, failed: T.err, changes: T.err, working: T.work, reviewing: T.rev, waiting: T.wait, queued: T.dim, stopped: T.dim, planned: T.dim, done: T.ok })[k] || T.text;
const stGlyph = (G, k, f) => k === 'working' ? G.work[(f || 0) % G.work.length] : G[k] || '·';

export const PRESETS = [
  ['01 Empty library', { stage: 'solo', view: 'library', lib: 'empty' }],
  ['02 New specialist', { stage: 'solo', view: 'library', libSel: 'forge', libEdit: 1 }],
  ['03 Start a session', { stage: 'solo', view: 'newsession' }],
  ['04 Small work, no workers', { stage: 'solo', sheet: 'team' }],
  ['05 Plan proposed', { stage: 'plan', sheet: 'team' }],
  ['06 Missing specialist', { stage: 'missing', sheet: 'team', sel: 2 }],
  ['07 Working', { stage: 'work', sheet: 'team' }],
  ['08 Task detail', { stage: 'work', sheet: 'task', task: 'T1' }],
  ['09 Stop worker?', { stage: 'work', sheet: 'task', task: 'T1', confirm: 'stop' }],
  ['10 Needs you', { stage: 'needs', sheet: 'team' }],
  ['11 Inbox', { stage: 'needs', sheet: 'inbox' }],
  ['12 Restart: interrupted', { stage: 'restart', sheet: 'team', sel: 0 }],
  ['13 Retry?', { stage: 'restart', sheet: 'team', confirm: 'retry' }],
  ['14 Failed attempt', { stage: 'failed', sheet: 'task', task: 'T4' }],
  ['15 Review: changes', { stage: 'review', sheet: 'team' }],
  ['16 Delivered', { stage: 'delivered', sheet: 'team' }],
  ['17 Merge?', { stage: 'delivered', sheet: 'team', confirm: 'merge' }],
  ['18 Sheet closed', { stage: 'needs', sheet: null, focus: 'pane' }],
];
export function preset(name, W, H, fl = {}) {
  const p = (PRESETS.find(x => x[0] === name) || PRESETS[0])[1];
  return { W, H, theme: fl.theme || 'pixls', glyphs: fl.glyphs || 'unicode', f: 0, stage: 'solo', view: 'session', sheet: null, task: null, sel: 0, focus: 'sheet', confirm: null, lib: 'full', libSel: 'forge', libEdit: 0, held: false, answered: false, t4: null, stopped: null, toast: null, leader: false, ...p, focus: p.focus || (p.view && p.view !== 'session' ? 'modal' : p.sheet ? 'sheet' : 'pane') };
}

// ---------- drawing helpers
const pad = (s, n) => { const a = Array.from(s); return a.length >= n ? trunc(s, n) : s + ' '.repeat(n - a.length); };
const K = (T, k, l) => [{ t: k, fg: T.acc, b: 1 }, { t: ' ' + l, fg: T.text }];
const hintSegs = (T, list) => list.flatMap((h, i) => [...(i ? [{ t: '   ' }] : []), ...K(T, h[0], h[1])]);
const BTN = (T, label, key, st) => { const P = st === 'primary' ? [T.acc, T.accInk, T.accInk] : st === 'danger' ? [T.btn, T.err, T.acc] : st === 'warn' ? [T.needs, T.bg, T.bg] : [T.btn, T.strong, T.acc]; return [{ t: ' ' + label + ' ', bg: P[0], fg: P[1], b: 1 }, ...(key ? [{ t: key + ' ', bg: P[0], fg: P[2], b: 1 }] : [])]; };
const specCell = (k, bg) => ({ t: SPEC[k].k, fg: '#141413', bg: SPEC[k].c, b: 1 });
// Pixel heads: 5×4 pixels in 5×2 cells (▀ = top pixel fg, bottom pixel bg). Eyes carry the state.
const ACC = { forge: ['HHHHH', '#e8c565'], canvas: ['.BBB.', '#a593ff'], lens: ['.....', null], scribe: ['HH.HH', '#ece9e1'] };
function head(g, x, y, k, T, st, f, bg, small) {
  if (small) return headSmall(g, x, y, k, T, st, f, bg);
  const p = 1, CW = 7, RH = 3;
  const sp = SPEC[k], dark = '#141413', back = bg || T.bg, dim = ['planned', 'queued', 'waiting', 'stopped'].includes(st), acc = ACC[k];
  const tile = mix(back, sp.c, dim ? 0.07 : 0.2), skin = dim ? mix(sp.c, tile, 0.55) : sp.c;
  const P = Array.from({ length: RH * 2 }, () => Array(CW).fill(null)), S = (r, c, v) => { P[r + p][c + p] = v; };
  for (let c = 0; c < 5; c++) { S(0, c, acc[0][c] !== '.' ? (dim ? mix(acc[1], tile, 0.55) : acc[1]) : (c === 0 || c === 4 ? null : skin)); S(1, c, skin); S(2, c, skin); S(3, c, c === 0 || c === 4 ? null : skin); }
  const look = st === 'working' || st === 'changes' ? [0, -1, 0, 1][(f >> 3) % 4] : st === 'reviewing' ? [0, 1][(f >> 4) % 2] : 0;
  const blink = !dim && (f + k.length * 7) % 34 < 1;
  let eye = dark;
  if (st === 'needs') eye = (f >> 2) % 2 ? T.needs : dark; if (st === 'failed' || st === 'interrupted') eye = T.err; if (dim || blink) eye = mix(skin, dark, 0.35);
  if (k === 'lens') { const ge = dim || blink ? mix(skin, dark, 0.35) : st === 'failed' ? T.err : '#ece9e1'; S(1, 1 + look, ge); S(1, 3 + look, ge); S(1, 2, dark); }
  else { S(1, 1 + look, eye); S(1, 3 + look, eye); }
  if (st === 'done') S(2, 2, dark); if (st === 'failed' || st === 'interrupted') { S(2, 1, dark); S(2, 3, dark); }
  for (let c = 0; c < CW; c++) for (let r = 0; r < RH; r++) g.set(x + c, y + r, '▀', { fg: P[2 * r][c] || tile, bg: P[2 * r + 1][c] || tile });
}
function headSmall(g, x, y, k, T, st, f, bg) {
  const sp = SPEC[k], dark = '#141413', back = bg || T.bg, dim = ['planned', 'queued', 'waiting', 'stopped'].includes(st);
  const tile = mix(back, sp.c, dim ? 0.07 : 0.2), skin = dim ? mix(sp.c, tile, 0.55) : sp.c, blink = !dim && (f + k.length * 7) % 34 < 1;
  let eye = k === 'lens' ? '#ece9e1' : dark; if (st === 'needs') eye = (f >> 2) % 2 ? T.needs : dark; if (st === 'failed' || st === 'interrupted') eye = T.err; if (dim || blink) eye = mix(skin, dark, 0.35);
  const P = Array.from({ length: 4 }, () => Array(5).fill(null));
  P[1][1] = eye; P[1][2] = skin; P[1][3] = eye; P[2][1] = skin; P[2][2] = st === 'done' ? dark : skin; P[2][3] = skin;
  for (let c = 0; c < 5; c++) for (let r = 0; r < 2; r++) g.set(x + c, y + r, '▀', { fg: P[2 * r][c] || tile, bg: P[2 * r + 1][c] || tile });
}
function button(g, R, x, y, T, label, key, st, act) { const segs = BTN(T, label, key, st); const x2 = g.putSegs(x, y, segs); R.push({ x, y, w: x2 - x, h: 1, act }); return x2 + 1; }
function card(g, x, y, w, h, T, lit, title, right) {
  g.fill(x, y, w, 1, T.card);
  if (x > 0) { g.set(x, y, '│', { fg: T.line, bg: T.card }); g.vl(x, y + 1, h - 1, '│', { fg: T.line }); }
  const cb = lit ? T.acc : T.btn, cf = lit ? T.accInk : T.strong;
  if (title) g.putSegs(x + 1, y, [{ t: ' ', bg: cb }, ...title.map(t => ({ ...t, fg: lit ? cf : (t.fg === T.dim ? T.text : t.fg), bg: cb })), { t: ' ', bg: cb }]);
  if (right) { const rs = right.map(t => ({ ...t, bg: T.card })); g.putSegs(x + w - 1 - segLen(rs), y, rs); }
}
const sel = (g, T, x, y, w, on, lit, G) => { if (!on) return; g.fill(x, y, w, 1, mix(T.bg, T.text, 0.12)); g.set(x, y, G.bar, { fg: lit ? T.acc : T.dim, bg: mix(T.bg, T.text, 0.12) }); };

// ---------- terminal content for the session (Codex output is shown as-is; Seshi draws nothing inside it)
function leadLines(T, st, s) {
  const a = T.a, U = t => [{ t: '› ', fg: T.dim }, { t, fg: T.strong }], B = (t, c) => [{ t: '• ', fg: c || a.green }, ...(Array.isArray(t) ? t : [{ t }])], I = t => [{ t: '  ' + t, fg: T.text }], E = [];
  const out = [U('I need to add payments to my affiliate website. Stripe Checkout; the referring affiliate gets 10% of each sale.')];
  if (st === 'solo') return [U('rename the footer copyright to 2026'), E, B([{ t: 'Edit ', b: 1 }, { t: 'src/components/Footer.tsx', fg: a.cyan }]), [{ t: '  - ', fg: a.red }, { t: '© 2025 Affiliate Co.', fg: a.red }], [{ t: '  + ', fg: a.green }, { t: '© 2026 Affiliate Co.', fg: a.green }], E, B('Done. One-line change; no plan or specialists needed.'), E, [{ t: 'Worked for 21s', fg: T.dim }]];
  out.push(E, B([{ t: 'Ran ' }, { t: 'rg -n "price_cents" src db', fg: a.blue }]), [{ t: '  └ 6 matches', fg: T.dim }], E,
    B('Acceptance checks I\'ll hold the work to:'), I('AC1 Buy reaches Stripe Checkout for the right product and price'), I('AC2 exactly one order per payment, even when the webhook retries'), I('AC3 a 10% commission row per order'), I('AC4 the order page says Paid only after the webhook confirms'), I('AC5 an end-to-end test checkout passes on the combined code'), E,
    B('Shared interface (contract v1):'), [{ t: '  POST ', fg: a.yellow }, { t: '/api/checkout/session  { productId, affiliateCode? } → { url }' }], [{ t: '  POST ', fg: a.yellow }, { t: '/api/stripe/webhook    checkout.session.completed' }], [{ t: '  GET  ', fg: a.yellow }, { t: '/api/orders/:id        → { status }' }], E,
    B([{ t: 'Proposed plan v2 to Seshi: ' }, { t: '4 tasks + review', b: 1 }, { t: '. Approve it in Team (Ctrl+Space T) or reply here.' }]));
  if (st === 'missing') out.push(B([{ t: 'T3 and T4 need a frontend specialist. ' }, { t: 'Canvas runs on Claude, which Seshi can\'t dispatch yet.', fg: a.yellow }]));
  if (['work', 'needs', 'restart', 'failed', 'review', 'delivered'].includes(st)) out.push(E, U('approved v2'), E, B('Dispatched T1 → Forge and T3 → Canvas. T2 and T4 wait on them. Lens reviews at the end.'));
  if (['needs', 'restart', 'failed', 'review', 'delivered'].includes(st)) out.push(B('T1 checked and handed to Canvas: endpoint live, unknown products return 400.'), B([{ t: 'T2 is paused on a question for you. ' }, { t: 'It\'s in your Inbox.', fg: a.yellow }]));
  if (['restart', 'failed', 'review', 'delivered'].includes(st)) out.push(E, [{ t: '── seshi daemon restarted 14:58 · session restored ──', fg: T.dim }], B([{ t: 'T4\'s worker was mid-turn. ' }, { t: 'Outcome unknown; I won\'t retry it without you.', fg: a.yellow }]));
  if (['failed', 'review', 'delivered'].includes(st)) out.push(B([{ t: 'T4 attempt 2 failed: ' }, { t: 'tsc found 2 errors in lib/useOrderStatus.ts.', fg: a.red }]));
  if (['review', 'delivered'].includes(st)) out.push(E, B('All four tasks completed and checked on their own. That isn\'t the feature working yet.'), B([{ t: 'Lens tested the combined branch: ' }, { t: 'changes requested', fg: a.red }, { t: ' (F1 webhook signature, F2 order page). Repairs dispatched.' }]));
  if (st === 'delivered') out.push(E, B([{ t: 'Result: ', b: 1 }, { t: 'payments work end to end in Stripe test mode on seshi/add-payments@9f3c2a1.' }]), I('Evidence and remaining items are in Team. Nothing is merged or published.'), E, [{ t: 'Worked for 1h 31m', fg: T.dim }]);
  return out;
}
function drawTerminal(g, x, y, w, h, T, st, s) {
  const lines = []; for (const l of leadLines(T, st, s)) { if (!l.length) { lines.push([]); continue; } let cur = [], n = 0; for (const sg of l) { let t = sg.t; while (L(t)) { const room = w - n; if (L(t) <= room) { cur.push({ ...sg, t }); n += L(t); t = ''; } else { const a = Array.from(t), cut = a.slice(0, room).join(''), sp = cut.lastIndexOf(' '); const take = sp > room * 0.5 ? cut.slice(0, sp + 1) : cut; cur.push({ ...sg, t: take }); lines.push(cur); cur = [{ t: '    ' }]; n = 4; t = a.slice(L(take)).join(''); } } } lines.push(cur); }
  const room = h - 3, shown = lines.slice(Math.max(0, lines.length - room));
  shown.forEach((l, i) => g.putSegs(x, y + room - shown.length + i, l, {}, x + w));
  g.putSegs(x, y + h - 2, [{ t: '› ', fg: T.strong }, { t: 'Ask Codex to do anything', fg: T.dim }], {}, x + w);
  g.putSegs(x, y + h - 1, [{ t: 'gpt-5-codex high', fg: T.a.yellow }, { t: ' · ~\\dev\\affiliate-site · ', fg: T.dim }, { t: 'Context 58% left', fg: T.a.yellow }], {}, x + w);
}

// ---------- sidebar
function teamCounts(ts) { const c = {}; Object.values(ts).forEach(t => { c[t.k] = (c[t.k] || 0) + 1; }); return c; }
function teamSummary(T, G, ts, f) {
  const c = teamCounts(ts), segs = [];
  const add = (k, n) => { if (n) segs.push({ t: stGlyph(G, k, f) + n + ' ', fg: k === 'working' ? T.dim : stColor(T, k) }); };
  if (!Object.keys(ts).length) return [{ t: 'no workers yet', fg: T.dim }];
  if (c.planned) return [{ t: 'plan v2 · ' + c.planned + ' tasks, not approved', fg: T.dim }];
  add('needs', c.needs); add('interrupted', c.interrupted); add('failed', c.failed); add('changes', c.changes); add('working', c.working); add('waiting', c.waiting); add('done', c.done);
  return [{ t: 'team ', fg: T.dim }, ...segs];
}
function drawSidebar(g, R, T, G, s, ts, x, y, w, h) {
  const lit = s.focus === 'side';
  if (w < 12) { // collapsed rail
    card(g, x, y, w, h, T, lit);
    const rows = [['add-payments', s.stage === 'solo' ? 'idle' : sessionState(s, ts)], ['fix-footer', 'idle'], ['warm-finch', 'idle'], ['quick-fox', 'done']];
    rows.forEach(([n, k], i) => { const yy = y + 1 + i; sel(g, T, x + 1, yy, w - 2, i === 0, lit, G); g.putSegs(x + 1, yy, [{ t: stGlyph(G, k === 'needs you' ? 'needs' : k, s.f), fg: stColor(T, k === 'needs you' ? 'needs' : k) }, { t: ' ' + n.slice(0, 2), fg: i === 0 ? T.strong : T.text }], {}, x + w); });
    const nc = (teamCounts(ts).needs || 0); if (nc) g.putSegs(x + 1, y + h - 2, [{ t: G.needs + nc, fg: T.needs, b: 1 }]);
    g.put(x + 1, y + h - 3, G.unfold, { fg: T.dim });
    return;
  }
  card(g, x, y, w, h, T, lit, [{ t: 'SESSIONS', fg: T.strong, b: 1 }, { t: ' 5', fg: T.dim }]);
  let yy = y + 2; const iw = w - 4;
  const proj = (name, meta) => { g.putSegs(x + 2, yy, [{ t: G.fold + ' ', fg: T.acc }, { t: trunc(name, w - 7 - L(meta)), fg: T.strong, b: 1 }]); g.put(x + w - 2 - L(meta), yy, meta, { fg: T.dim }); yy++; };
  const row = (name, agent, k, age, on, extra, sub, last) => {
    const n = 2 + (last ? 1 : 0) + (extra || []).length, tint = mix(T.bg, T.text, 0.12);
    if (on) { g.fill(x + 1, yy, w - 2, n, tint); for (let j = 0; j < n; j++) g.set(x + 1, yy + j, G.bar, { fg: lit ? T.acc : T.dim, bg: tint }); }
    const bg = on ? tint : undefined, kk = k === 'needs you' ? 'needs' : k, m = age;
    g.putSegs(x + 3, yy, [{ t: stGlyph(G, kk, s.f) + ' ', fg: stColor(T, kk), bg }, { t: trunc(name, w - 8 - L(m)), fg: on ? T.strong : T.text, b: 1, bg }]);
    g.put(x + w - 2 - L(m), yy, m, { fg: T.dim, bg }); R.push({ x: x + 1, y: yy, w: w - 2, h: 2, act: { t: 'focusPane' } }); yy++;
    g.putSegs(x + 5, yy++, [{ t: trunc(agent + (sub ? ' · ' + sub : ''), w - 7), fg: T.dim, bg }], {}, x + w - 2);
    if (last) g.putSegs(x + 5, yy++, [{ t: trunc(last, w - 7), fg: T.text, bg }], {}, x + w - 2);
    (extra || []).forEach(segs => { g.putSegs(x + 5, yy, segs.map(q => ({ ...q, bg })), {}, x + w - 2); yy++; });
    yy++;
  };
  proj('affiliate-site', G.branch + ' main');
  const extra = [teamSummary(T, G, ts, s.f)];
  Object.keys(ts).forEach(id => { if (ts[id].k === 'needs' || ts[id].k === 'interrupted' || ts[id].k === 'failed') extra.push([{ t: stGlyph(G, ts[id].k) + ' ', fg: stColor(T, ts[id].k) }, { t: SPEC[TASK[id].sp].n + ': ', fg: stColor(T, ts[id].k) }, { t: trunc(ts[id].note, iw - 12), fg: stColor(T, ts[id].k) }]); });
  if (extra.length > 1 || Object.keys(ts).length) R.push({ x: x + 1, y: yy, w: w - 2, h: extra.length, act: { t: 'sheet', v: 'team' } });
  row('add-payments', 'codex', sessionState(s, ts), '2m', true, extra, 'gpt-5-codex high', s.stage === 'solo' ? 'footer year updated · 1 file' : s.stage === 'delivered' ? 'payments verified · ready to merge' : 'lead · plan v2 · Stripe checkout');
  row('fix-footer', 'claude', 'idle', '1h', false, null, 'sonnet', 'waiting for your next prompt');
  proj('pixls', 'no git'); row('warm-finch', 'codex', 'working', '35s', false, [[{ t: '└ ', fg: T.dim }, { t: stGlyph(G, 'working', s.f + 3), fg: T.dim }, { t: ' explorer', fg: T.text }, { t: ' · native helper', fg: T.dim }]], 'gpt-5-codex medium', 'tracing the token pipeline');
  proj('seshi', G.branch + ' dev'); row('quick-fox', 'claude', 'done', '9h', false, null, 'opus', 'merged #142 · not seen');
  const by = y + h - 2; g.putSegs(x + 2, by, [{ t: '+', fg: T.acc, b: 1 }, { t: ' new', fg: T.text }]); const ac = 'a actions'; g.putSegs(x + w - 2 - L(ac), by, [{ t: 'a', fg: T.acc, b: 1 }, { t: ' actions', fg: T.text }]);
  if (w >= 32) { const tmx = x + 10; g.putSegs(tmx, by, [{ t: 'T', fg: T.acc, b: 1 }, { t: ' team', fg: T.text }]); R.push({ x: tmx, y: by, w: 6, h: 1, act: { t: 'sheet', v: s.sheet === 'team' ? null : 'team' } }); }
  R.push({ x: x + 2, y: by, w: 5, h: 1, act: { t: 'view', v: 'newsession' } });
}
function sessionState(s, ts) { const c = teamCounts(ts); if (c.needs || c.interrupted || c.failed) return 'needs you'; if (s.stage === 'plan' || s.stage === 'missing') return 'needs you'; if (s.stage === 'delivered') return 'done'; if (s.stage === 'solo') return 'idle'; return 'working'; }

// ---------- team sheet
function drawTeam(g, R, T, G, s, ts, x, y, w, h) {
  const lit = s.focus === 'sheet', iw = w - 4, X = x + 2;
  card(g, x, y, w, h, T, lit, [{ t: 'TEAM', fg: lit ? T.acc : T.strong, b: 1 }, { t: ' · add-payments', fg: T.text }], [{ t: G.close, fg: T.dim }]);
  R.push({ x: x + w - 4, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 1;
  const kv = (k, v) => { g.putSegs(X, yy, [{ t: pad(k, 10), fg: T.dim }, ...v], {}, X + iw); yy++; };
  const empty = !Object.keys(ts).length;
  if (empty) {
    yy++; g.putSegs(X, yy++, [{ t: 'No workers yet', fg: T.strong, b: 1 }]);
    ['This session is doing the work itself. That\'s the default for small', 'changes: nothing is dispatched unless a plan you approve has tasks', 'for specialists this session is allowed to use.'].forEach(l => g.put(X, yy++, trunc(l, iw), { fg: T.text }));
    yy++; ['forge', 'canvas', 'lens'].forEach((k, i) => { head(g, X + i * 10, yy, k, T, 'queued', s.f); g.put(X + i * 10, yy + 3, SPEC[k].n, { fg: T.dim }); }); yy += 5;
    kv('Allowed', [{ t: 'Forge, Canvas, Lens · resting until a plan needs them', fg: T.text }]);
    kv('Approval', [{ t: 'start tasks after I approve the plan', fg: T.text }]);
    kv('Workers', [{ t: 'up to 2 at once + this session + a reviewer', fg: T.text }]);
    yy++; let bx = X; bx = button(g, R, bx, yy, T, 'Change allowed', 'e', 'normal', { t: 'view', v: 'newsession' }); button(g, R, bx, yy, T, 'Specialists', 'P', 'normal', { t: 'view', v: 'library' });
    g.putSegs(X, y + h - 2, hintSegs(T, [['e', 'allowed'], ['P', 'library'], ['Esc', 'close']]), {}, X + iw);
    return;
  }
  const c = teamCounts(ts), busy = Object.keys(ts).filter(id => (ts[id].k === 'working' || ts[id].live || ts[id].k === 'needs') && id !== 'R').map(id => TASK[id].sp);
  kv('Goal', [{ t: 'Stripe checkout + 10% affiliate commission', fg: T.strong }]);
  kv('Plan', s.stage === 'plan' || s.stage === 'missing' ? [{ t: 'v2 proposed 13:58 · waiting for you', fg: T.needs }] : [{ t: 'v2 approved 14:02 · lead is this session', fg: T.text }]);
  kv('Workers', [{ t: new Set(busy).size + '/2 busy', fg: T.text }, { t: '  dispatch ', fg: T.dim }, s.held ? { t: 'held', fg: T.needs, b: 1 } : { t: 'on', fg: T.text }]);
  const integ = s.stage === 'delivered' ? [{ t: G.verified + ' verified', fg: T.ok, b: 1 }, { t: ' on seshi/add-payments@9f3c2a1', fg: T.dim }] : s.stage === 'review' ? [{ t: G.changes + ' changes requested', fg: T.err, b: 1 }, { t: ' · review #1', fg: T.dim }] : [{ t: 'not reviewed yet', fg: T.dim }, { t: ' · tasks done ≠ feature works', fg: T.dim }];
  kv('Combined', integ);
  g.hl(x + 1, yy, w - 2, '─', { fg: T.line }); yy++;

  if (s.stage === 'plan' || s.stage === 'missing') {
    let bx = X; bx = button(g, R, bx, yy, T, 'Approve plan v2', 'a', s.stage === 'missing' ? 'normal' : 'primary', { t: 'approve' }); bx = button(g, R, bx, yy, T, 'Ask for changes', 'm', 'normal', { t: 'toast', v: 'Follow-up to the lead opens beside the row (existing m popover)' });
    if (s.stage === 'missing' && iw > 60) g.put(bx + 1, yy, 'T3, T4 unassigned', { fg: T.needs });
    yy += 2;
  }
  if (s.stage === 'delivered') { yy = drawResult(g, R, T, G, s, X, yy, iw) + 1; }
  const rows = []; GROUP.forEach(([k, label]) => { const ids = Object.keys(ts).filter(id => ts[id].k === k); if (ids.length) rows.push({ label, k, ids }); });
  let idx = 0; const maxY = y + h - 3;
  for (const grp of rows) {
    if (yy >= maxY) break;
    const urgent = ['needs', 'interrupted', 'failed', 'changes'].includes(grp.k), gc = urgent ? stColor(T, grp.k) : grp.k === 'done' ? T.ok : T.btn;
    g.putSegs(X, yy, [{ t: ' ' + grp.label + ' ', fg: urgent || grp.k === 'done' ? T.bg : T.strong, bg: gc, b: 1 }, { t: ' ' + grp.ids.length + ' ', fg: urgent || grp.k === 'done' ? T.bg : T.text, bg: mix(gc, T.bg, 0.25), b: 1 }]); yy++;
    for (const id of grp.ids) {
      if (yy >= maxY) break;
      const tall = h >= 40, rh = tall ? 3 : 2, tx0 = X + (tall ? 9 : 7);
      const t = TASK[id], v = ts[id], on = s.sel === idx && lit, sp = SPEC[t.sp], bgc = on ? mix(T.bg, T.text, 0.12) : undefined;
      if (on) { g.fill(x + 1, yy, w - 2, rh, bgc); for (let j = 0; j < rh; j++) g.set(x + 1, yy + j, G.bar, { fg: T.acc, bg: bgc }); }
      const warn = v.warn;
      if (warn) { g.fill(X, yy, tall ? 7 : 5, rh, mix(T.bg, T.needs, 0.15)); g.put(X + (tall ? 3 : 2), yy + (tall ? 1 : 0), '?', { fg: T.needs, bg: mix(T.bg, T.needs, 0.15), b: 1 }); } else head(g, X, yy, t.sp, T, v.k, s.f, bgc, !tall);
      g.putSegs(tx0, yy, [{ t: stGlyph(G, v.k, s.f) + ' ', fg: stColor(T, v.k), bg: bgc }, { t: pad(id, 3), fg: T.dim, bg: bgc }, { t: trunc(t.title, iw - 29), fg: T.strong, b: v.k !== 'done', bg: bgc }], {}, X + iw);
      const who = warn ? [{ t: G.warn + ' none', fg: T.needs, bg: bgc }] : [{ t: sp.n, fg: sp.c, b: 1, bg: bgc }];
      g.putSegs(X + iw - 8, yy, who, {}, X + iw);
      const note = v.note + (v.ago ? ' · ' + v.ago : '');
      const wk = ['planned', 'waiting', 'queued'].includes(v.k) ? 'no worker yet' : 'w' + (t.sp === 'forge' ? 1 : t.sp === 'canvas' ? 2 : 3) + ' · ' + sp.model.split(' · ').slice(1).join(' ');
      if (tall) g.putSegs(X + 11, yy + 2, [{ t: trunc((t.deps.length ? 'after ' + t.deps.join(', ') + ' · ' : '') + t.ac + ' · ' + wk, iw - 11), fg: T.dim, bg: bgc }], {}, X + iw);
      g.putSegs(tx0 + 2, yy + 1, [{ t: trunc(note, iw - (tx0 - X) - 2), fg: ['needs', 'failed', 'interrupted', 'changes'].includes(v.k) || warn ? stColor(T, warn ? 'needs' : v.k) : T.dim, bg: bgc }], {}, X + iw);
      R.push({ x: x + 1, y: yy, w: w - 2, h: rh, act: { t: 'open', id, i: idx } });
      yy += rh;
      if (v.k === 'needs' && yy < maxY) { let bx = tx0 + 2; bx = button(g, R, bx, yy, T, 'Allow once', '1', 'warn', { t: 'answer' }); bx = button(g, R, bx, yy, T, 'For this session', '2', 'normal', { t: 'answer' }); if (bx + 8 < X + iw) button(g, R, bx, yy, T, 'Deny', '3', 'normal', { t: 'answer' }); yy += 2; }
      if (v.k === 'interrupted' && yy < maxY) { let bx = tx0 + 2; bx = button(g, R, bx, yy, T, 'Retry…', 'r', 'normal', { t: 'confirm', v: 'retry' }); bx = button(g, R, bx, yy, T, 'Open', '⏎', 'normal', { t: 'open', id, i: idx }); yy += 2; }
      if (warn && id === 'T3' && yy < maxY) { let bx = tx0 + 2; bx = button(g, R, bx, yy, T, 'Pick specialist', 'p', 'normal', { t: 'fixMissing' }); bx = button(g, R, bx, yy, T, 'Edit Canvas', 'e', 'normal', { t: 'view', v: 'library', libSel: 'canvas' }); if (bx + 14 < X + iw) button(g, R, bx, yy, T, 'Keep in session', 'k', 'normal', { t: 'fixMissing' }); yy += 2; }
      idx++;
    }
  }
  if (s.stage === 'review' && yy < maxY - 4) yy = drawFindings(g, T, G, s, X, yy + 1, iw);
  const hints = s.stage === 'plan' || s.stage === 'missing' ? [['a', 'approve'], ['m', 'changes'], ['⏎', 'open'], ['Esc', 'close']] : (c.working || c.needs || c.changes) ? [['⏎', 'open'], ['m', 'message'], ['x', 'stop worker'], ['h', s.held ? 'resume dispatch' : 'hold dispatch'], ['Esc', 'close']] : [['⏎', 'open'], ['d', 'changes'], ['Esc', 'close']];
  g.putSegs(X, y + h - 2, hintSegs(T, iw < 60 ? hints.filter(([k]) => k !== 'm') : hints), {}, X + iw);
}
function drawFindings(g, T, G, s, X, yy, iw) {
  g.putSegs(X, yy++, [{ t: 'REVIEW #1 · LENS · COMBINED BRANCH', fg: T.err, b: 1 }]);
  g.putSegs(X, yy++, [{ t: G.done + ' build  ' + G.done + ' tsc  ' + G.done + ' unit 52/52  ', fg: T.ok }, { t: G.failed + ' e2e 2/3', fg: T.err }], {}, X + iw);
  [['F1', 'high', 'webhook accepts unsigned events if the secret is missing', 'T2 · Forge repaired · verified', T.ok], ['F2', 'med', 'order page trusts ?paid=1 before the webhook confirms', 'T4 · Canvas repairing', T.work]].forEach(([id, sev, txt, st, c]) => {
    g.putSegs(X, yy++, [{ t: id + ' ', fg: T.dim }, { t: pad(sev, 5), fg: sev === 'high' ? T.err : T.needs, b: 1 }, { t: trunc(txt, iw - 9), fg: T.text }], {}, X + iw);
    g.putSegs(X + 9, yy++, [{ t: trunc(st, iw - 9), fg: c }], {}, X + iw);
  });
  return yy;
}
function drawResult(g, R, T, G, s, X, yy, iw) {
  g.putSegs(X, yy++, [{ t: 'RESULT', fg: T.ok, b: 1 }, { t: ' · Lens review #2 · 15:32', fg: T.dim }]);
  [['AC1', 'checkout.spec › buy flow reaches Checkout'], ['AC2', 'webhook.test › 3 deliveries → 1 order'], ['AC3', 'commission.test › 10% of 4900 → 490'], ['AC4', 'checkout.spec › Pending until webhook'], ['AC5', 'e2e 3/3 on the combined branch']].forEach(([a, e]) => g.putSegs(X, yy++, [{ t: G.done + ' ', fg: T.ok }, { t: a + ' ', fg: T.dim }, { t: trunc(e, iw - 6), fg: T.text }], {}, X + iw));
  g.putSegs(X, yy++, [{ t: 'Checks   ', fg: T.dim }, { t: 'build ' + G.done + '  tsc ' + G.done + '  unit 58/58 ' + G.done + '  e2e 3/3 ' + G.done, fg: T.ok }], {}, X + iw);
  g.putSegs(X, yy++, [{ t: 'Changes  ', fg: T.dim }, { t: '4 tasks · 13 files · ', fg: T.text }, { t: '+598 ', fg: T.a.green }, { t: '−41', fg: T.a.red }], {}, X + iw);
  g.putSegs(X, yy++, [{ t: 'Left     ', fg: T.dim }, { t: trunc('refunds (out of scope) · F3 index deferred by you', iw - 9), fg: T.text }], {}, X + iw);
  g.putSegs(X, yy++, [{ t: 'Not merged or published. Review doesn\'t authorize either.', fg: T.needs }], {}, X + iw);
  let bx = X; bx = button(g, R, bx, yy, T, 'Changes', 'd', 'normal', { t: 'toast', v: 'Opens the existing Changes view on seshi/add-payments' }); bx = button(g, R, bx, yy, T, 'Merge…', 'M', 'normal', { t: 'confirm', v: 'merge' }); if (bx + 12 < X + iw) button(g, R, bx, yy, T, 'Open PR…', 'P', 'normal', { t: 'confirm', v: 'pr' });
  return yy + 2;
}

// ---------- task detail
function drawTask(g, R, T, G, s, ts, x, y, w, h) {
  const id = s.task, t = TASK[id], v = ts[id] || { k: 'planned', note: '' }, sp = SPEC[t.sp], lit = s.focus === 'sheet', iw = w - 4, X = x + 2;
  card(g, x, y, w, h, T, lit, [{ t: G.back + ' TEAM', fg: T.dim }, { t: ' / ', fg: T.dim }, { t: id + ' ' + t.title, fg: lit ? T.acc : T.strong, b: 1 }], [{ t: G.close, fg: T.dim }]);
  R.push({ x: x + 1, y, w: 7, h: 1, act: { t: 'back' } }); R.push({ x: x + w - 4, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 1; const kv = (k, segs) => { g.putSegs(X, yy, [{ t: pad(k, 12), fg: T.dim }, ...segs], {}, X + iw); yy++; };
  head(g, X, yy, t.sp, T, v.k, s.f);
  g.putSegs(X + 9, yy++, [{ t: stGlyph(G, v.k, s.f) + ' ' + KL[v.k], fg: stColor(T, v.k), b: 1 }, { t: '  ' + v.note, fg: T.text }], {}, X + iw);
  const help = { working: 'A worker is running. Message it, open its pane, or stop just this worker.', waiting: 'Starts by itself when its dependency completes.', needs: 'Paused on you. Answer here, in the Inbox, or in its pane.', interrupted: 'Seshi lost the turn in progress. Inspect, then retry on purpose.', failed: 'The attempt ended and its checks failed. Retry, or hand back to the lead.', done: 'Completed: the worker\'s turn ended, checks passed and review passed.', changes: 'Review found a problem; a repair attempt is fixing it.', stopped: 'You stopped it. The worktree is kept. Retry starts a new attempt.' }[v.k];
  if (help) g.put(X + 9, yy, trunc(help, iw - 9), { fg: T.dim }); yy += 2;
  yy++;
  kv('Specialist', [specCell(t.sp), { t: ' ' + sp.n, fg: T.strong, b: 1 }, { t: ' · ' + sp.role + ' · auto-selected (allowed here)', fg: T.dim }]);
  kv('Worker', v.k === 'planned' || v.k === 'waiting' || v.k === 'queued' ? [{ t: 'none yet · starts when ready', fg: T.dim }] : [{ t: 'w' + (id === 'T1' || id === 'T2' ? '1' : '2') + ' · ' + sp.model, fg: T.text }, { t: ' · captured 14:02', fg: T.dim }]);
  kv('Depends', t.deps.length ? t.deps.map(d => ({ t: d + ' ' + stGlyph(G, (ts[d] || {}).k || 'planned', s.f) + '  ', fg: stColor(T, (ts[d] || {}).k) })) : [{ t: 'nothing', fg: T.dim }]);
  kv('Checks', [{ t: t.ac, fg: T.text }, { t: '  ', fg: T.dim }, ...(v.k === 'done' ? [{ t: 'tsc ' + G.done + '  unit ' + G.done + '  review ' + G.done, fg: T.ok }] : v.k === 'failed' ? [{ t: 'tsc ' + G.failed + ' 2 errors', fg: T.err }] : v.k === 'interrupted' ? [{ t: 'unit test run never finished', fg: T.err }] : [{ t: 'tsc ' + G.done + '  unit running', fg: T.text }])]);
  kv('Worktree', [{ t: '~\\affiliate-site-worktrees\\' + t.tree, fg: T.text }]);
  yy++;
  g.putSegs(X, yy++, [{ t: 'ACTIVITY', fg: T.dim, b: 1 }, { t: '  lead ' + G.down + ' task   worker ' + G.up + ' result/question', fg: T.dim }]);
  const acts = {
    T1: [['14:02', 'down', 'lead', 'Implement POST /api/checkout/session per contract v1 (AC1). Backend files only.'], ['14:05', 'up', 'Forge', 'prices are integer cents; creating the Checkout price inline'], ['14:11', 'tool', 'Forge', 'Ran npm test -- checkout   12 passed'], ['14:12', 'up', 'Forge', 'turn ended · waiting on checks before this counts as done']],
    T4: [['14:37', 'down', 'lead', 'Order page polls GET /api/orders/:id; Paid only when status is paid (AC4).'], ['14:58', 'sys', 'seshi', 'daemon restart · attempt 1 outcome unknown'], ['15:00', 'down', 'you', 'retry keeping attempt 1 changes'], ['15:04', 'tool', 'Canvas', 'Ran npx tsc --noEmit   2 errors'], ['15:04', 'sys', 'seshi', 'attempt 2 failed · checks failed']],
  };
  (acts[id] || acts.T1).forEach(([tm, dir, who, txt]) => { if (yy >= y + h - 8) return; const c = dir === 'down' ? T.wait : dir === 'up' ? T.ok : dir === 'sys' ? T.needs : T.dim, gl = dir === 'down' ? G.down : dir === 'up' ? G.up : '·'; g.putSegs(X, yy++, [{ t: tm + ' ', fg: T.dim }, { t: gl + ' ', fg: c, b: 1 }, { t: pad(who, 7), fg: T.strong }, { t: trunc(txt, iw - 16), fg: T.text }], {}, X + iw); });
  yy++;
  g.putSegs(X, yy++, [{ t: 'FILES', fg: T.dim, b: 1 }, { t: id === 'T4' ? '  3 files · +133 −0' : '  3 files · +171 −0', fg: T.dim }]);
  (id === 'T4' ? [['pages/order/[id].tsx', 71], ['pages/order/success.tsx', 33], ['lib/useOrderStatus.ts', 29]] : [['api/checkout/session.ts', 86], ['lib/stripe.ts', 24], ['tests/checkout.test.ts', 61]]).forEach(([f, a]) => { if (yy < y + h - 5) g.putSegs(X + 2, yy++, [{ t: pad(f, Math.min(30, iw - 10)), fg: T.text }, { t: '+' + a, fg: T.a.green }], {}, X + iw); });
  if (id === 'T1') g.putSegs(X + 2, yy++, [{ t: 'native helper: ', fg: T.dim }, { t: 'codex › test-runner (done)', fg: T.dim }, { t: '  shown here only', fg: T.dim }], {}, X + iw);
  const by = y + h - 3; let bx = X;
  if (v.k === 'working') { bx = button(g, R, bx, by, T, 'Message', 'm', 'normal', { t: 'toast', v: 'Follow-up popover to Forge (existing m)' }); bx = button(g, R, bx, by, T, 'Open pane', 'o', 'normal', { t: 'toast', v: 'Opens worker w1 as a split; it is an ordinary Seshi pane' }); button(g, R, bx, by, T, 'Stop worker…', 'x', 'danger', { t: 'confirm', v: 'stop' }); }
  else if (v.k === 'failed') { bx = button(g, R, bx, by, T, 'Retry…', 'r', 'normal', { t: 'confirm', v: 'retry' }); bx = button(g, R, bx, by, T, 'Hand to lead', 'l', 'normal', { t: 'toast', v: 'The lead gets the failure and proposes a change to the task' }); button(g, R, bx, by, T, 'Stop task', 'x', 'danger', { t: 'toast', v: 'Task stopped; worktree kept' }); }
  else button(g, R, bx, by, T, 'Back', 'Esc', 'normal', { t: 'back' });
  g.putSegs(X, y + h - 2, hintSegs(T, [['Esc', 'back'], ['i', 'why this state'], ['d', 'diff'], ['o', 'open pane']]), {}, X + iw);
}

// ---------- inbox (existing sheet, worker questions arrive with their path)
function drawInbox(g, R, T, G, s, ts, x, y, w, h) {
  const lit = s.focus === 'sheet', iw = w - 4, X = x + 2;
  card(g, x, y, w, h, T, lit, [{ t: 'INBOX', fg: lit ? T.acc : T.strong, b: 1 }, { t: ' · what needs you', fg: T.text }], [{ t: G.close, fg: T.dim }]);
  R.push({ x: x + w - 4, y, w: 3, h: 1, act: { t: 'sheet', v: null } });
  let yy = y + 1; g.putSegs(X, yy++, [{ t: '/ ', fg: T.dim }, { t: 'type to go anywhere', fg: T.dim }]); yy++;
  const bg = mix(T.bg, T.text, 0.12);
  g.fill(x + 1, yy, w - 2, 5, bg); for (let j = 0; j < 5; j++) g.set(x + 1, yy + j, G.bar, { fg: T.acc, bg });
  head(g, X + 1, yy, 'forge', T, 'needs', s.f, bg); g.putSegs(X + 10, yy++, [{ t: G.needs + ' ', fg: T.needs, bg }, { t: 'add-payments', fg: T.strong, b: 1, bg }, { t: ' › T2 › ', fg: T.dim, bg }, specCell('forge'), { t: ' Forge', fg: T.text, bg }, { t: '   3m', fg: T.dim, bg }], {}, X + iw);
  g.putSegs(X + 10, yy++, [{ t: trunc('Run npm install stripe@16? Network is "ask" for Forge.', iw - 10), fg: T.needs, bg }], {}, X + iw);
  g.putSegs(X + 10, yy++, [{ t: trunc('Came up through the lead; answering goes straight to the worker.', iw - 10), fg: T.dim, bg }], {}, X + iw);
  let bx = X + 10; bx = button(g, R, bx, yy, T, 'Allow once', '1', 'warn', { t: 'answer' }); bx = button(g, R, bx, yy, T, 'For this session', '2', 'normal', { t: 'answer' }); button(g, R, bx, yy, T, 'Deny', '3', 'normal', { t: 'answer' }); yy += 3;
  g.putSegs(X, yy++, [{ t: G.done + ' ', fg: T.ok }, { t: 'quick-fox', fg: T.text }, { t: '  finished · not seen', fg: T.dim }]);
  g.putSegs(X, y + h - 2, hintSegs(T, [['1–3', 'answer'], ['m', 'reply'], ['⏎', 'open'], ['Esc', 'close']]), {}, X + iw);
}

// ---------- modals
function modal(g, T, W, H, w, h, title) { const x = Math.floor((W - w) / 2), y = Math.max(1, Math.floor((H - h) / 2)); g.dimAll(); g.fill(x + 1, y + 1, w, h, '#000000'); g.fill(x, y, w, h, T.card); g.box(x, y, w, h, { fg: T.line }); g.fill(x, y, w, 1, T.acc); g.putSegs(x + 2, y, [{ t: title, fg: T.accInk, b: 1, bg: T.acc }]); g.putSegs(x + w - 11, y, [{ t: 'Esc close', fg: T.accInk, bg: T.acc }]); return { x, y }; }
function drawConfirm(g, R, T, G, s, W, H) {
  const C = {
    stop: ['STOP WORKER', ['Stop Forge on T1? This interrupts its running turn.', '', 'Closes   worker pane w1 (codex)', 'Keeps    worktree t1-checkout and its 3 changed files', '         the task, as stopped · the rest of the team', 'Tells    the lead, which won\'t reassign it by itself'], [['Stop worker', 'x', 'danger', { t: 'doStop' }], ['Hold dispatch instead', 'h', 'normal', { t: 'hold' }], ['Cancel', 'Esc', 'normal', { t: 'confirm', v: null }]]],
    retry: ['RETRY T4', ['Attempt 1 was running when the daemon restarted.', 'Seshi can\'t tell whether it finished.', '', 'Worktree t4-order-page has 3 uncommitted files.', 'A retry starts attempt 2. Nothing retries on its own.'], [['Keep changes', '1', 'primary', { t: 'retry', v: 'keep' }], ['From clean', '2', 'normal', { t: 'retry', v: 'clean' }], ['Stop task', '3', 'danger', { t: 'retry', v: 'stop' }]]],
    merge: ['MERGE INTO MAIN', ['Merge seshi/add-payments (9f3c2a1) into main?', '', 'Review passing does not authorize this. You do, here.', 'The task worktrees are removed after a clean merge;', 'branches stay until you clean up.'], [['Merge', 'y', 'primary', { t: 'toast', v: 'Merged (prototype). Uses the existing Changes merge.' }], ['Cancel', 'Esc', 'normal', { t: 'confirm', v: null }]]],
    pr: ['OPEN A PULL REQUEST', ['Push seshi/add-payments to origin and open a PR into main?', '', 'Body: acceptance checks, evidence and what\'s left.', 'No agent can push; this is your authorization.'], [['Push & open PR', 'y', 'primary', { t: 'toast', v: 'PR opened (prototype). Uses the existing Changes PR flow.' }], ['Cancel', 'Esc', 'normal', { t: 'confirm', v: null }]]],
  }[s.confirm];
  if (!C) return;
  const w = Math.min(W - 4, 66), h = C[1].length + 6, p = modal(g, T, W, H, w, h, C[0]);
  C[1].forEach((l, i) => g.put(p.x + 2, p.y + 2 + i, trunc(l, w - 4), { fg: i === 0 ? T.strong : T.text, bg: T.card, b: i === 0 }));
  let bx = p.x + 2; C[2].forEach(b => { bx = button(g, R, bx, p.y + h - 2, T, b[0], b[1], b[2], b[3]); });
}
function drawLibrary(g, R, T, G, s, W, H) {
  const w = W >= 140 ? Math.min(W - 8, 120) : W - 2, h = H - 3, p = modal(g, T, W, H, w, h, 'SPECIALISTS');
  const narrow = w < 90, lw = narrow ? w - 4 : 38, X = p.x + 2; let yy = p.y + 2;
  if (s.lib === 'empty') {
    g.putSegs(X, yy++, [{ t: 'No specialists yet', fg: T.strong, b: 1, bg: T.card }]);
    ['A specialist is a saved profile: a name, a description, instructions,', 'references, a provider and model, and limits on what it may do.', 'Sessions can call on the ones you allow. Without any, sessions', 'still work alone. Nothing is created for you.'].forEach(l => g.put(X, yy++, trunc(l, w - 4), { fg: T.text, bg: T.card }));
    yy++; let bx = X; bx = button(g, R, bx, yy, T, 'New specialist', 'n', 'primary', { t: 'libNew' }); button(g, R, bx, yy, T, 'Start from a template…', 't', 'normal', { t: 'toast', v: 'Templates: Backend, Frontend, Reviewer, Docs. Picking one opens it here to edit; nothing saves until you do.' });
    yy += 2; g.putSegs(X, yy++, [{ t: 'Project specialists', fg: T.dim, b: 1, bg: T.card }]); g.put(X, yy++, trunc('Profiles in .seshi/specialists/ in a repo appear here after you allow them.', w - 4), { fg: T.dim, bg: T.card });
    return;
  }
  const list = ['forge', 'canvas', 'lens', 'scribe'];
  if (!(narrow && s.libEdit)) {
    g.putSegs(X, yy++, [{ t: 'YOURS', fg: T.dim, b: 1, bg: T.card }, { t: ' · follow you to every project', fg: T.dim, bg: T.card }]);
    list.forEach(k => { const sp = SPEC[k], on = s.libSel === k, bg = on ? mix(T.card, T.text, 0.12) : T.card; g.fill(X - 1, yy, lw + 1, 1, bg); if (on) g.set(X - 1, yy, G.bar, { fg: T.acc, bg }); const unsup = sp.prov !== 'codex' && !(k === 'canvas'); g.putSegs(X, yy, [specCell(k), { t: ' ' + pad(sp.n, 8), fg: T.strong, b: on, bg }, { t: pad(sp.role, 10), fg: T.text, bg }, { t: trunc(sp.prov, 7), fg: unsup ? T.needs : T.dim, bg }, ...(unsup ? [{ t: ' ' + G.warn, fg: T.needs, bg }] : [])], {}, X + lw); R.push({ x: X - 1, y: yy, w: lw + 1, h: 1, act: { t: 'libSel', v: k } }); yy++; });
    yy++; g.putSegs(X, yy++, [{ t: 'THIS PROJECT', fg: T.dim, b: 1, bg: T.card }, { t: ' · .seshi/specialists', fg: T.dim, bg: T.card }]); g.put(X, yy++, 'none', { fg: T.dim, bg: T.card });
    if (narrow) { g.putSegs(X, p.y + h - 2, hintSegs(T, [['⏎', 'edit'], ['n', 'new'], ['Esc', 'close']]), { bg: T.card }); return; }
  }
  const sp = SPEC[s.libSel], ex = narrow ? X : X + lw + 3, ew = narrow ? w - 4 : w - lw - 7; let ey = p.y + 2;
  if (!narrow) g.vl(X + lw + 1, p.y + 1, h - 2, '│', { fg: T.line, bg: T.card });
  const maxEy = p.y + h - 3, compact = h < 26;
  const row = (k, segs) => { if (ey > maxEy) return; g.putSegs(ex, ey, [{ t: pad(k, 13), fg: T.dim, bg: T.card }, ...segs.map(x => ({ bg: T.card, ...x }))], {}, ex + ew); ey++; };
  const field = (k, v) => { if (ey > maxEy) return; g.putSegs(ex, ey, [{ t: pad(k, 13), fg: T.dim, bg: T.card }], {}); g.fill(ex + 13, ey, ew - 13, 1, T.card2); g.putSegs(ex + 14, ey, [{ t: trunc(v, ew - 15), fg: T.strong, bg: T.card2 }], {}, ex + ew); ey++; };
  head(g, ex, ey, s.libSel, T, 'idle', s.f, T.card); g.putSegs(ex + 9, ey, [{ t: sp.n, fg: T.strong, b: 1, bg: T.card }, { t: '  ' + sp.role, fg: sp.c, bg: T.card }]); g.putSegs(ex + 9, ey + 1, [{ t: s.libEdit ? 'editing' : 'saved profile · not a running agent', fg: T.dim, bg: T.card }]); ey += 4;
  field('Name', sp.n); field('Description', sp.desc);
  const il = compact ? 1 : 3; g.putSegs(ex, ey, [{ t: pad('Instructions', 13), fg: T.dim, bg: T.card }]); g.fill(ex + 13, ey, ew - 13, il, T.card2);
  ['You build server-side features in TypeScript. Follow the', 'session\'s shared interface exactly. Ask before adding', 'dependencies. Report what you changed and how you checked it.'].slice(0, il).forEach((l, i) => g.put(ex + 14, ey + i, trunc(l + (compact ? ' …' : ''), ew - 15), { fg: T.text, bg: T.card2 })); ey += il + (compact ? 0 : 1);
  field('References', sp.refs);
  const unsupported = sp.prov !== 'codex';
  row('Provider', [{ t: sp.prov, fg: T.strong }, unsupported ? { t: '  ' + G.warn + ' dispatch on ' + sp.prov + ' isn\'t supported yet; you can still run it by hand', fg: T.needs } : { t: '  ' + G.done + ' dispatch supported', fg: T.ok }]);
  row('Model', [{ t: sp.model.split(' · ')[1] || sp.model, fg: T.strong }]);
  row('Effort', [...['low', 'medium', 'high'].map(e => ({ t: ' ' + e + ' ', fg: sp.model.endsWith(e) ? T.accInk : T.text, bg: sp.model.endsWith(e) ? T.acc : T.btn, b: 1 }))]);
  if (compact) { row('Limits', Object.entries({ edit: 'edit', net: 'net', commit: 'commit', merge: 'merge', publish: 'PR' }).flatMap(([k, lab]) => { const v = sp.lim[k]; return [{ t: lab + ' ', fg: T.dim }, { t: v + '  ', fg: v === 'never' || v === 'none' ? T.dim : v === 'ask' ? T.needs : T.text }]; })); }
  else { ey++; g.putSegs(ex, ey++, [{ t: 'LIMITS', fg: T.dim, b: 1, bg: T.card }, { t: ' · upper bounds; a session can only narrow them', fg: T.dim, bg: T.card }]);
  Object.entries({ edit: 'Edit files', run: 'Run', net: 'Network', commit: 'Commit', merge: 'Merge', publish: 'Push / PR' }).forEach(([k, lab]) => { const v = sp.lim[k], c = v === 'never' || v === 'none' ? T.dim : v === 'ask' ? T.needs : T.text; row('  ' + lab, [{ t: v, fg: c }]); }); ey++; }
  row('Auto-select', [{ t: sp.auto ? 'on' : 'off', fg: T.strong }, { t: ' · sessions may pick it when you allow it; never widens limits', fg: T.dim }]);
  row('In use', s.libSel === 'forge' ? [{ t: 'add-payments T2 (w1) · keeps the settings it started with', fg: T.text }] : [{ t: 'not right now', fg: T.dim }]);
  const by = p.y + h - 2; let bx = ex; bx = button(g, R, bx, by, T, 'Save', 'Ctrl+S', 'primary', { t: 'toast', v: 'Saved. Applies to future assignments; running workers keep their captured settings.' }); bx = button(g, R, bx, by, T, 'Duplicate', 'D', 'normal', { t: 'toast', v: 'Duplicated as "Forge copy"' }); button(g, R, bx, by, T, 'Delete…', 'Del', 'danger', { t: 'toast', v: 'Asks first; profiles in use can\'t be deleted until their workers end' });
}
function drawNewSession(g, R, T, G, s, W, H) {
  const w = Math.min(W - 2, 84), h = Math.min(H - 2, 21), p = modal(g, T, W, H, w, h, 'NEW SESSION'), X = p.x + 2, iw = w - 4; let yy = p.y + 2;
  const fld = (k, v, on) => { g.putSegs(X, yy, [{ t: pad(k, 13), fg: on ? T.strong : T.dim, b: on, bg: T.card }]); g.fill(X + 13, yy, iw - 13, 1, on ? T.card2 : T.card); g.putSegs(X + 14, yy, v.map(x => ({ bg: on ? T.card2 : T.card, ...x })), {}, X + iw); yy++; };
  fld('Project', [{ t: 'affiliate-site', fg: T.strong }, { t: '  ' + G.branch + ' main', fg: T.dim }]);
  fld('Worktree', [{ t: 'new: seshi/add-payments', fg: T.strong }]);
  fld('Agent', [{ t: 'codex', fg: T.strong }, { t: '  gpt-5-codex · high', fg: T.dim }]);
  fld('Task', [{ t: 'Add payments to the affiliate site', fg: T.strong }], true); yy++;
  g.putSegs(X, yy++, [{ t: 'SPECIALISTS', fg: T.dim, b: 1, bg: T.card }, { t: ' · new · this session can call on them', fg: T.dim, bg: T.card }]);
  [['Work alone', 'no workers; good for small changes', false], ['Auto from allowed', 'it picks per task from the ones ticked below', true], ['Ask me per task', 'it proposes, you pick each one', false]].forEach(([a, b, on]) => { g.putSegs(X, yy++, [{ t: (on ? G.needs === '!' ? '(*)' : '(•)' : '( )') + ' ', fg: on ? T.acc : T.dim, bg: T.card }, { t: pad(a, 19), fg: on ? T.strong : T.text, b: on, bg: T.card }, { t: trunc(b, iw - 24), fg: T.dim, bg: T.card }]); });
  yy++; g.putSegs(X, yy++, [{ t: 'Allowed  ', fg: T.dim, bg: T.card }, { t: '[x] ', fg: T.acc, bg: T.card }, specCell('forge'), { t: ' Forge  ', bg: T.card }, { t: '[x] ', fg: T.acc, bg: T.card }, specCell('canvas'), { t: ' Canvas  ', bg: T.card }, { t: '[x] ', fg: T.acc, bg: T.card }, specCell('lens'), { t: ' Lens  ', bg: T.card }, { t: '[ ] ', fg: T.dim, bg: T.card }, specCell('scribe'), { t: ' Scribe ' + G.warn, fg: T.dim, bg: T.card }], {}, X + iw);
  g.putSegs(X, yy++, [{ t: 'Approval ', fg: T.dim, bg: T.card }, { t: 'start tasks after I approve the plan', fg: T.strong, bg: T.card }, { t: '  ‹ ›', fg: T.dim, bg: T.card }], {}, X + iw);
  g.putSegs(X, yy++, [{ t: 'Limit    ', fg: T.dim, bg: T.card }, { t: '2 workers at once + a reviewer · no nested delegation', fg: T.text, bg: T.card }], {}, X + iw);
  let bx = X; bx = button(g, R, bx, p.y + h - 2, T, 'Start', '⏎', 'primary', { t: 'go', p: '05 Plan proposed' }); button(g, R, bx, p.y + h - 2, T, 'Cancel', 'Esc', 'normal', { t: 'view', v: 'session' });
}

// ---------- main render
export function render(s) {
  const T = THEMES[s.theme] || THEMES.pixls, G = GL[s.glyphs] || GL.unicode, W = s.W, H = s.H, g = new Grid(W, H, T), R = [];
  const ts = tasksFor(s.stage, s);
  const SW = W >= 200 ? 42 : W >= 140 ? 36 : W >= 100 ? 26 : 7;
  const PH = H; drawSidebar(g, R, T, G, s, ts, 0, 0, SW, H);
  const px = SW - 1, pw = W - px;
  // tabs
  
  const sheetOn = s.sheet && s.view === 'session', SHW = W >= 200 ? 92 : W >= 140 ? 72 : pw;
  const paneW = sheetOn && W >= 140 ? pw - SHW + 1 : pw, paneLit = s.focus === 'pane';
  if (!(sheetOn && W < 140)) {
    const st = sessionState(s, ts), kk = st === 'needs you' ? 'needs' : st;
    card(g, px, 0, paneW, PH, T, paneLit, null, [{ t: stGlyph(G, kk, s.f) + ' ' + st, fg: stColor(T, kk) }, { t: '  ' + G.close + ' ', fg: T.dim }]);
    // tabs sit on the pane's top edge, level with the sidebar and sheet titles
    let tx = px + 1; tx = g.putSegs(tx, 0, [{ t: ' 1 ', fg: T.accInk, bg: T.acc, b: 1 }, { t: ' add-payments ', fg: paneLit ? T.accInk : T.strong, bg: paneLit ? T.acc : T.btn, b: 1 }]); tx = g.putSegs(tx, 0, [{ t: ' 2 ', fg: T.text, bg: T.btn }, { t: ' fix-footer ', fg: T.text, bg: T.card }]); g.putSegs(tx + 1, 0, [{ t: '+', fg: T.acc, b: 1, bg: T.card }]);
    drawTerminal(g, px + 3, 2, paneW - 6, PH - 3, T, s.stage, s);
    if (!paneLit) for (let yy = 1; yy < PH - 1; yy++) for (let xx = px + 1; xx < px + paneW - 1; xx++) { const c = g.c[yy][xx]; c.fg = mix(c.fg, c.bg, 0.3); }
    R.push({ x: px + 1, y: 1, w: paneW - 2, h: PH - 2, act: { t: 'focusPane' } });
  }
  let sx = null; if (sheetOn) { sx = W >= 140 ? px + paneW - 1 : px; const sw = W >= 140 ? SHW : pw; if (s.sheet === 'team') drawTeam(g, R, T, G, s, ts, sx, 0, sw, PH); else if (s.sheet === 'task') drawTask(g, R, T, G, s, ts, sx, 0, sw, PH); else if (s.sheet === 'inbox') drawInbox(g, R, T, G, s, ts, sx, 0, sw, PH); }

  if (s.leader) { const lb = ' Ctrl+Space ', lx = W - L(lb) - 1; g.putSegs(lx, 0, [{ t: lb, fg: T.bg, bg: T.sky, b: 1 }]); }
  if (s.view === 'library') drawLibrary(g, R, T, G, s, W, H);
  if (s.view === 'newsession') drawNewSession(g, R, T, G, s, W, H);
  if (s.confirm) drawConfirm(g, R, T, G, s, W, H);
  if (s.toast) { const t = trunc(s.toast, W - 6), x = Math.max(1, W - L(t) - 4); g.fill(x, H - 2, L(t) + 2, 1, T.btn); g.put(x + 1, H - 2, t, { fg: T.strong, bg: T.btn }); }
  return { rows: g.rows(), regions: R, W, H };
}

// ---------- actions (keyboard + mouse share one reducer)
const stageNext = { plan: 'work', missing: 'plan', needs: 'needs' };
export function act(s, a) {
  const S = (p, fx) => ({ state: { ...s, toast: null, ...p }, fx });
  const toast = v => ({ state: { ...s, toast: v }, fx: [{ after: 2600, act: { t: 'clearToast' } }] });
  const ts = tasksFor(s.stage, s), order = []; GROUP.forEach(([k]) => Object.keys(ts).filter(id => ts[id].k === k).forEach(id => order.push(id)));
  switch (a.t) {
    case 'tick': return S({ f: s.f + 1, toast: s.toast });
    case 'clearToast': return S({ toast: null });
    case 'toast': return toast(a.v);
    case 'go': return S(preset(a.p, s.W, s.H, { theme: s.theme, glyphs: s.glyphs }));
    case 'leader': return S({ leader: !s.leader });
    case 'sheet': return S({ sheet: a.v, view: 'session', focus: a.v ? 'sheet' : 'pane', confirm: null });
    case 'focusPane': return S({ focus: 'pane', view: 'session' });
    case 'view': return S({ view: a.v, focus: a.v === 'session' ? 'pane' : 'modal', libSel: a.libSel || s.libSel, lib: 'full' });
    case 'open': return S({ sheet: 'task', task: a.id, sel: a.i ?? s.sel, focus: 'sheet' });
    case 'back': return s.sheet === 'task' ? S({ sheet: 'team' }) : S({ sheet: null, focus: 'pane' });
    case 'approve': return s.stage === 'missing' ? toast('Assign T3 and T4 first, or keep them in this session') : S({ stage: 'work', sel: 0 });
    case 'fixMissing': return S({ stage: 'plan', toast: 'Canvas switched to Codex for this session. Plan v2 can be approved.' });
    case 'answer': return S({ answered: true, toast: 'Sent to Forge. T2 resumes; the lead is told.' });
    case 'confirm': return S({ confirm: a.v });
    case 'doStop': return S({ confirm: null, stopped: 'T1', toast: 'Forge stopped on T1. Worktree kept. Other workers continue.' });
    case 'hold': return S({ confirm: null, held: !s.held, toast: s.held ? 'Dispatch resumed: waiting tasks can start.' : 'Dispatch held: running workers continue; nothing new starts.' });
    case 'retry': return S({ confirm: null, t4: a.v === 'stop' ? 'stopped' : 'retry', toast: a.v === 'stop' ? 'T4 stopped. Review won\'t start until you resume or drop it.' : 'Attempt ' + (s.stage === 'failed' ? 3 : 2) + ' started ' + (a.v === 'clean' ? 'from a clean worktree.' : 'keeping earlier changes.') });
    case 'libSel': return S({ libSel: a.v, libEdit: 1 });
    case 'libNew': return S({ lib: 'full', libSel: 'forge', libEdit: 1, toast: 'New profile (sample filled in as Forge)' });
    case 'key': {
      const k = a.key;
      if (s.leader) { const p = { leader: false }; if (k === 'T' || k === 't') return S({ ...p, sheet: s.sheet === 'team' ? null : 'team', view: 'session', focus: 'sheet' }); if (k === 'P' || k === 'p') return S({ ...p, view: 'library', focus: 'modal' }); if (k === 'j') return S({ ...p, sheet: 'inbox', view: 'session', focus: 'sheet' }); if (k === 'n') return S({ ...p, view: 'newsession', focus: 'modal' }); return S(p); }
      if (s.confirm) { if (k === 'Escape') return S({ confirm: null }); if (s.confirm === 'stop') { if (k === 'x') return act(s, { t: 'doStop' }); if (k === 'h') return act(s, { t: 'hold' }); } if (s.confirm === 'retry') { if (k === '1') return act(s, { t: 'retry', v: 'keep' }); if (k === '2') return act(s, { t: 'retry', v: 'clean' }); if (k === '3') return act(s, { t: 'retry', v: 'stop' }); } if ((s.confirm === 'merge' || s.confirm === 'pr') && k === 'y') return act({ ...s, confirm: null }, { t: 'toast', v: s.confirm === 'merge' ? 'Merged (prototype)' : 'PR opened (prototype)' }); return null; }
      if (s.view !== 'session') { if (k === 'Escape') return S({ view: 'session', focus: s.sheet ? 'sheet' : 'pane' }); if (s.view === 'library' && (k === 'ArrowDown' || k === 'ArrowUp')) { const L2 = ['forge', 'canvas', 'lens', 'scribe'], i = L2.indexOf(s.libSel); return S({ libSel: L2[(i + (k === 'ArrowDown' ? 1 : 3)) % 4] }); } if (s.view === 'newsession' && k === 'Enter') return act(s, { t: 'go', p: '05 Plan proposed' }); return null; }
      if (s.focus === 'sheet') {
        if (k === 'Escape') return act(s, { t: 'back' });
        if (s.sheet === 'team') { if (k === 'ArrowDown' || k === 'j') return S({ sel: Math.min(order.length - 1, s.sel + 1) }); if (k === 'ArrowUp' || k === 'k') return S({ sel: Math.max(0, s.sel - 1) }); if (k === 'Enter' && order[s.sel]) return act(s, { t: 'open', id: order[s.sel], i: s.sel }); if (k === 'a') return act(s, { t: 'approve' }); if (k === 'h') return act(s, { t: 'hold' }); if (k === 'x') return S({ confirm: 'stop' }); if (k === 'r' && s.stage === 'restart') return S({ confirm: 'retry' }); if (['1', '2', '3'].includes(k) && s.stage === 'needs') return act(s, { t: 'answer' }); if (k === 'M' && s.stage === 'delivered') return S({ confirm: 'merge' }); if (k === 'P' && s.stage === 'delivered') return S({ confirm: 'pr' }); if (k === 'p' && s.stage === 'missing') return act(s, { t: 'fixMissing' }); }
        if (s.sheet === 'task') { if (k === 'x') return S({ confirm: 'stop' }); if (k === 'r') return S({ confirm: 'retry' }); if (k === 'm' || k === 'o' || k === 'i' || k === 'd') return toast({ m: 'Follow-up popover (existing m)', o: 'Opens the worker pane as a split', i: 'Status evidence popover (existing i)', d: 'Changes for this task\'s worktree' }[k]); }
        if (s.sheet === 'inbox' && ['1', '2', '3'].includes(k)) return act(s, { t: 'answer' });
        return null;
      }
      return null;
    }
  }
  return null;
}
