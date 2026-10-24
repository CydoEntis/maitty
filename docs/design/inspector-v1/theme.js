// Maitty theme applied to the current app layout (sidebar + session pane), in its four layout styles. SAMPLE DATA ONLY.
import { Grid, trunc, L, mix } from './seshi-grid.js';

const DARK = {
  name: 'dark', bg: '#08111d', fg: '#c9d6e3', panel: '#0b1624', tint: '#15253a', btn: '#1a2c44', line: '#1d2d42', dim: '#6f8399', text: '#c9d6e3', strong: '#eef5fb',
  acc: '#3dd6c6', ink: '#08111d', brass: '#e9b44c', needs: '#f5c542', ok: '#7fd99a', err: '#ff7b72', sea: '#1f6f8b', scrim: '#000000',
  a: { green: '#7fd99a', red: '#ff7b72', yellow: '#f2c75c', blue: '#6cb6ff', magenta: '#c4a7ff', cyan: '#3dd6c6' },
};
const LIGHT = {
  name: 'light', bg: '#f6f8fa', fg: '#1f2f3f', panel: '#eef3f6', tint: '#e0edf2', btn: '#e4ebf1', line: '#d3dce5', dim: '#61748a', text: '#1f2f3f', strong: '#0b1724',
  acc: '#0f8f84', ink: '#ffffff', brass: '#a8741a', needs: '#b06a00', ok: '#2e8b57', err: '#c93c37', sea: '#5aa3bd', scrim: '#000000',
  a: { green: '#2e8b57', red: '#c93c37', yellow: '#9a6700', blue: '#2f6fbf', magenta: '#7e57c2', cyan: '#0f8f84' },
};
const MIDNIGHT = {
  name: 'dark', bg: '#0c0e11', fg: '#c4ccd4', panel: '#111418', tint: '#1a2128', btn: '#1d242b', line: '#252c34', dim: '#717c87', text: '#c4ccd4', strong: '#eef2f5',
  acc: '#45dccb', ink: '#0c0e11', brass: '#d9a441', needs: '#f2c14e', ok: '#6fd391', err: '#ff6f66', sea: '#1a5f78', scrim: '#000000',
  a: { green: '#6fd391', red: '#ff6f66', yellow: '#eac25a', blue: '#62a8f5', magenta: '#b79cff', cyan: '#45dccb' },
};
let T = DARK;
export const MODES = { light: 'Day', dark: 'Night', midnight: 'Midnight' };
export const LAYOUTS = { cards: 'Square', rounded: 'Rounded', pillcards: 'Rounded · pills', pill: 'Flat · pills', flat: 'Flat' };
const SPIN = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
const PROJ = [
  { n: 'dev', m: 'no git', l: [{ id: 'claude', age: '3h', k: 'idle' }] },
  { n: 'browser-4x-game', m: '', l: [{ id: 'Commander', k: 'group' }] },
  { n: 'pixls', m: '⎇ phase3-owner-gate-opt…', l: [{ id: 'warm-finch', k: 'group' }] },
  { n: 'linkaroo', m: '', l: [{ id: 'bright-owl', age: '2h', k: 'needs' }] },
  { n: 'seshi', m: '⎇ dev', l: [{ id: 'Bug Fix', age: '1h', k: 'idle' }, { id: 'Docs Pruning', age: '3h', k: 'idle' }, { id: 'TUIOS Research', age: '4m', k: 'working', sel: 1 }] },
  { n: 'snipux', m: '⎇ dev', l: [{ id: 'Snipux Bugs', age: '1h', k: 'idle' }, { id: 'Memory Hog', age: '2h', k: 'idle' }] },
];
function out(f) {
  const A = T.a, n = (k, c) => ({ t: String(k).padStart(6) + ' ', fg: c }), add = (k, t) => [n(k, A.green), { t: '+ ' + t, fg: T.text }];
  return [
    add(1052, '"sysinfo",'), add(1053, '"tokio",'), add(1054, '"tokio-util",'), add(1055, '"toml",'), add(1056, '"toml_edit",'), add(1057, '"tracing",'), add(1058, '"tracing-subscriber",'), add(1059, '"unicode-width",'), add(1060, '"vt100",'), add(1061, '"windows-sys 0.61.2",'), [n(1062, A.green), { t: '+]', fg: T.text }],
    [{ t: '     … 46 more lines (ctrl+o to expand)', fg: T.dim }],
    [{ t: '   └ Updated ', fg: T.text }, { t: 'docs\\ROADMAP.md', fg: A.cyan }, { t: ' (+1 −0)', fg: T.dim }],
    [n(720, T.dim), { t: '- [', fg: T.text }, { t: 'ADR-0011', fg: A.green }, { t: '](adr/0011-keep-terminal-first.md) — Keep Maitty terminal-first.', fg: T.text }],
    [n(721, A.green), { t: '+- [', fg: T.text }, { t: 'ADR-0020', fg: A.green }, { t: '](adr/0020-rename-to-maitty.md) — The app is called Maitty, "All agents on deck".', fg: T.text }],
    [n(722, T.dim), { t: '- [', fg: T.text }, { t: 'ADR-0019', fg: A.green }, { t: '](adr/0019-the-name-lives-in-one-place.md) — The app\'s name lives in Cargo.toml.', fg: T.text }],
    [{ t: '   └ Allowed by auto mode classifier', fg: T.dim }], [],
    [{ t: '● ', fg: A.green }, { t: 'Bash', fg: T.strong, b: 1 }, { t: '(cd C:/dev/seshi-worktrees/tall-crane && cargo clippy --all-targets -- -D warnings)', fg: T.text }],
    [{ t: '   └ error: constant `APP_TAGLINE` is never used', fg: T.text }], [{ t: '       --> src\\brand.rs:10:11', fg: T.dim }], [],
    [{ t: '● ', fg: T.text }, { t: 'The splash reads the tagline straight from the build value, so the new constant is unused. Removing it:', fg: T.text }], [],
    [{ t: '● ', fg: A.green }, { t: 'Update', fg: T.strong, b: 1 }, { t: '(src\\brand.rs)', fg: T.text }],
    [n(8, T.dim), { t: 'pub const ', fg: A.magenta }, { t: 'APP_NAME: &', fg: T.text }, { t: 'str', fg: A.magenta }, { t: ' = ', fg: T.text }, { t: 'env!', fg: A.cyan }, { t: '("APP_NAME");', fg: A.green }],
    [n(9, A.red), { t: '-/// A few words under the name, on the splash.', fg: T.dim }],
    [n(10, A.red), { t: '-pub const APP_TAGLINE: &str = env!("APP_TAGLINE");', fg: T.dim }], [],
    [{ t: '✶ ', fg: A.yellow }, { t: 'Fluttering… ', fg: A.yellow }, { t: '(5m 11s · ↓ 8.4k tokens)', fg: T.dim }],
  ];
}
function box(g, x, y, w, h, c, round) { const k = round ? '╭╮╰╯' : '┌┐└┘', s = { fg: c }; g.set(x, y, k[0], s); g.set(x + w - 1, y, k[1], s); g.set(x, y + h - 1, k[2], s); g.set(x + w - 1, y + h - 1, k[3], s); g.hl(x + 1, y, w - 2, '─', s); g.hl(x + 1, y + h - 1, w - 2, '─', s); g.vl(x, y + 1, h - 2, '│', s); g.vl(x + w - 1, y + 1, h - 2, '│', s); }
const pill = (segs, bg, on) => on ? [{ t: '◖', fg: bg }, ...segs.map(q => ({ ...q, bg })), { t: '◗', fg: bg }] : segs.map(q => ({ ...q, bg }));

