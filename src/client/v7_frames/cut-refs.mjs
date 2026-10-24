// Rebuilds v7_ref.txt and check_modals_frames.txt by cutting the same regions out of render-frames.mjs output. Run through regen.sh.
import fs from 'node:fs';
const [,, framesPath, oldRef, oldModals, outRef, outModals] = process.argv;
const lines = (p) => fs.readFileSync(p, 'utf8').split(String.fromCharCode(13)).join('').split('\n');
const frames = new Map();
let cur = null;
for (const line of lines(framesPath)) {
  if (line.startsWith('===== ')) { cur = []; frames.set(line.slice(6), cur); } else if (cur) cur.push(line);
}
const cut = (rows, x, y, w, h) => rows.slice(y, y + h).map(r => [...r].slice(x, x + w).join('').trimEnd());
let out = '';
for (const l of lines(oldRef)) {
  if (!l.startsWith('===== ')) continue;
  const [preset, layout, size, geo] = l.slice(6).split(' | ');
  const [at, wh] = geo.split(' ');
  const [x, y] = at.split(',').map(Number);
  const [w, h] = wh.split('x').map(Number);
  const rows = frames.get(`${preset} | ${layout} | ${size} | unicode`);
  if (!rows) throw new Error('no frame ' + l);
  out += l + '\n' + cut(rows, x, y, w, h).join('\n') + '\n';
}
fs.writeFileSync(outRef, out);
const rect = (preset, w, h) => {
  let mw, mh;
  if (preset.startsWith('12')) { mw = Math.min(70, w - 2); mh = 15; }
  else if (preset.startsWith('A')) { mw = Math.min(84, w - 2); mh = Math.min(20, h - 1) - (h < 30 ? 1 : 0); }
  else { mw = Math.min(80, w - 2); mh = Math.min(20, h - 2); }
  return [Math.floor((w - mw) / 2), Math.floor((h - mh) / 2), mw, mh];
};
let mo = '';
for (const l of lines(oldModals)) {
  if (!l.startsWith('===== ')) continue;
  const [preset, size] = l.slice(6).split(' | ');
  const [W, H] = size.split('x').map(Number);
  const [x, y, w, h] = rect(preset, W, H);
  const rows = frames.get(`${preset} | floating | ${size} | unicode`);
  if (!rows) throw new Error('no frame ' + l);
  mo += l + '\n' + cut(rows, x + 1, y + 1, w - 2, h - 2).join('\n') + '\n';
}
fs.writeFileSync(outModals, mo);
console.log('ok');
