// Splits render-frames.mjs's output into one file per preset here ("04.txt", "A1.txt"), each
// block headed "===== <layout> | <W>x<H> | <glyphs>", trailing spaces trimmed.
// Usage, from the repo root: node src/client/v7_frames/split-frames.mjs frames.txt
import fs from 'node:fs';
const [,, input] = process.argv;
const out = new Map();
let file = null;
for (const line of fs.readFileSync(input, 'utf8').split('\n')) {
  if (line.startsWith('===== ')) {
    const [preset, layout, size, glyphs] = line.slice(6).split(' | ');
    file = preset.slice(0, 2);
    out.set(file, (out.get(file) || '') + `===== ${layout} | ${size} | ${glyphs}\n`);
  } else if (file) {
    out.set(file, out.get(file) + line.trimEnd() + '\n');
  }
}
const dir = new URL('.', import.meta.url);
for (const [name, text] of out) fs.writeFileSync(new URL(name + '.txt', dir), text.replace(/\n+$/, '\n'));
console.log('wrote', out.size, 'files');
