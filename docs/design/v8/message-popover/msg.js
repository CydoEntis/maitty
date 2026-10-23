// Seshi · message-agent popover redesign. Built on the v7 frame renderer (checks7.js). SAMPLE DATA ONLY.
import * as C7 from './checks7.js';
import { trunc, L } from './seshi-grid.js';
export const THEMES = C7.THEMES;

const bl = [0, 0x40, 0x44, 0x46, 0x47], br = [0, 0x80, 0xA0, 0xB0, 0xB8];
const BARS = [[1, 3, 2], [2, 4, 1], [3, 3, 3], [4, 2, 4], [3, 1, 3], [2, 2, 2], [1, 4, 3], [2, 3, 4], [4, 1, 2], [3, 2, 1]].map(([a, b, c]) => String.fromCharCode(0x2800 + bl[a] + br[b]) + String.fromCharCode(0x2800 + bl[c]));
const GX = {
  unicode: { ptr: '◂', more: '↑', q: '◷', sent: '›', needs: '●', done: '○', bar: '▏', work: BARS, prompt: '›' },
  ascii: { ptr: '<', more: '^', q: '~', sent: '>', needs: '!', done: 'o', bar: '|', work: ['| ', '/ ', '- ', '\\ '], prompt: '>' },
};
GX.nerd = GX.unicode;
const QUOTES = {
  'warm-finch': ['codex', 'asked', ['Both pnpm-lock.yaml and package-lock.json are in the repo. pnpm-lock is newer and CI runs pnpm install, but the README still says npm install, and two scripts call npx directly.', 'Use pnpm or npm for the workspace?']],
  'add-payments': ['claude', 'said', ['Wiring the Stripe webhook handler now: signature check, idempotency key on the event id, then the commission row. Tests come after that.']],
  'zsh-cleanup': ['codex', 'asked', ['Found 3 aliases that shadow git commands (gc, gp, gl).', 'Remove them?']],
};
const DEF_Q = ['claude', 'said', ['Done. Footer updated and its tests pass.']];
const FILL = ['dotfiles-sync', 'nvim-lsp', 'tmux-theme', 'brew-bundle', 'git-hooks', 'ssh-config', 'fonts', 'kitty-keys', 'starship', 'fzf-preview', 'direnv', 'mise-pins'];
const fillers = (W, H) => Math.min(FILL.length, W >= 140 ? Math.max(0, Math.floor((H - 25) / 2)) : Math.max(0, H - 21));
const ONE = ['pnpm. CI already uses it and its lockfile is newer.'];
const SEVERAL = ['pnpm. CI already uses it and its lockfile is newer.', 'Delete package-lock.json,', 'and change the README to say pnpm install.'];
const LONG = [...SEVERAL, 'Replace the two npx calls in scripts/ with pnpm dlx.', 'Add packageManager to package.json so corepack', 'picks the right version, then run pnpm install', 'once to refresh the lockfile.', 'Leave the CI config alone; it is already right.', 'Tell me when the tests pass.'];

export const PRESETS = [
  ['01 Empty · agent blocked', { sel: 2, mopen: 1, text: [''] }],
  ['02 Typing · one line', { sel: 2, mopen: 1, text: ONE }],
  ['03 Typing · several lines', { sel: 2, mopen: 1, text: SEVERAL }],
  ['04 Long reply · ↑ n more', { sel: 2, mopen: 1, text: LONG }],
  ['05 Agent working · will queue', { sel: 1, mopen: 1, text: ['Also add a test for a duplicate webhook.'] }],
  ['06 After send · queued', { sel: 1, after: 'queued', sentFor: 'add-payments', sentText: 'Also add a test for a duplicate webhook.' }],
  ['07 After send · sent', { sel: 1, after: 'sent', sentFor: 'add-payments', sentText: 'Also add a test for a duplicate webhook.' }],
  ['08 Row at the bottom · opens upward', { many: 1, mopen: 1, text: ['Yes, remove all three.'] }],
];
export function preset(name, W, H, fl = {}) {
  const p = (PRESETS.find(x => x[0] === name) || PRESETS[0])[1];
  const s = { ...C7.preset('01 No check set', W, H, fl), focus: 'side', mopen: 0, text: [''], after: null, sentFor: null, sentText: '', many: 0, ...p };
  if (p.many) s.sel = 5 + fillers(W, H);
  return s;
}
function rowsOf(s) {
  const r = [['fix-footer', 'done', 'codex'], ['add-payments', 'working', 'claude'], ['warm-finch', 'needs', 'codex'], ['quick-fox', 'done', 'claude'], ['old-moth', 'done', 'codex']];
  if (s.many) { FILL.slice(0, fillers(s.W, s.H)).forEach(id => r.push([id, 'done', 'codex'])); r.push(['zsh-cleanup', 'needs', 'codex']); }
  return r.map(([id, k, agent]) => ({ id, agent, k: s.after === 'sent' && s.sentFor === id ? 'working' : k }));
}
function wrap(text, w) {
  const out = [];
  for (const para of text) { if (!para.length) { out.push(''); continue; } let line = ''; for (const word of para.split(' ')) { if (!line) line = word; else if (L(line) + 1 + L(word) <= w) line += ' ' + word; else { out.push(line); line = word; } while (L(line) > w) { out.push(Array.from(line).slice(0, w).join('')); line = Array.from(line).slice(w).join(''); } } out.push(line); }
  return out;
}

