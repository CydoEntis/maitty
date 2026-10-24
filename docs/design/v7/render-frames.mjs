// From the repo root (Git Bash or any sh): T=$(mktemp -d) && cp docs/design/v7/*.js "$T" && echo '{"type":"module"}' > "$T/package.json" && node docs/design/v7/render-frames.mjs "$T" frames.txt
// Writes every preset x layout x size (plus ASCII glyphs at Flat 80x24) as "===== <preset> | <layout> | <W>x<H> | <glyphs>" blocks of plain text.
// The design names the app with __APP_NAME__ / __APP_COMMAND__, filled in here from the repo's
// Cargo.toml (ADR 0019), so the frames match whatever the app is called.
import { pathToFileURL } from 'node:url';
import fs from 'node:fs';
const [,, dir, out] = process.argv;
const manifest = fs.readFileSync('Cargo.toml', 'utf8');
const command = manifest.match(/^name\s*=\s*"([^"]+)"/m)[1];
const appName = manifest.split('[package.metadata.brand]')[1].match(/^name\s*=\s*"([^"]+)"/m)[1];
const design = dir + '/checks7.js';
fs.writeFileSync(design, fs.readFileSync(design, 'utf8').replaceAll('__APP_NAME__', appName).replaceAll('__APP_COMMAND__', command));
const m = await import(pathToFileURL(design).href);
const sizes = [[160,45],[100,30],[80,24]];
let all = '';
for (const [name] of m.PRESETS) for (const layout of ['floating','tiled','flat']) for (const [W,H] of sizes) for (const glyphs of (layout==='flat' && W===80 ? ['unicode','ascii'] : ['unicode'])) {
  const s = m.preset(name, W, H, { layout, glyphs, motion: false });
  const r = m.render(s);
  const txt = r.rows.map(runs => runs.map(x => x.t).join('')).join('\n');
  all += `===== ${name} | ${layout} | ${W}x${H} | ${glyphs}\n${txt}\n`;
}
fs.writeFileSync(out, all);
console.log('ok', all.length);
