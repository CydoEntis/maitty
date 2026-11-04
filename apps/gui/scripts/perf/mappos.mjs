import fs from 'node:fs'
import { TraceMap, originalPositionFor } from '/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/.pnpm/@jridgewell+trace-mapping@0.3.31/node_modules/@jridgewell/trace-mapping/dist/trace-mapping.mjs'
const [file, line, col] = process.argv.slice(2)
const tm = new TraceMap(JSON.parse(fs.readFileSync(`dist/assets/${file}.map`, 'utf8')))
const p = originalPositionFor(tm, { line: +line, column: +col })
console.log(p)
if (p.source && p.source.includes('xterm/lib/xterm.mjs')) {
  const tm2 = new TraceMap(JSON.parse(fs.readFileSync('/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/@xterm/xterm/lib/xterm.mjs.map', 'utf8')))
  console.log(originalPositionFor(tm2, { line: p.line, column: p.column }))
}