function patchF(F, T, G, s) {
  const GG = GX[s.glyphs] || GX.unicode;
  if (s.many) F.push({ name: 'dotfiles', meta: 'no git', list: [...FILL.slice(0, fillers(s.W, s.H)).map(id => ({ id, agent: 'codex', age: '1d', k: 'done', checks: [], sub: [{ t: 'finished', fg: T.dim }] })), { id: 'zsh-cleanup', agent: 'codex', age: '1m', k: 'needs', keep: 1, checks: [], sub: [{ t: 'asks: remove 3 aliases?', fg: T.needs }] }] });
  if (s.after) { const q = F.flatMap(f => f.list).find(q => q.id === s.sentFor); if (q) { const m = '"' + s.sentText + '"'; q.keep = 1; q.sub = s.after === 'queued' ? [{ t: GG.q + ' queued ', fg: T.dim, b: 1 }, { t: m, fg: T.text }] : [{ t: GG.sent + ' sent ', fg: T.acc, b: 1 }, { t: m, fg: T.text }]; if (s.after === 'sent') q.k = 'working'; } }
  return F;
}

// The popover. Colours: card (ground), bg (input well), acc (border, title, prompt, cursor, keys), dim (labels, gutter, counts), needs (blocked chip), text/strong (words).
function overlay(g, R, T, G7, s, info) {
  if (!s.mopen) return;
  const GG = GX[s.glyphs] || GX.unicode, W = s.W, H = s.H, ascii = s.glyphs === 'ascii';
  const anc = R.find(r => r.act && r.act.t === 'selSess' && r.act.i === s.sel); if (!anc) return;
  const q = rowsOf(s)[s.sel], mode = q.k === 'working' ? 'working' : q.k === 'needs' ? 'blocked' : 'idle';
  const keep = []; for (let y = anc.y; y < anc.y + anc.h; y++) for (let x = anc.x; x < anc.x + anc.w; x++) keep.push([x, y, { ...g.c[y][x] }]);
  g.dimAll(); keep.forEach(([x, y, c]) => { g.c[y][x] = c; });
  const x0 = info.px + 1, w = Math.min(68, W - x0 - 1), iw = w - 4, X = x0 + 2, narrow = w < 60;
  const [agent, verb, said] = QUOTES[q.id] || DEF_Q, wl = wrap(said, iw - 2), Q = H >= 45 ? 4 : H >= 30 ? 3 : 2, shown = wl.slice(-Q), cut = wl.length > Q;
  const il = wrap(s.text.length ? s.text : [''], iw - 2), maxIn = H >= 45 ? 6 : H >= 30 ? 4 : 3, more = il.length > maxIn ? il.length - (maxIn - 1) : 0, vis = more ? il.slice(-(maxIn - 1)) : il, inRows = more ? maxIn : il.length;
  const h = 2 + 1 + shown.length + 1 + inRows + (mode === 'working' ? 1 : 0) + 1;
  let top = anc.y - 1, up = false;
  if (top + h > H) { top = anc.y + anc.h - h + 1; up = true; if (top + h > H) top = H - h; }
  top = Math.max(0, top);
  const C = T.card, S = { fg: T.acc, bg: C }, K = s.corners === 'rounded' ? '╭╮╰╯' : '┌┐└┘';
  g.fill(x0, top, w, h, C);
  g.set(x0, top, K[0], S); g.set(x0 + w - 1, top, K[1], S); g.set(x0, top + h - 1, K[2], S); g.set(x0 + w - 1, top + h - 1, K[3], S);
  g.hl(x0 + 1, top, w - 2, '─', S); g.hl(x0 + 1, top + h - 1, w - 2, '─', S); g.vl(x0, top + 1, h - 2, '│', S); g.vl(x0 + w - 1, top + 1, h - 2, '│', S);
  const py = anc.y; if (py > top && py < top + h - 1) g.set(x0, py, GG.ptr, { fg: T.acc, bg: C, b: 1 });
  g.putSegs(x0 + 2, top, [{ t: ' message ' + q.id + ' ', fg: T.accInk, bg: T.acc, b: 1 }, { t: ' ' + agent + ' ', fg: T.dim, bg: C }]);
  const st = mode === 'blocked' ? [{ t: ' ' + GG.needs + ' needs you ', fg: T.needs, bg: C, b: 1 }] : mode === 'working' ? [{ t: ' ' + GG.work[s.motion ? s.f % GG.work.length : 2] + 'working ', fg: T.acc, bg: C, b: 1 }] : [{ t: ' ' + GG.done + ' done ', fg: T.dim, bg: C }];
  g.putSegs(x0 + w - 2 - st.reduce((n, x) => n + L(x.t), 0), top, st);
  R.push({ x: x0, y: top, w, h, act: { t: 'noop' } });
  let y = top + 1;
  g.putSegs(X, y++, [{ t: agent + ' ' + verb, fg: T.dim, bg: C, b: 1 }, { t: cut ? ' · last ' + Q + ' of ' + wl.length + ' lines' : '', fg: T.dim, bg: C }]);
  shown.forEach(l => g.putSegs(X, y++, [{ t: GG.bar + ' ', fg: T.dim, bg: C }, { t: l, fg: T.text, bg: C }], {}, X + iw));
  g.hl(x0 + 1, y++, w - 2, '─', { fg: T.line, bg: C });
  g.fill(X - 1, y, iw + 2, inRows, T.bg);
  if (more) g.putSegs(X, y++, [{ t: GG.more + ' ' + more + ' more line' + (more > 1 ? 's' : ''), fg: T.dim, bg: T.bg }]);
  const empty = il.length === 1 && !il[0];
  vis.forEach((l, i) => {
    const first = !more && i === 0, last = i === vis.length - 1;
    const segs = [{ t: first ? GG.prompt + ' ' : '  ', fg: T.acc, bg: T.bg, b: 1 }];
    if (empty) segs.push({ t: '█', fg: T.acc, bg: T.bg }, { t: ' Reply to ' + agent + '…', fg: T.dim, bg: T.bg });
    else { segs.push({ t: l, fg: T.strong, bg: T.bg }); if (last) segs.push({ t: '█', fg: T.acc, bg: T.bg }); }
    g.putSegs(X, y++, segs, {}, X + iw);
  });
  if (mode === 'working') g.putSegs(X, y++, [{ t: agent + ' is mid-turn. ', fg: T.text, bg: C }, { t: narrow ? 'Enter queues it.' : 'Enter queues this until the turn ends.', fg: T.dim, bg: C }], {}, X + iw);
  const hints = mode === 'working' ? [['Enter', 'queue', 'msgSend'], ['Ctrl+Enter', narrow ? 'now' : 'send now', 'msgNow'], ['Esc', 'cancel', 'msgClose']] : [['Enter', 'send', 'msgSend'], ['Shift+Enter', narrow ? 'line' : 'new line', 'msgNL'], ['Esc', 'cancel', 'msgClose']];
  let hx = X; hints.forEach(([k, l, t]) => { const x1 = hx; hx = g.putSegs(hx, y, [{ t: k, fg: T.acc, b: 1, bg: C }, { t: ' ' + l, fg: T.text, bg: C }]); R.push({ x: x1, y, w: hx - x1, h: 1, act: { t } }); hx += 3; });
  const words = empty ? 0 : s.text.join(' ').split(/\s+/).filter(Boolean).length;
  if (words) { const wc = words + (words === 1 ? ' word' : ' words'); if (hx + L(wc) < X + iw) g.put(X + iw - L(wc), y, wc, { fg: T.dim, bg: C }); }
  if (up && !ascii) {}
}

