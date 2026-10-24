// Maitty — All agents on deck. Splash screen. SAMPLE DATA ONLY.
import { Grid, L, mix } from './seshi-grid.js';

const T = { name: 'dark', bg: '#08111d', fg: '#c9d6e3', card: '#0f1b2b', card2: '#15253a', btn: '#1a2c44', line: '#1d2d42', dim: '#6f8399', text: '#c9d6e3', strong: '#eef5fb', acc: '#3dd6c6', ink: '#08111d', sea: '#1f6f8b', deep: '#123049', foam: '#e8f1f8', brass: '#e9b44c', scrim: '#000000' };
const FONT = { M: ['1...1', '11.11', '1.1.1', '1...1', '1...1'], A: ['.11.', '1..1', '1111', '1..1', '1..1'], I: ['111', '.1.', '.1.', '.1.', '111'], T: ['11111', '..1..', '..1..', '..1..', '..1..'], Y: ['1...1', '.1.1.', '..1..', '..1..', '..1..'] };
const TALL = { M: ['1...1', '11.11', '1.1.1', '1.1.1', '1...1', '1...1', '1...1'], A: ['.111.', '1...1', '1...1', '11111', '1...1', '1...1', '1...1'], I: ['111', '.1.', '.1.', '.1.', '.1.', '.1.', '111'], T: ['11111', '..1..', '..1..', '..1..', '..1..', '..1..', '..1..'], Y: ['1...1', '1...1', '.1.1.', '..1..', '..1..', '..1..', '..1..'] };
export const TITLES = { classic: 'Classic', ai: 'AI on deck', tall: 'Tall' };
export const LETTERS = { block: 'Block', shadow: 'Shadow', rope: 'Rope', tide: 'Tide' };
const ROPE = { '0101': '╭', '1001': '╮', '0110': '╰', '1010': '╯', '1100': '─', '0011': '│', '1101': '┬', '1110': '┴', '0111': '├', '1011': '┤', '1111': '┼', '1000': '─', '0100': '─', '0010': '│', '0001': '│', '0000': '•' };
const WAVE = '▁▂▃▄▅▄▃▂';