export function render(s) {
  T = s.mode === 'light' ? LIGHT : s.mode === 'midnight' ? MIDNIGHT : DARK;
  const W = s.W, H = s.H, g = new Grid(W, H, T), R = [], f = s.motion ? Math.floor(s.f / (s.div || 2)) : 2, lay = s.layout, boxed = lay === 'cards' || lay === 'rounded' || lay === 'pillcards', round = lay === 'rounded' || lay === 'pillcards', pills = lay === 'pill' || lay === 'pillcards';
  const SW = 38, sideLit = s.focus === 'side', paneLit = s.focus === 'pane';
  // sidebar
  const sx = 0, sw = SW, sg = new Grid(SW, H, T); let noteY = null;
  if (boxed) box(sg, sx, 0, sw, H, sideLit ? T.acc : T.line, round);
  sg.put(sx + sw - 16, 0, ' context left ', { fg: T.dim }); sg.putSegs(sx + 1, 0, pill([{ t: ' SESSIONS ', fg: sideLit ? T.ink : T.strong, b: 1 }, { t: '9 ', fg: sideLit ? T.ink : T.dim }], sideLit ? T.acc : T.btn, pills));
  let y = 2; const X = sx + 2, RX = sx + sw - 3;
  for (const p of PROJ) {
    sg.putSegs(X, y, [{ t: '– ', fg: T.brass, b: 1 }, { t: p.n, fg: T.strong, b: 1 }]); if (p.m) { const m = trunc(p.m, RX - X - L(p.n) - 3); sg.put(RX + 1 - L(m), y, m, { fg: T.dim }); } y++;
    for (const q of p.l) {
      if (q.k === 'group') { sg.putSegs(X + 1, y, [{ t: '› ', fg: T.dim }, { t: q.id, fg: T.text }]); y++; continue; }
      const on = !!q.sel, nd = q.k === 'needs', bg = on ? T.tint : nd ? mix(T.bg, T.needs, 0.1 + 0.08 * Math.sin(f * 0.9)) : undefined;
      if (nd && !on) { sg.fill(sx + 1, y, sw - 2, 2, bg); sg.vl(sx + 1, y, 2, '▎', { fg: T.needs, bg, b: 1 }); }
      if (on) { sg.fill(sx + 1, y, sw - 2, 2, T.tint); if (pills) { sg.set(sx + 1, y, '◖', { fg: T.tint, bg: T.bg }); sg.set(sx + sw - 2, y, '◗', { fg: T.tint, bg: T.bg }); } else sg.vl(sx + 1, y, 2, '▎', { fg: T.acc, bg, b: 1 }); }
      sg.putSegs(X + 1, y, [q.k === 'working' ? { t: SPIN[f % 8] + ' ', fg: T.acc, b: 1, bg } : q.k === 'needs' ? { t: '● ', fg: T.needs, b: 1, bg } : { t: '○ ', fg: T.dim, bg }, { t: q.id, fg: on ? T.strong : T.text, b: on, bg }]); { const cs = (COST[q.id] != null ? '$' + COST[q.id].toFixed(2) + ' · ' : '') + q.age; sg.put(RX + 1 - L(cs), y, cs, { fg: T.dim, bg }); } if (q.id === s.note) noteY = y;
      { const got = s.hDone && q.id === HTARGETS[s.hSel || 0], w2 = got ? '↙ from TUIOS Research' : q.k === 'working' ? 'working' : q.k === 'needs' ? 'needs you' : NOTE[q.id] ? '✎ ' + NOTE[q.id].short : 'idle'; sg.put(X + 3, y + 1, trunc(w2, RX - X - 13), { fg: got || q.k === 'working' ? T.acc : q.k === 'needs' ? T.needs : NOTE[q.id] ? T.text : T.dim, b: nd || got, bg }); }
      { const pct = CTX[q.id] ?? 50, n = Math.round(pct / 20), c = pct < 15 ? T.err : pct < 35 ? T.needs : T.dim, m = [{ t: '▰'.repeat(n), fg: c, bg }, { t: '▱'.repeat(5 - n), fg: mix(T.bg, T.dim, 0.5), bg }, { t: String(pct).padStart(4) + '%', fg: c, b: pct < 15, bg }]; sg.putSegs(RX + 1 - 10, y + 1, m); } y += 2;
    }
    y++;
  }
  { const spent = Object.values(COST).reduce((a, b) => a + b, 0), by = H - (boxed ? 2 : 1) - 2, n = Math.round(spent / BUDGET * 10); sg.putSegs(X, by, [{ t: 'today ', fg: T.dim }, { t: '$' + spent.toFixed(2), fg: T.strong, b: 1 }, { t: ' of $' + BUDGET + '  ', fg: T.dim }, { t: '▰'.repeat(n), fg: T.brass }, { t: '▱'.repeat(10 - n), fg: mix(T.bg, T.dim, 0.5) }]); }
  const fy = H - (boxed ? 2 : 1); sg.hl(X - 1, fy - 1, sw - 3, '─', { fg: T.line });
  sg.putSegs(X, fy, pill([{ t: ' + ', fg: T.acc, b: 1 }, { t: 'New ', fg: T.strong }], T.btn, pills)); const mn = pill([{ t: ' Menu ', fg: T.strong }], T.btn, pills); sg.putSegs(RX + 2 - mn.reduce((n, q) => n + L(q.t), 0), fy, mn);
  // sidebar gives up its room when the inspector opens (unless pinned): it folds to a rail of session glyphs
  const DO0 = openness(s), fold = s.pinSide ? 0 : DO0, SWN = Math.round(SW - (SW - 8) * fold);
  if (SWN > 14) { for (let yy = 0; yy < H; yy++) for (let xx = 0; xx < SWN; xx++) g.c[yy][xx] = sg.c[yy][xx]; if (boxed && SWN < SW) { const c = sideLit ? T.acc : T.line; g.vl(SWN - 1, 1, H - 2, '│', { fg: c }); g.set(SWN - 1, 0, round ? '╮' : '┐', { fg: c }); g.set(SWN - 1, H - 1, round ? '╯' : '┘', { fg: c }); } }
  else {
    if (boxed) box(g, 0, 0, SWN, H, sideLit ? T.acc : T.line, round);
    g.putSegs(1, 0, pill([{ t: ' » ', fg: sideLit ? T.ink : T.strong, b: 1 }], sideLit ? T.acc : T.btn, pills)); R.push({ x: 1, y: 0, w: 4, h: 1, act: { t: 'pinSide' } });
    let ry = 2; PROJ.forEach((p, pi) => { if (pi) { g.hl(2, ry, SWN - 4, '·', { fg: T.line }); ry++; } p.l.forEach(q => { if (q.k === 'group') return; const on = !!q.sel, nd = q.k === 'needs', bg = on ? T.tint : nd ? mix(T.bg, T.needs, 0.1 + 0.08 * Math.sin(f * 0.9)) : undefined, pct = CTX[q.id] ?? 50;
      if (on || nd) { g.fill(1, ry, SWN - 2, 1, bg); g.set(1, ry, '▎', { fg: nd ? T.needs : T.acc, bg, b: 1 }); }
      g.putSegs(2, ry, [q.k === 'working' ? { t: SPIN[f % 8], fg: T.acc, b: 1, bg } : nd ? { t: '●', fg: T.needs, b: 1, bg } : { t: '○', fg: T.dim, bg }, { t: ' ' + q.id[0] + ' ', fg: on || nd ? T.strong : T.dim, b: on || nd, bg }, { t: '▁▂▃▄▅▆▇█'[Math.min(7, Math.floor(pct / 12.5))], fg: pct < 15 ? T.err : pct < 35 ? T.needs : T.dim, bg }]); ry++; }); });
    g.putSegs(2, H - (boxed ? 2 : 1), [{ t: ' + ', fg: T.acc, bg: T.btn, b: 1 }]);
  }
  R.push({ x: 0, y: 0, w: SWN, h: H, act: { t: 'focus', v: 'side' } });
  // pane
  const DFULL = dWidth(s), DO = openness(s), DW = Math.round(DFULL * DO), dx = W - DW;
  const px = boxed ? SWN + 1 : SWN, pw = (DW ? dx - (boxed ? 1 : 0) : W) - px;
  const TO = tOpenness(s), DH = 1 + Math.round((tFull(s) - 1) * TO), PH = H - DH;
  if (boxed) box(g, px, 0, pw, PH, paneLit ? T.acc : T.line, round); else g.vl(px, 0, H, '│', { fg: paneLit ? T.acc : T.line, b: paneLit });
  if (!boxed) g.fill(px + 1, 0, pw - 1, 1, T.bg);
  let tx = g.putSegs(px + 2, 0, pill([{ t: ' 1  TUIOS Research ', fg: paneLit ? T.ink : T.strong, b: 1 }], paneLit ? T.acc : T.btn, pills));
  g.putSegs(tx + 1, 0, [{ t: ' + ', fg: T.acc, b: 1 }]); if (s.watch) { g.putSegs(tx + 5, 0, [{ t: ' WATCH ', fg: T.ink, bg: T.acc, b: 1 }]); tx += 8; }
  const tabEnd = tx + 4, rlen = q => q.reduce((n, z) => n + L(z.t), 0);
  let right = [{ t: '▲ 1% CPU · 2.2 GB  ', fg: T.dim }, { t: SPIN[f % 8] + ' ', fg: T.acc, b: 1 }, { t: 'working', fg: T.acc, b: 1 }, { t: '  ✕ ', fg: T.dim }];
  if (px + pw - 2 - rlen(right) < tabEnd + 1) right = right.slice(1);
  if (px + pw - 2 - rlen(right) < tabEnd + 1) right = [{ t: SPIN[f % 8] + ' ', fg: T.acc, b: 1 }, { t: '✕ ', fg: T.dim }];
  const rx0 = px + pw - 2 - right.reduce((n, q) => n + L(q.t), 0); g.putSegs(rx0, 0, right);
  let chipR = null; if (!s.dOpen) { const ins = pill([{ t: ' Inspect ', fg: T.text }, { t: 'f ', fg: T.acc, b: 1 }], T.btn, pills), iw = ins.reduce((n, q) => n + L(q.t), 0); g.putSegs(rx0 - iw - 2, 0, ins); chipR = { x: rx0 - iw - 2, y: 0, w: iw, h: 1, act: { t: 'drawer', tab: 'files' } }; }
  const cx = px + 3, cw = pw - 6, lines = [...out(f), ...(s.sent ? [[], [{ t: '› ', fg: T.dim }, { t: 'The tests terminal failed. Fix this:', fg: T.strong, b: 1 }], [{ t: '  test client::tests::caps_lock_does_not_open_what_happened ... FAILED', fg: T.err }], [{ t: '  assertion failed at src\\client\\tests.rs:2098:5', fg: T.text }], [], [{ t: '● ', fg: T.a.green }, { t: 'Read', fg: T.strong, b: 1 }, { t: '(src\\client\\tests.rs)', fg: T.text }], [{ t: SPIN[f % 8] + ' ', fg: T.acc }, { t: 'Looking at the Caps Lock test…', fg: T.acc }]] : [])], room = PH - 9;
  lines.slice(-room).forEach((l, k) => g.putSegs(cx, 2 + room - Math.min(room, lines.length) + k, l, {}, cx + cw));
  const links = [];
  for (let yy = 2; yy < PH - 6; yy++) { let str = ''; for (let xx = cx; xx < cx + cw; xx++) str += g.c[yy][xx].ch; const re = /src[\\\/](?:[\w-]+[\\\/])*([\w-]+\.rs)(?::(\d+))?/g; let m;
    while ((m = re.exec(str))) { const ti = TREE.findIndex(n => n.n === m[1] && !n.dir); if (ti < 0) continue; for (let k = 0; k < m[0].length; k++) { const c = g.c[yy][cx + m.index + k]; c.fg = T.acc; c.u = 1; } links.push({ x: cx + m.index, y: yy, w: m[0].length, h: 1, act: { t: 'jump', i: ti, line: m[2] ? +m[2] : null } }); } }
  const py = PH - 6; g.hl(cx, py, cw, '─', { fg: T.dim }); g.putSegs(cx, py + 1, [{ t: '› ', fg: T.dim }, ...(s.promptText ? [{ t: s.promptText, fg: T.strong }] : []), { t: paneLit ? '▏' : ' ', fg: T.strong }]); g.hl(cx, py + 2, cw, '─', { fg: T.dim });
  const bar = (p, c) => [{ t: '█'.repeat(Math.round(p / 10)), fg: c }, { t: '░'.repeat(10 - Math.round(p / 10)), fg: mix(c, T.bg, 0.6) }];
  g.putSegs(cx + 1, py + 3, [{ t: 'Opus 5.5', fg: T.a.blue }, { t: '  ·  tall-crane  ·  ', fg: T.dim }, { t: 'tall-crane', fg: T.a.green }, { t: '  ·  ', fg: T.dim }, { t: 'context ', fg: T.a.yellow }, ...bar(94, T.a.yellow), { t: ' 94%', fg: T.a.yellow }, { t: '  ·  5h ', fg: T.dim }, ...bar(2, T.a.yellow), { t: ' 2%', fg: T.a.yellow }], {}, cx + cw);
  const upd = [{ t: '✓', fg: T.acc, b: 1 }, { t: 'Update installed · Restart to update', fg: T.a.green }]; if (cw > 118) g.putSegs(cx + cw - upd.reduce((n, q) => n + L(q.t), 0), py + 3, upd);
  g.putSegs(cx + 1, py + 4, [{ t: '⏵⏵ auto mode on', fg: T.a.yellow }, { t: ' (shift+tab to cycle) · ← 1 agent', fg: T.dim }], {}, cx + cw);
  R.push({ x: px, y: 0, w: pw, h: PH, act: { t: 'focus', v: 'pane' } }); if (chipR) R.push(chipR);
  if (s.watch) { links.length = 0; g.fill(px + 1, 1, pw - 2, PH - 2, T.bg); const wx = px + Math.max(3, Math.floor((pw - 56) / 2)), ww = Math.min(56, pw - 6); let wy = 3;
    g.putSegs(wx, wy++, [{ t: 'TUIOS Research', fg: T.strong, b: 1 }, { t: '  claude · Opus 5.5', fg: T.dim }]); wy++;
    g.putSegs(wx, wy++, [{ t: SPIN[f % 8] + ' working', fg: T.acc, b: 1 }, { t: '  ·  step 4 of 6', fg: T.dim }]);
    g.putSegs(wx, wy++, [{ t: 'Fixing the Caps Lock test', fg: T.strong, b: 1 }]);
    g.putSegs(wx, wy++, [{ t: '▰'.repeat(Math.round(ww * 4 / 6)), fg: T.acc }, { t: '▱'.repeat(ww - Math.round(ww * 4 / 6)), fg: mix(T.bg, T.dim, 0.5) }]); wy++;
    g.put(wx, wy++, 'FILES TOUCHED', { fg: T.dim, b: 1 });
    [['src/client/tests.rs', 6 + Math.floor(f / 20) % 5, 6, 1], ['src/client/screen/draw.rs', 40, 18, 0], ['src/brand.rs', 0, 2, 0]].forEach(([p, a, r, live]) => { g.putSegs(wx, wy, [{ t: live ? '● ' : '  ', fg: T.acc }, { t: p, fg: live ? T.strong : T.text, b: live }]); const c = [{ t: '+' + a, fg: T.a.green }, { t: ' −' + r, fg: T.a.red }]; g.putSegs(wx + ww - c.reduce((n, q) => n + L(q.t), 0), wy++, c); }); wy++;
    g.putSegs(wx, wy++, [{ t: 'elapsed 12m 40s  ·  ', fg: T.dim }, { t: '$1.84', fg: T.text }, { t: '  ·  context ', fg: T.dim }, { t: '6% left', fg: T.err, b: 1 }]); wy++;
    g.putSegs(wx, wy, [{ t: ' w ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' leave watch   ', fg: T.dim }, { t: ' Enter ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' open full output', fg: T.dim }]); }
  links.forEach(l => R.push(l));
  if (s.queue) { const items = QUEUE.map((q, i) => [q, i]).filter(([, i]) => !(s.qDone || []).includes(i)), aw = Math.min(cw, 76), ah = items.length + 4, ax = cx, ay = PH - 6 - ah - 1;
    g.fill(ax, ay, aw, ah, T.panel); box(g, ax, ay, aw, ah, T.needs, round); g.putSegs(ax + 2, ay, pill([{ t: ' APPROVALS · ' + items.length + ' waiting ', fg: T.bg, b: 1 }], T.needs, pills));
    items.forEach(([[who, cmd, why], i], k) => { const yy = ay + 1 + k, on = k === 0, bg = on ? mix(T.panel, T.needs, 0.12) : T.panel; g.fill(ax + 1, yy, aw - 2, 1, bg); if (on) g.set(ax + 1, yy, '▎', { fg: T.needs, bg, b: 1 });
      g.putSegs(ax + 3, yy, [{ t: (who + '            ').slice(0, 13), fg: on ? T.strong : T.text, b: on, bg }, { t: '$ ' + cmd, fg: on ? T.strong : T.text, bg }], {}, ax + aw - L(why) - 4); g.put(ax + aw - 3 - L(why), yy, why, { fg: T.dim, bg }); });
    g.putSegs(ax + 3, ay + ah - 2, [{ t: ' 1 ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' allow once   ' }, { t: ' 2 ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' always   ' }, { t: ' 3 ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' deny   ' }, { t: ' Esc ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' later' }].map(q => ({ fg: T.dim, bg: q.bg || T.panel, ...q })));
    R.push({ x: ax, y: ay, w: aw, h: ah, act: { t: 'focus', v: 'pane' } }); }
  if (s.away) { const aw = Math.min(cw, 64), ah = 7, ax = cx, ay = PH - 6 - ah - 1; g.fill(ax, ay, aw, ah, T.panel); box(g, ax, ay, aw, ah, T.acc, round);
    g.putSegs(ax + 2, ay, pill([{ t: ' While you were away · 38m ', fg: T.ink, b: 1 }], T.acc, pills));
    const P = (yy, segs) => g.putSegs(ax + 2, yy, segs.map(q => ({ bg: T.panel, ...q })), {}, ax + aw - 2);
    P(ay + 1, [{ t: '✎ ', fg: T.a.yellow, b: 1 }, { t: '6 files changed  ', fg: T.text }, { t: '+116', fg: T.a.green }, { t: ' −37', fg: T.a.red }]);
    P(ay + 2, [{ t: '✓ ', fg: T.ok, b: 1 }, { t: 'cargo test  ', fg: T.text }, { t: '57 passed', fg: T.ok }, { t: ' · ', fg: T.dim }, { t: '1 failed', fg: T.err }]);
    P(ay + 3, [{ t: '● ', fg: T.needs, b: 1 }, { t: 'asked: use pnpm or npm for the workspace?', fg: T.needs }]);
    P(ay + 5, [{ t: ' Enter ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' see changes   ' }, { t: ' a ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' answer   ' }, { t: ' Esc ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' dismiss' }].map(q => ({ fg: T.dim, ...q })));
    R.push({ x: ax, y: ay, w: aw, h: ah, act: { t: 'awayGo' } }); }
  const tedge = drawDock(g, R, s, px, PH, pw, DH, boxed, round, pills, s.focus === 'dock', f);
  if (DW > 0) {
    const d = drawDrawer(s, DFULL, H, boxed, round, pills, s.focus === 'drawer');
    for (let y = 0; y < H; y++) for (let x = 0; x < DW; x++) { const c = d.g.c[y][x]; if (DO < 1 && c.fg) c.fg = mix(T.bg, c.fg, 0.35 + 0.65 * DO); g.c[y][dx + x] = c; }
    R.push({ x: dx, y: 0, w: DW, h: H, act: { t: 'focus', v: 'drawer' } }); d.R.forEach(r => { if (r.x < DW) R.push({ ...r, x: r.x + dx }); });
  }
  if (s.note && noteY != null && NOTE[s.note]) { const nt = NOTE[s.note], nx = Math.min(SWN + 1, W - 44), nw = 42, nh = nt.lines.length + 3; g.fill(nx, noteY, nw, nh, T.panel); box(g, nx, noteY, nw, nh, T.acc, round); g.set(nx, noteY + 1, '◂', { fg: T.acc, b: 1 });
    g.putSegs(nx + 2, noteY, pill([{ t: ' ✎ ' + s.note + ' left a note ', fg: T.ink, b: 1 }], T.acc, pills)); nt.lines.forEach((l, i) => g.putSegs(nx + 2, noteY + 1 + i, [{ t: l, fg: T.text, bg: T.panel }], {}, nx + nw - 2)); g.put(nx + 2, noteY + nh - 2, '2h ago · Esc close', { fg: T.dim, bg: T.panel }); }
  if (s.handoff) { g.dimAll(); const mw = Math.min(64, W - 10), mh = 16, mx = Math.floor((W - mw) / 2), my = 6; g.fill(mx, my, mw, mh, T.panel); box(g, mx, my, mw, mh, T.acc, round);
    g.putSegs(mx + 2, my, pill([{ t: ' Send to another session ', fg: T.ink, b: 1 }], T.acc, pills)); let yy = my + 2; const P = segs => g.putSegs(mx + 3, yy++, segs.map(q => ({ bg: T.panel, ...q })), {}, mx + mw - 3);
    P([{ t: 'From ', fg: T.dim }, { t: 'TUIOS Research', fg: T.strong, b: 1 }, { t: ': its findings · 4 points · 1.2k tokens', fg: T.dim }]);
    P([{ t: '▏ TUIOS draws panes with a damage map, not full redraws', fg: T.text }]); P([{ t: '▏ its theme file maps 1:1 to our roles …', fg: T.dim }]); yy++;
    P([{ t: 'TO', fg: T.dim, b: 1 }]); HTARGETS.forEach((n, i) => { const on = i === (s.hSel || 0), bg = on ? T.tint : T.panel; g.fill(mx + 1, yy, mw - 2, 1, bg); if (on) g.set(mx + 1, yy, '▎', { fg: T.acc, bg, b: 1 }); g.putSegs(mx + 3, yy, [{ t: n, fg: on ? T.strong : T.text, b: on, bg }]); R.push({ x: mx + 1, y: yy, w: mw - 2, h: 1, act: { t: 'hSel', i } }); yy++; });
    g.putSegs(mx + 3, my + mh - 2, [{ t: ' Enter ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' send   ' }, { t: ' Esc ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' cancel' }].map(q => ({ fg: T.dim, bg: q.bg || T.panel, ...q }))); }
  if (s.pal) { g.dimAll(); const pw2 = Math.min(72, W - 10), groups = palItems(s), n = groups.reduce((a, q) => a + q.items.length + 1, 0), ph2 = Math.min(H - 6, n + 5), qx = Math.floor((W - pw2) / 2), qy = 4; let k = 0, yy = qy + 1;
    g.fill(qx, qy, pw2, ph2, T.panel); box(g, qx, qy, pw2, ph2, T.acc, round);
    g.putSegs(qx + 2, yy, [{ t: '› ', fg: T.acc, b: 1, bg: T.panel }, { t: s.q || '', fg: T.strong, b: 1, bg: T.panel }, { t: '▏', fg: T.acc, bg: T.panel }, ...(s.q ? [] : [{ t: 'sessions, files, terminals, actions', fg: T.dim, bg: T.panel }])]);
    g.putSegs(qx + pw2 - 8, yy, [{ t: ' Esc ', fg: T.strong, bg: T.btn, b: 1 }]); yy++; g.hl(qx + 1, yy, pw2 - 2, '─', { fg: T.line, bg: T.panel }); yy++;
    const ql = (s.q || '').toLowerCase();
    groups.forEach(gr => { g.put(qx + 2, yy++, gr.g, { fg: T.dim, b: 1, bg: T.panel }); gr.items.forEach(it => { const on = k === (s.pSel || 0), bg = on ? T.tint : T.panel; g.fill(qx + 1, yy, pw2 - 2, 1, bg); if (on) g.set(qx + 1, yy, '▎', { fg: T.acc, bg, b: 1 });
      const ix = ql ? it.n.toLowerCase().indexOf(ql) : -1, name = ix < 0 ? [{ t: it.n, fg: on ? T.strong : T.text, b: on, bg }] : [{ t: it.n.slice(0, ix), fg: on ? T.strong : T.text, bg }, { t: it.n.slice(ix, ix + ql.length), fg: T.acc, b: 1, bg }, { t: it.n.slice(ix + ql.length), fg: on ? T.strong : T.text, bg }];
      const ic = { prompt: '★ ', session: '◉ ', file: '▤ ', term: '▸ ', action: '⚡ ' }[it.k];
      g.putSegs(qx + 3, yy, [{ t: ic, fg: T.brass, bg }, ...name, ...(it.sub ? [{ t: '  ' + it.sub, fg: T.dim, bg }] : [])], {}, qx + pw2 - 12);
      const tag = { prompt: 'insert', session: 'open', file: 'open file', term: 'show', action: 'run' }[it.k]; g.put(qx + pw2 - 3 - L(tag), yy, tag, { fg: T.dim, bg });
      R.push({ x: qx + 1, y: yy, w: pw2 - 2, h: 1, act: { t: 'palRun', k } }); yy++; k++; }); });
  }
  return { rows: g.rows(), regions: R, W, H, bg: T.bg, edge: DW ? dx : null, tedge, tx0: px, tx1: px + pw };
}
// ---------- terminal dock: under the agent pane, like an editor's panel. Terminals belong to the session and keep running when hidden.
const TERMS = [
  { n: 'dev server', cmd: 'cargo run -- --port 4000', st: 'run', meta: 'running · pid 48211 · 12m', out: ['   Compiling maitty v0.42.0 (C:\\dev\\maitty-worktrees\\grey-wolf)', '    Finished `dev` profile [unoptimized] in 3.81s', '     Running `target\\debug\\maitty.exe --port 4000`', ' INFO maitty::server: listening on 127.0.0.1:4000', ' INFO maitty::server: GET /health 200 0.4ms', ' INFO maitty::server: GET /sessions 200 2.1ms'] },
  { n: 'tests', cmd: 'cargo watch -x test', st: 'fail', meta: 'watching · last run failed 14:22', out: ['running 58 tests', 'test client::tests::new_opens_no_form ... ok', 'test client::tests::caps_lock_does_not_open_what_happened ... FAILED', '', 'failures:', '  assertion failed: matches!(app.mode, Mode::Prefix { .. })', '  at src\\client\\tests.rs:2098:5', '', 'test result: FAILED. 57 passed; 1 failed; finished in 1.92s', '[Watching for changes…]'] },
  { n: 'zsh', cmd: '', st: 'idle', meta: 'idle', out: ['~/dev/maitty-worktrees/grey-wolf on ⎇ grey-wolf', '❯ git status -sb', '## grey-wolf...origin/grey-wolf [ahead 3]', ' M src/client/tests.rs', ' M src/client/screen/draw.rs'] },
];
function tOpenness(s) { const tgt = s.tOpen ? 1 : 0; if (!s.motion || s.tT0 == null) return tgt; const p = Math.min(1, Math.max(0, (s.f - s.tT0) / 9)), e = 1 - Math.pow(1 - p, 3), fr = s.tFrom ?? 0; return fr + (tgt - fr) * e; }
const tFull = s => Math.max(7, Math.min(s.H - 16, s.tH ?? Math.round(s.H * 0.36)));
const tState = (s, i) => (s.tKilled || {})[i] ? 'stopped' : TERMS[i].st;
const tGlyph = (st, f) => st === 'run' ? { t: '●', fg: T.ok } : st === 'fail' ? { t: '✕', fg: T.err } : st === 'stopped' ? { t: '■', fg: T.dim } : { t: '○', fg: T.dim };
function drawDock(g, R, s, x, y, w, h, boxed, round, pills, lit, f) {
  const sel = s.tSel ?? 0;
  if (h <= 2) {
    const yy = y + h - 1, x0 = x + (boxed ? 0 : 1), ww = w - (boxed ? 0 : 1);
    g.fill(x0, yy, ww, 1, T.panel);
    let cx = g.putSegs(x0 + 2, yy, [{ t: '▴ ', fg: T.dim, bg: T.panel }, { t: 'TERMINALS', fg: T.dim, b: 1, bg: T.panel }]) + 2;
    TERMS.forEach((tm, i) => { const gl = tGlyph(tState(s, i), f), segs = [{ t: ' ' + gl.t + ' ', fg: gl.fg, bg: T.btn, b: 1 }, { t: tm.n + ' ', fg: T.text, bg: T.btn }], x1 = cx; if (cx + 16 > x0 + ww - 14) return; cx = g.putSegs(cx, yy, segs) + 1; R.push({ x: x1, y: yy, w: cx - x1 - 1, h: 1, act: { t: 'term', i } }); });
    const hint = [{ t: ' ` ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' show ', fg: T.dim, bg: T.panel }]; g.putSegs(x0 + ww - 10, yy, hint); R.push({ x: x0 + ww - 10, y: yy, w: 9, h: 1, act: { t: 'tToggle' } });
    return null;
  }
  const c = lit ? T.acc : T.line;
  if (boxed) box(g, x, y, w, h, c, round); else { g.fill(x + 1, y, w - 1, h, T.bg); g.hl(x + 1, y, w - 1, '─', { fg: c }); }
  let tx = x + 2;
  TERMS.forEach((tm, i) => { const on = i === sel, gl = tGlyph(tState(s, i), f), x1 = tx;
    const segs = on ? pill([{ t: ' ' + gl.t + ' ', fg: lit ? T.ink : gl.fg, b: 1 }, { t: tm.n + ' ', fg: lit ? T.ink : T.strong, b: 1 }], lit ? T.acc : T.btn, pills) : [{ t: ' ' + gl.t + ' ', fg: gl.fg, b: 1 }, { t: tm.n + ' ', fg: T.dim }];
    tx = g.putSegs(tx, y, segs) + 1; R.push({ x: x1, y, w: tx - x1 - 1, h: 1, act: { t: 'term', i } }); });
  const nx = tx; tx = g.putSegs(tx, y, [{ t: ' + ', fg: T.acc, b: 1 }]); R.push({ x: nx, y, w: 3, h: 1, act: { t: 'toast' } });
  const rt = [{ t: ' ▾ hide ', fg: T.text }, { t: ' ■ stop ', fg: T.text }], rw = rt.reduce((n, q) => n + L(q.t), 0), rx = x + w - 3 - rw;
  const hot = s.tHot || s.tDrag, gap = rx - tx - 1; if (gap >= 9) { const mid = tx + Math.floor(gap / 2); for (let k = -3; k <= 3; k++) g.set(mid + k, y, hot ? '━' : '╍', { fg: hot ? T.acc : T.dim }); }
  g.putSegs(rx, y, rt); R.push({ x: rx, y, w: 8, h: 1, act: { t: 'tToggle' } }); R.push({ x: rx + 8, y, w: 8, h: 1, act: { t: 'tKill' } });
  const tm = TERMS[sel], st = tState(s, sel), X = x + 2, RX = x + w - 3, top = y + 1, bot = y + h - (boxed ? 2 : 1);
  const meta = st === 'stopped' ? 'stopped by you · output kept' : tm.meta;
  if (st === 'fail') { const bt = s.sent ? [{ t: ' ✓ sent to TUIOS Research ', fg: T.ok, b: 1 }] : [{ t: ' ↗ Send failure to agent ', fg: T.ink, bg: T.acc, b: 1 }, { t: ' a ', fg: T.ink, bg: mix(T.acc, T.bg, 0.25), b: 1 }], bw = bt.reduce((n, q) => n + L(q.t), 0);
    g.putSegs(X, top, [{ t: '$ ' + tm.cmd, fg: T.dim }], {}, RX - bw - 2); g.putSegs(RX - bw, top, bt); if (!s.sent) R.push({ x: RX - bw, y: top, w: bw, h: 1, act: { t: 'send' } }); }
  else { g.putSegs(X, top, [{ t: tm.cmd ? '$ ' + tm.cmd : 'shell', fg: T.dim }], {}, RX - L(meta) - 2); g.put(RX - L(meta), top, meta, { fg: st === 'run' ? T.ok : T.dim }); }
  let lines = tm.out.slice();
  if (sel === 0 && st === 'run') { const n = Math.floor(f / 5); for (let k = 0; k < n % 40; k++) lines.push(' INFO maitty::server: ' + ['GET /sessions 200 1.8ms', 'GET /events 101 upgrade', 'POST /sessions/grey-wolf/input 204 0.9ms', 'GET /health 200 0.3ms'][k % 4]); }
  if (st === 'stopped') lines.push('^C', '[process exited · press r to restart]');
  if (sel === 2) lines.push('❯ ' + (lit ? '▏' : ''));
  const room = bot - top - 1, show = lines.slice(-room);
  show.forEach((l, k) => { const yy = bot - show.length + 1 + k; const fg = /FAILED|failed|error|assertion/.test(l) ? T.err : / ok$|Finished|listening/.test(l) ? T.ok : l.startsWith('❯') || l.startsWith('$') ? T.strong : l.startsWith('[') || l.startsWith('  at') ? T.dim : T.text; g.putSegs(X, yy, [{ t: l, fg }], {}, RX); });
  R.push({ x, y: y + 1, w, h: h - 1, act: { t: 'focus', v: 'dock' } });
  return y;
}
export function initial() { return { W: 160, H: 45, f: 0, motion: true, layout: 'cards', focus: 'pane', mode: 'light' }; }
function toggle(s, tab, set) {
  const cur = openness(s);
  if (tab == null || (s.dOpen && s.dTab === tab && !set)) return { ...s, dOpen: false, dFrom: cur, dT0: s.f, focus: 'pane' };
  if (s.dOpen) return { ...s, dTab: tab, focus: 'drawer' };
  return { ...s, dOpen: true, dTab: tab, dFrom: cur, dT0: s.f, focus: 'drawer' };
}
export function act(s, a) { const r = act0(s, a); if (r && r.dOpen && s.dOpen && modeKey(r) !== modeKey(s)) return { ...r, wFrom: dWidth(s), wT0: s.f }; return r; }
function tToggle(s, open, i) { const o = open ?? !s.tOpen; return { ...s, tOpen: o, tFrom: tOpenness(s), tT0: o === !!s.tOpen ? s.tT0 : s.f, tSel: i ?? s.tSel ?? 0, focus: o ? 'dock' : 'pane' }; }
function act0(s, a) {
  if (a.t === 'focus') return { ...s, focus: a.v };
  if (a.t === 'tToggle') return tToggle(s);
  if (a.t === 'jump') { const o = s.dOpen ? s : { ...s, dOpen: true, dFrom: openness(s), dT0: s.f }; return { ...o, dTab: 'files', dFile: a.i, dSel: { ...(s.dSel || {}), files: a.i }, dLine: a.line, focus: 'drawer' }; }
  if (a.t === 'send') return { ...s, sent: true };
  if (a.t === 'hSel') return { ...s, hSel: a.i };
  if (a.t === 'awayGo') return { ...toggle({ ...s, away: false }, 'changes', 1) };
  if (a.t === 'cp') return { ...s, cpAsk: a.i, cpDone: null, focus: 'drawer' };
  if (a.t === 'palRun') { const flat = palItems(s).flatMap(g => g.items), it = flat[a.k]; const c = { ...s, pal: false, q: '' }; if (!it) return c;
    if (it.k === 'prompt') return { ...c, promptText: it.n, focus: 'pane' }; if (it.k === 'file') return act0(c, { t: 'jump', i: it.i, line: null }); if (it.k === 'term') return tToggle(c, true, it.i); if (it.k === 'session') return { ...c, focus: 'pane' };
    if (it.n === 'Run tests') return tToggle(c, true, 1); if (it.n === 'Toggle file drawer') return toggle(c, 'files'); if (it.n === 'Toggle terminal dock') return tToggle(c); if (it.n === 'Save a checkpoint') return toggle(c, 'branch', 1); return c; }
  if (a.t === 'term') return tToggle(s, true, a.i);
  if (a.t === 'tKill') return { ...s, tKilled: { ...(s.tKilled || {}), [s.tSel ?? 0]: 1 } };
  if (a.t === 'th') return { ...s, tT0: null, tH: Math.max(7, Math.min(s.H - 16, Math.round(a.h))) };
  if (a.t === 'drawer') return toggle(s, a.tab, a.set);
  if (a.t === 'dsel') { const sel = { ...(s.dSel || {}), [a.tab]: a.i }; if (a.tab === 'files') { const n = TREE[a.i]; if (n.dir) return { ...s, focus: 'drawer', dSel: sel, dirs: { ...(s.dirs || {}), [a.i]: !isClosed(s, a.i) } }; return { ...s, focus: 'drawer', dSel: sel, dFile: a.i }; } return { ...s, focus: 'drawer', dSel: sel }; }
  if (a.t === 'dback') return { ...s, dFile: null, focus: 'drawer' };
  if (a.t === 'dw') return { ...s, wT0: null, dWs: { ...(s.dWs || {}), [modeKey(s)]: Math.max(32, Math.min(dMax(s), Math.round(a.w))) } };
  if (a.t === 'pinSide') return { ...s, pinSide: !s.pinSide, focus: 'side' };
  if (a.t !== 'key') return null;
  const k = a.key, tab = s.dTab || 'files', inD = s.dOpen && s.focus === 'drawer', inT = s.tOpen && s.focus === 'dock';
  if (s.pal) { const n = palItems(s).reduce((a, q) => a + q.items.length, 0);
    if (k === 'Escape') return { ...s, pal: false }; if (k === 'Enter') return act0(s, { t: 'palRun', k: s.pSel || 0 });
    if (k === 'ArrowDown') return { ...s, pSel: Math.min(n - 1, (s.pSel || 0) + 1) }; if (k === 'ArrowUp') return { ...s, pSel: Math.max(0, (s.pSel || 0) - 1) };
    if (k === 'Backspace') return { ...s, q: (s.q || '').slice(0, -1), pSel: 0 }; if (k.length === 1) return { ...s, q: (s.q || '') + k, pSel: 0 }; return null; }
  if (s.handoff) { if (k === 'Escape') return { ...s, handoff: false }; if (k === 'Enter') return { ...s, handoff: false, hDone: true }; if (k === 'ArrowDown' || k === 'ArrowUp') return { ...s, hSel: Math.max(0, Math.min(HTARGETS.length - 1, (s.hSel || 0) + (k === 'ArrowDown' ? 1 : -1))) }; return null; }
  if (s.queue && ['1', '2', '3'].includes(k)) { const left = QUEUE.map((_, i) => i).filter(i => !(s.qDone || []).includes(i)); const qd = [...(s.qDone || []), left[0]]; return { ...s, qDone: qd, queue: qd.length < QUEUE.length }; }
  if (s.queue && k === 'Escape') return { ...s, queue: false };
  if (s.note && k === 'Escape') return { ...s, note: null };
  if (k === 'w' && !inD && !inT) return { ...s, watch: !s.watch, focus: 'pane' };
  if (k === 'h' && !inD && !inT) return { ...s, handoff: true, hSel: 0 };
  if (k === '/') return { ...s, pal: true, q: '', pSel: 0 };
  if (s.away && k === 'Escape') return { ...s, away: false };
  if (s.away && k === 'Enter') return act0(s, { t: 'awayGo' });
  if (s.cpAsk != null && (k === 'y' || k === 'n')) return { ...s, cpAsk: null, cpDone: k === 'y' ? s.cpAsk : null };
  if (inT && k === 'a' && tState(s, s.tSel ?? 0) === 'fail') return { ...s, sent: true };
  if (k === '`') return tToggle(s);
  if (inT && k === 'Escape') return tToggle(s, false);
  if (inT && (k === 'ArrowLeft' || k === 'ArrowRight')) return { ...s, tSel: ((s.tSel ?? 0) + (k === 'ArrowRight' ? 1 : TERMS.length - 1)) % TERMS.length };
  if (inT && k === 'x') return act0(s, { t: 'tKill' });
  if (inT && k === 'r' && (s.tKilled || {})[s.tSel ?? 0]) { const m = { ...s.tKilled }; delete m[s.tSel ?? 0]; return { ...s, tKilled: m }; }
  if (k === 's') return { ...s, pinSide: !s.pinSide };
  if (k === 'f') return toggle(s, 'files');
  if (k === 'g') return toggle(s, 'branch');
  if (k === 'd') { if (inD && tab === 'files') { const n = TREE[(s.dSel || {}).files ?? 0], ci = CHG.findIndex(c => n && c.f === n.n); return { ...s, dTab: 'changes', dSel: { ...(s.dSel || {}), changes: Math.max(0, ci) } }; } return toggle(s, 'changes'); }
  if (k === 'Escape' && inD && tab === 'files' && s.dFile != null && !splitOn(s)) return { ...s, dFile: null };
  if (k === 'Escape' && s.dOpen) return toggle(s, null);
  if (s.dOpen && (k === '[' || k === ']' || k === '=')) { if (k === '=') { const m = { ...(s.dWs || {}) }; delete m[modeKey(s)]; return { ...s, dWs: m, wFrom: dWidth(s), wT0: s.f }; } return act(s, { t: 'dw', w: dTarget(s) + (k === ']' ? 6 : -6) }); }
  if (inD && k === 'Enter') { if (tab === 'files' && (s.dFile == null || splitOn(s))) return act(s, { t: 'dsel', tab: 'files', i: (s.dSel || {}).files ?? 0 }); if (tab === 'changes') { const c = CHG[(s.dSel || {}).changes ?? 0], ti = TREE.findIndex(n => n.n === c.f); if (ti >= 0) return { ...s, dTab: 'files', dFile: ti, dSel: { ...(s.dSel || {}), files: ti } }; } return null; }
  if (k === 'Tab' || k === 'S-Tab') { const o = ['side', 'pane', ...(s.tOpen ? ['dock'] : []), ...(s.dOpen ? ['drawer'] : [])], i = o.indexOf(s.focus); return { ...s, focus: o[(i + (k === 'Tab' ? 1 : o.length - 1)) % o.length] }; }
  if (inD && tab === 'files' && (k === 'ArrowDown' || k === 'ArrowUp' || k === 'j' || k === 'k')) { if (s.dFile != null && !splitOn(s)) return null; const vis = visibleTree(s), cur = (s.dSel || {}).files ?? 0, at = Math.max(0, vis.indexOf(cur)), nx = vis[Math.max(0, Math.min(vis.length - 1, at + (k === 'ArrowDown' || k === 'j' ? 1 : -1)))]; return { ...s, dSel: { ...(s.dSel || {}), files: nx } }; }
  if (inD && (k === 'ArrowDown' || k === 'ArrowUp' || k === 'j' || k === 'k')) { const cur = (s.dSel || {})[tab] ?? 0, n = dLen(tab), nx = Math.max(0, Math.min(n - 1, cur + (k === 'ArrowDown' || k === 'j' ? 1 : -1))); return { ...s, dSel: { ...(s.dSel || {}), [tab]: nx } }; }
  if (inD && (k === 'ArrowLeft' || k === 'ArrowRight')) { const i = DTABS.findIndex(t => t[0] === tab); return { ...s, dTab: DTABS[(i + (k === 'ArrowRight' ? 1 : 2)) % 3][0] }; }
  return null;
}

// ---------- inspector drawer (files · changes · branch): slides in from the right and pushes the pane, so agent output stays visible
const DTABS = [['files', 'Files', 'f'], ['changes', 'Changes', 'd'], ['branch', 'Branch', 'g']];
const TREE = [
  { d: 0, n: 'docs', dir: 1 }, { d: 0, n: 'src', dir: 1, open: 1 }, { d: 1, n: 'cli', dir: 1 }, { d: 1, n: 'client', dir: 1, open: 1 }, { d: 2, n: 'screen', dir: 1, open: 1 },
  { d: 3, n: 'behaviour.rs' }, { d: 3, n: 'card.rs', m: 'M' }, { d: 3, n: 'check.rs' }, { d: 3, n: 'draw.rs', m: 'M', ed: 1 }, { d: 3, n: 'inbox.rs' }, { d: 3, n: 'leader.rs', m: 'M' }, { d: 3, n: 'popups.rs', m: 'M' }, { d: 3, n: 'side.rs' }, { d: 3, n: 'splash.rs' },
  { d: 2, n: 'actions.rs' }, { d: 2, n: 'branch.rs', m: 'A' }, { d: 2, n: 'tests.rs', m: 'M', ed: 1 }, { d: 1, n: 'main.rs' }, { d: 1, n: 'brand.rs', m: 'M' }, { d: 0, n: 'Cargo.toml' }, { d: 0, n: 'README.md' },
];
const CHG = [
  { f: 'tests.rs', dir: 'src/client', a: 6, r: 6, ed: 1 }, { f: 'draw.rs', dir: 'src/client/screen', a: 40, r: 18, ed: 1 }, { f: 'popups.rs', dir: 'src/client/screen', a: 22, r: 9 },
  { f: 'card.rs', dir: 'src/client/screen', a: 12, r: 3 }, { f: 'leader.rs', dir: 'src/client/screen', a: 5, r: 1 }, { f: 'branch.rs', dir: 'src/client', a: 31, r: 0, n: 1 },
];
const HUNK = { 'tests.rs': ['@@ -1953,7 +1953,7 @@', '         leader(&mut app, \'n\');', '     }', '-    assert_eq!(app.mode, Mode::Normal, "new opens no form");', '+    assert!(matches!(app.mode, Mode::Normal | Mode::Prefix { .. }));', '     let commands = sent(&mut rx);', '@@ -2098,4 +2098,4 @@', '     caps_state,', '-    assert_eq!(app.mode, Mode::Normal, "Caps Lock …");', '+    assert!(matches!(app.mode, Mode::Prefix { .. }), "Caps Lock …");', '@@ -1652,2 +1652,2 @@', '-    app.mode = Mode::Normal;', '+    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));'] };
const hunkFor = c => HUNK[c.f] || ['@@ -1,6 +1,' + (6 + c.a - c.r) + ' @@', '  use crate::screen::{Frame, Rect};', '-  fn draw(frame: &mut Frame) {', '+  fn draw(frame: &mut Frame, area: Rect) {', '+      let inner = area.inner(1);', '      frame.clear();', '      …'];
const COMMITS = [['a1f9c2e', 'Prefix mode stays after dialogs', '12m'], ['7be0d14', 'Splash reads tagline from build', '1h'], ['3c55e08', 'Rename seshi to maitty', '2h'], ['e02b7aa', 'Collapse sidebar into top strip', '1d']];
const dLen = tab => tab === 'files' ? TREE.length : tab === 'changes' ? CHG.length : COMMITS.length;
// width follows what's open: a bare tree or branch is narrow, a file or a diff gets room. Your drag is remembered per kind.
const modeKey = s => (s.dTab || 'files') === 'files' ? (s.dFile != null ? 'file' : 'tree') : s.dTab;
const DEF = { tree: 0.27, file: 0.55, changes: 0.5, branch: 0.3 };
const dMax = s => s.W - (s.pinSide ? 38 : 8) - 52;
const dTarget = s => Math.max(32, Math.min(dMax(s), (s.dWs || {})[modeKey(s)] ?? Math.round(s.W * DEF[modeKey(s)])));
function dWidth(s) { const t = dTarget(s); if (!s.motion || s.wT0 == null) return t; const p = Math.min(1, Math.max(0, (s.f - s.wT0) / 9)), e = 1 - Math.pow(1 - p, 3); return Math.round(s.wFrom + (t - s.wFrom) * e); }
const splitOn = s => s.dFile != null && dTarget(s) >= 60;
function isClosed(s, i) { return s.dirs && i in s.dirs ? s.dirs[i] : !!(TREE[i].dir && !TREE[i].open); }
function visibleTree(s) { const out = []; let hide = Infinity; TREE.forEach((n, i) => { if (n.d <= hide) hide = Infinity; if (n.d > hide) return; out.push(i); if (n.dir && isClosed(s, i)) hide = n.d; }); return out; }
function pathOf(i) { const p = [TREE[i].n]; let d = TREE[i].d; for (let j = i - 1; j >= 0 && d > 0; j--) if (TREE[j].d < d) { p.unshift(TREE[j].n); d = TREE[j].d; } return p.join('/'); }
const FILES = {
  'draw.rs': ['use crate::screen::{Frame, Rect, Style};', 'use crate::theme::Theme;', '', '/// Draws one session row in the sidebar.', 'pub fn session_row(f: &mut Frame, area: Rect, row: &Row, t: &Theme) {', '    let selected = row.selected;', '    let bg = if selected { t.tint } else { t.bg };', '    f.fill(area, bg);', '    if selected {', '        f.set(area.x, area.y, "▎", Style::fg(t.accent).bold());', '    }', '    let glyph = match row.state {', '        State::Working => spinner(row.frame),', '        State::NeedsYou => "●",', '        _ => "○",', '    };', '    f.text(area.x + 2, area.y, glyph, Style::fg(row.color(t)));', '    f.text(area.x + 4, area.y, &row.name, Style::fg(t.strong));', '    // second line: what the session is doing', '    f.text(area.x + 4, area.y + 1, row.word(), Style::fg(t.muted));', '}'],
  'tests.rs': ['#[test]', 'fn caps_lock_does_not_open_what_happened() {', '    let (mut app, mut rx) = shortcut_mode();', '    app.on_key(plain(KeyCode::CapsLock));', '    assert!(matches!(app.mode, Mode::Prefix { .. }), "Caps Lock must not open it");', '    assert!(sent(&mut rx).is_empty());', '}', '', '#[test]', 'fn new_opens_no_form() {', '    let (mut app, mut rx) = shortcut_mode();', '    leader(&mut app, \'n\');', '    let commands = sent(&mut rx);', '    assert_eq!(commands.len(), 1);', '}'],
  'Cargo.toml': ['[package]', 'name = "maitty"', 'version = "0.42.0"', 'edition = "2021"', '', '[dependencies]', 'crossterm = "0.28"', 'tokio = { version = "1", features = ["full"] }'],
  'README.md': ['# Maitty', '', 'All agents on deck.', '', 'Run every coding agent side by side in one terminal.'],
};
const CTX = { claude: 62, 'bright-owl': 41, 'Bug Fix': 80, 'Docs Pruning': 45, 'TUIOS Research': 6, 'Snipux Bugs': 70, 'Memory Hog': 28 };
FILES['brand.rs'] = ['//! Names the app shows, set once at build time.', '', '/// The app\'s name, from Cargo.toml.', 'pub const APP_NAME: &str = env!("APP_NAME");', '', '/// Shown under the name on the splash.', 'pub const APP_MOTTO: &str = env!("APP_MOTTO");', '', '/// A few words under the name, on the splash.', 'pub const APP_TAGLINE: &str = env!("APP_TAGLINE");', '', 'pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");'];
const COST = { claude: 0.42, 'bright-owl': 0.88, 'Bug Fix': 1.12, 'Docs Pruning': 0.31, 'TUIOS Research': 1.84, 'Snipux Bugs': 2.06, 'Memory Hog': 0.67 }, BUDGET = 20;
const NOTE = { 'Docs Pruning': { short: 'done · 1 fails', lines: ['Done: rename landed in 31 files.', 'Still failing: the Caps Lock test,', 'src\\client\\tests.rs:2098.', 'Next: decide npm or pnpm.'] }, 'Bug Fix': { short: 'done · PR ready', lines: ['Fixed the menu bug (#212).', 'Branch bug-fix is pushed;', 'the PR is ready to open. Tests pass.'] } };
const QUEUE = [['bright-owl', 'npm install -g pnpm', 'uses the network'], ['Bug Fix', 'rm -rf target/', 'deletes files'], ['Snipux Bugs', 'git push origin dev', 'pushes to remote']];
const PROMPTS = ['Run the tests and fix what fails', 'Write the PR description', 'Explain this diff', 'Summarize what changed today'];
const HTARGETS = ['Bug Fix', 'Docs Pruning', 'Snipux Bugs', 'Memory Hog', 'claude'];
const CPS = [['◆', 'before fixing the failing test', '2m', 'auto · 3 files'], ['◆', 'before rename seshi → maitty', '2h', 'auto · 31 files'], ['◇', 'splash done', '1d', 'saved by you']];
const PAL_ACTIONS = ['Run tests', 'Commit…', 'Push', 'Open PR…', 'New terminal', 'Toggle file drawer', 'Toggle terminal dock', 'Save a checkpoint'];
function palItems(s) { const q = (s.q || '').toLowerCase(), out = [];
  [['PINNED PROMPTS', PROMPTS.map(n => ({ n, k: 'prompt' }))], ['SESSIONS', ['TUIOS Research', 'Bug Fix', 'Docs Pruning', 'bright-owl', 'Snipux Bugs', 'Memory Hog'].map(n => ({ n, k: 'session' }))],
   ['FILES', TREE.map((n, i) => ({ n: n.n, k: 'file', i, sub: pathOf(i) })).filter(x => !TREE[x.i].dir)],
   ['TERMINALS', TERMS.map((t, i) => ({ n: t.n, k: 'term', i, sub: t.cmd }))],
   ['ACTIONS', PAL_ACTIONS.map(n => ({ n, k: 'action' }))]].forEach(([g, list]) => { const m = list.filter(x => x.n.toLowerCase().includes(q)); if (m.length) out.push({ g, items: m.slice(0, 4) }); });
  return out; }
const CHANGED = { 'brand.rs': [9, 10], 'draw.rs': [7, 8, 9, 10, 13, 14, 15], 'tests.rs': [4, 5, 12], 'card.rs': [5, 6], 'leader.rs': [6], 'popups.rs': [5, 6, 7] };
const fileLines = n => FILES[n] || ['use crate::screen::{Frame, Rect};', '', '/// ' + n.replace(/\.rs$/, '') + ' for the client.', 'pub fn draw(f: &mut Frame, area: Rect) {', '    let inner = area.inner(1);', '    f.clear(inner);', '    f.text(inner.x, inner.y, "' + n + '", Style::default());', '}'];
const KWS = new Set(['use', 'pub', 'fn', 'let', 'if', 'else', 'match', 'mut', 'crate', 'for', 'in', 'return', 'true', 'false', 'self', 'struct', 'impl', 'mod']);
function code(l) { const out = [], re = /(\/\/.*$|"[^"]*"|[A-Za-z_]\w*|\d+|\s+|.)/g; let m; while ((m = re.exec(l))) { const t = m[0]; let fg = T.text, b = 0, i = 0; if (t.startsWith('//') || t.startsWith('#')) { fg = T.dim; i = t.startsWith('//') ? 1 : 0; } else if (t[0] === '"') fg = T.a.green; else if (KWS.has(t)) { fg = T.a.magenta; b = 1; } else if (/^[A-Z]/.test(t)) fg = T.a.yellow; else if (/^\d/.test(t)) fg = T.a.blue; out.push({ t, fg, b, i }); } return out; }
function openness(s) { const tgt = s.dOpen ? 1 : 0; if (!s.motion || s.dT0 == null) return tgt; const p = Math.min(1, Math.max(0, (s.f - s.dT0) / 9)), e = 1 - Math.pow(1 - p, 3), fr = s.dFrom ?? 0; return fr + (tgt - fr) * e; }

function drawDrawer(s, DW, H, boxed, round, pills, lit) {
  const g = new Grid(DW, H, T), R = [], tab = s.dTab || 'files', sel = (s.dSel || {})[tab] ?? 0;
  if (boxed) box(g, 0, 0, DW, H, lit ? T.acc : T.line, round); else g.vl(0, 0, H, '│', { fg: lit ? T.acc : T.line, b: lit });
  const hot = s.edgeHot || s.dDrag, gy = Math.floor(H / 2); for (let k = -2; k <= 2; k++) g.set(0, gy + k, hot ? '┃' : '╏', { fg: hot ? T.acc : T.dim, b: 1 });
  const X = 2, XW = DW - 4, RX = X + XW;
  let tx = X;
  DTABS.forEach(([k, n]) => { const on = k === tab, lab = n + (k === 'changes' ? ' ' + CHG.length : ''); const segs = on ? pill([{ t: ' ' + lab + ' ', fg: lit ? T.ink : T.strong, b: 1 }], lit ? T.acc : T.btn, pills) : [{ t: ' ' + lab + ' ', fg: T.dim }]; const x1 = tx; tx = g.putSegs(tx, 0, segs) + 1; R.push({ x: x1, y: 0, w: tx - x1, h: 1, act: { t: 'drawer', tab: k, set: 1 } }); });
  g.putSegs(DW - 5, 0, [{ t: ' ✕ ', fg: T.dim }]); R.push({ x: DW - 5, y: 0, w: 3, h: 1, act: { t: 'drawer', tab: null } });
  const yb = H - (boxed ? 2 : 1), bodyEnd = yb - 2;
  { const pth = '~/dev/maitty-worktrees/grey-wolf', room = RX - 14 - X; g.putSegs(X, 1, [{ t: L(pth) > room ? '…' + pth.slice(L(pth) - room + 1) : pth, fg: T.dim }], {}, RX - 14); }
  const br = '⎇ grey-wolf'; g.put(RX - L(br), 1, br, { fg: T.a.green });
  let y = 3;
  const rowSel = (yy, on) => { if (!on) return; g.fill(1, yy, DW - 2, 1, T.tint); g.set(1, yy, '▎', { fg: lit ? T.acc : T.dim, bg: T.tint, b: 1 }); };
  if (tab === 'files') {
    const split = s.dFile != null && DW >= 60, TW = split ? Math.max(22, Math.min(30, Math.round(DW * 0.38))) : XW;
    const tree = (x0, xr) => { let yy = y; visibleTree(s).forEach(i => { const n = TREE[i]; if (yy > bodyEnd - 1) return; const on = i === sel, open = i === s.dFile, bg = on ? T.tint : undefined;
        if (on) { g.fill(1, yy, xr, 1, T.tint); g.set(1, yy, '▎', { fg: lit ? T.acc : T.dim, bg: T.tint, b: 1 }); }
        const mk = [...(n.ed ? [{ t: '● ', fg: T.acc, bg }] : []), ...(n.m ? [{ t: n.m, fg: n.m === 'A' ? T.a.green : T.a.yellow, b: 1, bg }] : [])], mw = mk.reduce((q, z) => q + L(z.t), 0);
        g.putSegs(x0 + n.d * 2, yy, n.dir ? [{ t: isClosed(s, i) ? '▸ ' : '▾ ', fg: T.brass, bg }, { t: n.n, fg: T.strong, b: 1, bg }] : [{ t: (open ? '› ' : '  ') + n.n, fg: open ? T.acc : on ? T.strong : T.text, b: on || open, bg }], {}, xr - mw - 1);
        if (mw) g.putSegs(xr - mw, yy, mk);
        R.push({ x: 1, y: yy, w: xr, h: 1, act: { t: 'dsel', tab, i } }); yy++; }); };
    const preview = (x0, xr, back) => { let yy = y; const n = TREE[s.dFile], path = pathOf(s.dFile), lines = fileLines(n.n), ch = n.m === 'A' ? lines.map((_, i) => i + 1) : (CHANGED[n.n] || []);
      const meta = (n.m ? (n.m === 'A' ? 'new · ' : 'changed · ') : '') + lines.length + ' lines';
      g.putSegs(x0, yy, [...(back ? [{ t: '‹ ', fg: T.acc, b: 1 }] : []), { t: back ? path : n.n, fg: T.strong, b: 1 }], {}, xr - L(meta) - 2); g.put(xr - L(meta), yy, meta, { fg: n.m === 'A' ? T.a.green : n.m ? T.a.yellow : T.dim });
      if (back) R.push({ x: 1, y: yy, w: Math.min(DW - 2, L(path) + 4), h: 1, act: { t: 'dback' } }); yy += 2;
      lines.forEach((l, i) => { if (yy > bodyEnd + 1) return; const c = ch.includes(i + 1), hl = s.dLine === i + 1 ? mix(T.bg, T.acc, 0.22) : undefined; if (hl) g.fill(x0 - 1, yy, xr - x0 + 2, 1, hl); g.putSegs(x0, yy, [{ t: String(i + 1).padStart(3) + ' ', fg: hl ? T.strong : c ? T.text : T.dim, b: !!hl }, { t: c ? '▎' : ' ', fg: n.m === 'A' ? T.a.green : T.a.yellow, b: 1 }, { t: ' ' }, ...code(l)].map(q => hl ? { ...q, bg: hl } : q), {}, xr); yy++; }); };
    if (split) { tree(X, X + TW); g.vl(X + TW + 1, y - 1, bodyEnd - y + 4, '│', { fg: T.line }); preview(X + TW + 3, RX, false); }
    else if (s.dFile != null) preview(X, RX, true);
    else { tree(X, RX); g.putSegs(X, bodyEnd + 1, [{ t: 'M', fg: T.a.yellow, b: 1 }, { t: ' changed   ', fg: T.dim }, { t: 'A', fg: T.a.green, b: 1 }, { t: ' new   ', fg: T.dim }, { t: '●', fg: T.acc }, { t: ' an agent is editing', fg: T.dim }], {}, RX); }
  } else if (tab === 'changes') {
    const ta = CHG.reduce((a, c) => a + c.a, 0), tr = CHG.reduce((a, c) => a + c.r, 0), tot = [{ t: '+' + ta, fg: T.a.green, b: 1 }, { t: ' −' + tr, fg: T.a.red, b: 1 }];
    g.putSegs(X, y, [{ t: CHG.length + ' files changed', fg: T.strong, b: 1 }, { t: '  not committed', fg: T.dim }]); g.putSegs(RX - tot.reduce((a, q) => a + L(q.t), 0), y, tot); y += 2;
    CHG.forEach((c, i) => { const on = i === sel, bg = on ? T.tint : undefined; rowSel(y, on);
      const right = [...(c.ed ? [{ t: '● ', fg: T.acc, bg }] : []), { t: '+' + c.a, fg: T.a.green, bg }, { t: ' −' + c.r, fg: T.a.red, bg }], rw = right.reduce((a, q) => a + L(q.t), 0);
      g.putSegs(X, y, [{ t: (c.n ? 'A' : 'M') + ' ', fg: c.n ? T.a.green : T.a.yellow, b: 1, bg }, { t: c.f, fg: on ? T.strong : T.text, b: on, bg }, { t: '  ' + c.dir, fg: T.dim, bg }], {}, RX - rw - 1); g.putSegs(RX - rw, y, right);
      R.push({ x: 1, y, w: DW - 2, h: 1, act: { t: 'dsel', tab, i } }); y++; });
    y++; g.hl(X, y, XW, '─', { fg: T.line }); const c = CHG[sel] || CHG[0]; g.putSegs(X + 1, y, [{ t: ' ' + c.dir + '/' + c.f + ' ', fg: T.strong, b: 1 }]); y += 2;
    hunkFor(c).forEach(l => { if (y > bodyEnd + 1) return; const k = l[0]; g.putSegs(X, y, [{ t: l, fg: l.startsWith('@@') ? T.a.blue : k === '+' ? T.a.green : k === '-' ? T.a.red : T.dim, b: k === '+' || k === '-' }], {}, RX); y++; });
  } else {
    g.putSegs(X, y, [{ t: '⎇ grey-wolf', fg: T.strong, b: 1 }, { t: '  off ', fg: T.dim }, { t: 'dev', fg: T.text }]); y++;
    g.putSegs(X, y, [{ t: '↑3', fg: T.a.green, b: 1 }, { t: ' ahead   ', fg: T.dim }, { t: '↓0', fg: T.text, b: 1 }, { t: ' behind   ', fg: T.dim }, { t: '6', fg: T.a.yellow, b: 1 }, { t: ' uncommitted', fg: T.dim }]); y += 2;
    g.put(X, y++, 'COMMITS', { fg: T.dim, b: 1 });
    COMMITS.forEach(([hh, m, t], i) => { const on = i === sel, bg = on ? T.tint : undefined; rowSel(y, on); g.putSegs(X, y, [{ t: hh + ' ', fg: T.brass, bg }, { t: m, fg: on ? T.strong : T.text, b: on, bg }], {}, RX - L(t) - 1); g.put(RX - L(t), y, t, { fg: T.dim, bg }); R.push({ x: 1, y, w: DW - 2, h: 1, act: { t: 'dsel', tab, i } }); y++; });
    y++; g.put(X, y++, 'PULL REQUEST', { fg: T.dim, b: 1 }); g.put(X, y, 'none yet · push first', { fg: T.text }); y += 2;
    g.put(X, y++, 'CHECK', { fg: T.dim, b: 1 }); g.putSegs(X, y, [{ t: '✓ ', fg: T.ok, b: 1 }, { t: 'cargo test', fg: T.text }, { t: '  passed 14:22', fg: T.dim }]); y += 2;
    g.putSegs(X, y++, [{ t: 'CHECKPOINTS', fg: T.dim, b: 1 }, { t: '  saved before big edits', fg: T.dim }]);
    CPS.forEach(([gl, m, t, w2], i) => { const on = s.cpAsk === i, bg = on ? mix(T.bg, T.needs, 0.14) : undefined; if (on) g.fill(1, y, DW - 2, 1, bg);
      g.putSegs(X, y, [{ t: gl + ' ', fg: gl === '◆' ? T.acc : T.brass, b: 1, bg }, { t: m, fg: T.text, bg }, { t: '  ' + w2, fg: T.dim, bg }], {}, RX - L(t) - 1); g.put(RX - L(t), y, t, { fg: T.dim, bg });
      R.push({ x: 1, y, w: DW - 2, h: 1, act: { t: 'cp', i } }); y++;
      if (on) { g.putSegs(X + 2, y++, [{ t: 'Restore grey-wolf to this point?', fg: T.needs, b: 1 }], {}, RX); g.putSegs(X + 2, y++, [{ t: 'Later changes go to a stash.', fg: T.needs }], {}, RX); g.putSegs(X + 2, y++, [{ t: ' y ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' restore   ', fg: T.dim }, { t: ' n ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' cancel', fg: T.dim }]); } });
    if (s.cpDone != null) { g.putSegs(X, y + 1, [{ t: '✓ restored to "' + trunc(CPS[s.cpDone][1], XW - 16) + '"', fg: T.ok, b: 1 }], {}, RX); g.putSegs(X + 2, y + 2, [{ t: 'stash@{0} keeps the later changes', fg: T.dim }], {}, RX); }
  }
  g.fill(1, yb, DW - 2, 1, T.panel);
  const acts = tab === 'files' ? (s.dFile != null && DW >= 60 ? [['↑↓', 'file'], ['Enter', 'open'], ['i', 'insert path'], ['e', 'editor']] : s.dFile != null ? [['Esc', 'back'], ['i', 'insert path'], ['e', 'editor'], ['d', 'diff']] : [['Enter', 'open'], ['i', 'insert path'], ['e', 'editor'], ['d', 'diff']]) : tab === 'changes' ? [['↑↓', 'file'], ['Enter', 'open file'], ['e', 'editor'], ['y', 'copy']] : [['c', 'commit…'], ['p', 'push'], ['r', 'PR…'], ['k', 'checkpoint']];
  let ax = X; acts.forEach(([k, l]) => { if (ax + L(k) + L(l) + 4 > RX) return; ax = g.putSegs(ax, yb, [{ t: ' ' + k + ' ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' ' + l, fg: T.dim, bg: T.panel }]) + 2; });
  return { g, R };
}