export function render(s) { return C7.render({ ...s, overlay, patchF }); }
function send(s, now) {
  const txt = s.text.join(' ').trim(); if (!txt) return { state: { ...s, mopen: 0 } };
  const q = rowsOf(s)[s.sel], first = trunc(s.text.find(Boolean) || txt, 44);
  if (q.k === 'working' && !now) return { state: { ...s, mopen: 0, text: [''], after: 'queued', sentFor: q.id, sentText: first }, fx: [{ after: 3200, act: { t: 'turnEnd' } }] };
  return { state: { ...s, mopen: 0, text: [''], after: 'sent', sentFor: q.id, sentText: first } };
}
export function act(s, a) {
  const S = p => ({ state: { ...s, ...p } });
  switch (a.t) {
    case 'noop': return null;
    case 'msgSend': return send(s, false);
    case 'msgNow': return send(s, true);
    case 'msgNL': return S({ text: [...s.text, ''] });
    case 'msgClose': return S({ mopen: 0 });
    case 'turnEnd': return s.after === 'queued' ? S({ after: 'sent' }) : null;
    case 'selSess': return C7.act({ ...s, mopen: 0 }, a);
    case 'key': {
      const k = a.key;
      if (s.mopen) {
        if (k === 'Escape') return S({ mopen: 0, draftFor: s.sel });
        if (k === 'Enter') return send(s, false);
        if (k === 'C-Enter') return send(s, true);
        if (k === 'S-Enter') return S({ text: [...s.text, ''] });
        if (k === 'Backspace') { const t = [...s.text], l = t[t.length - 1]; if (l.length) t[t.length - 1] = Array.from(l).slice(0, -1).join(''); else if (t.length > 1) t.pop(); return S({ text: t }); }
        if (Array.from(k).length === 1) { const t = [...s.text]; t[t.length - 1] += k; return S({ text: t }); }
        return null;
      }
      if (k === 'm' && s.focus === 'side') return S({ mopen: 1, text: s.draftFor === s.sel ? s.text : [''], after: null, draftFor: s.sel });
      return C7.act(s, a);
    }
  }
  return C7.act(s, a);
}