export function initial(W = 120, H = 34) { return { W, H, f: 0, motion: true, sel: 0, t0: 0, title: 'ai', letters: 'block' }; }
export function render(s) {
  const W = s.W, H = s.H, g = new Grid(W, H, T), R = [], t = s.motion ? s.f - s.t0 : 999;
  const px = W >= 110 ? 2 : 1, word = 'MAITTY', gap = px, F = s.title === 'tall' && H >= 30 ? TALL : FONT, hi = s.title === 'classic' ? [4, 5] : [1, 2];
  const ww = [...word].reduce((n, c) => n + F[c][0].length * px + gap, -gap);
  const compact = H < 30, block = F === TALL ? 7 : 5, total = block + 2 + 1 + 2 + 3 * 2 + 1 + 2 + 1;
  let y = Math.max(1, Math.floor((H - total) / 2) - (compact ? 0 : 1));
  // wordmark: each letter fades in from the sea colour to foam, left to right (colour only, nothing moves)
  let x = Math.floor((W - ww) / 2);
  [...word].forEach((c, k) => {
    const a = Math.max(0, Math.min(1, (t - k * 3) / 8)), e = 1 - Math.pow(1 - a, 3), col = mix(T.deep, hi.includes(k) ? T.acc : T.foam, e);
    const P = F[c], on = (rx, ry) => ry >= 0 && ry < P.length && rx >= 0 && rx < P[0].length && P[ry][rx] === '1', st = s.letters || 'block';
    P.forEach((r, ry) => [...r].forEach((b, rx) => {
      if (b !== '1') return;
      const X = x + rx * px, Y = y + ry;
      if (st === 'block') g.fill(X, Y, px, 1, col);
      else if (st === 'shadow') { g.fill(X, Y, px, 1, col); if (!on(rx + 1, ry + 1)) g.fill(X + px, Y + 1, px > 1 ? 1 : 1, 1, mix(T.bg, T.deep, 0.9 * e)); if (!on(rx, ry + 1)) g.fill(X + 1, Y + 1, px, 1, mix(T.bg, T.deep, 0.9 * e)); }
      else if (st === 'rope') { const key = [on(rx - 1, ry), on(rx + 1, ry), on(rx, ry - 1), on(rx, ry + 1)].map(v => v ? 1 : 0).join(''); g.set(X, Y, ROPE[key], { fg: col, b: 1 }); if (px > 1) g.set(X + 1, Y, on(rx + 1, ry) ? '─' : ' ', { fg: col, b: 1 }); }
      else { const lvl = P.length - 1 - (s.motion ? (Math.sin(s.f / 9 + k * 0.6) + 1) : 1), sub = ry > lvl; g.fill(X, Y, px, 1, mix(T.deep, sub ? mix(T.acc, T.sea, (ry - lvl) / P.length) : (hi.includes(k) ? T.acc : T.foam), e)); }
    }));
    x += F[c][0].length * px + gap;
  });
  y += block + 1;
  // the sea: one quiet rolling line under the name
  const sx = Math.floor((W - ww) / 2) - 4, sw = ww + 8;
  for (let k = 0; k < sw; k++) { const ph = Math.floor((k + (s.motion ? s.f / 3 : 0)) % WAVE.length), edge = Math.min(k, sw - 1 - k) / 6; g.set(sx + k, y, WAVE[ph], { fg: mix(T.bg, T.sea, Math.min(1, edge) * Math.min(1, t / 14)) }); }
  y += 2;
  const tag = [{ t: 'All agents on deck.', fg: mix(T.bg, T.text, Math.min(1, Math.max(0, (t - 10) / 10))), i: 1 }];
  g.putSegs(Math.floor((W - L(tag[0].t)) / 2), y, tag); y += compact ? 2 : 3;
  // choices: drawn from frame 0, usable immediately
  const mw = Math.min(56, W - 8), mx = Math.floor((W - mw) / 2);
  [['Resume where you left off', 'r', 'seshi › TUIOS Research'], ['New session', 'n', null], ['Open a project…', 'o', null]].forEach(([label, k, d], i) => {
    const on = s.sel === i, rows = d && !compact ? 2 : 1, bg = on ? T.card2 : T.bg;
    g.fill(mx, y, mw, rows, bg); if (on) for (let j = 0; j < rows; j++) g.set(mx, y + j, '▌', { fg: T.acc, bg });
    g.putSegs(mx + 2, y, [{ t: label, fg: on ? T.strong : T.text, b: on, bg }]);
    g.putSegs(mx + mw - 4, y, [{ t: ' ' + k + ' ', fg: on ? T.ink : T.strong, bg: on ? T.acc : T.btn, b: 1 }]);
    if (rows === 2) g.put(mx + 2, y + 1, d, { fg: T.dim, bg });
    R.push({ x: mx, y, w: mw, h: rows, act: { t: 'sel', i } }); y += rows + (compact ? 0 : 1);
  });
  y += 1;
  const hs = [['↑↓', 'choose'], ['Enter', 'open'], ['?', 'keys']].flatMap(([k, l], j) => [...(j ? [{ t: '   ' }] : []), { t: ' ' + k + ' ', fg: T.strong, bg: T.btn, b: 1 }, { t: ' ' + l, fg: T.dim }]);
  g.putSegs(Math.floor((W - hs.reduce((n, q) => n + L(q.t), 0)) / 2), y, hs);
  const ver = 'maitty 0.9.0'; g.put(W - L(ver) - 2, H - 1, ver, { fg: T.line });
  return { rows: g.rows(), regions: R, W, H, bg: T.bg };
}
export function act(s, a) {
  const P = p => ({ ...s, ...p });
  if (a.t === 'sel') return P({ sel: a.i }); if (a.t !== 'key') return null;
  const k = a.key;
  if (k === 'ArrowDown' || k === 'j') return P({ sel: Math.min(2, s.sel + 1) }); if (k === 'ArrowUp' || k === 'k') return P({ sel: Math.max(0, s.sel - 1) });
  const m = { r: 0, n: 1, o: 2 }[k]; if (m != null) return P({ sel: m });
  return null;
}
