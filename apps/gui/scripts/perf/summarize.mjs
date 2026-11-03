// Tabulates results/*.json into one text table plus experiment deltas.
import fs from 'node:fs'
import path from 'node:path'
const OUT = path.join(path.dirname(new URL(import.meta.url).pathname), 'results')
const rows = []
for (const f of fs.readdirSync(OUT).filter((f) => f.endsWith('.json') && !f.endsWith('.trace.json'))) {
  const r = JSON.parse(fs.readFileSync(path.join(OUT, f), 'utf8'))
  if (r.label) rows.push(r)
}
rows.sort((a, b) => a.label.localeCompare(b.label))
const pad = (s, n) => String(s).padEnd(n)
const num = (v, n = 6) => String(v ?? '').padStart(n)
console.log(pad('label', 28), num('busy%'), num('taskB%'), num('prof%'), num('script'), num('style'), num('layout'), num('paint'), num('comp'), num('other'), num('gc'), num('draw/s'), num('main/s'), num('lay/s'), num('sty/s'), num('tasks/s'), num('long'), num('tmo/s'), num('int/s'), num('raf/s'), num('heapΔ'), num('nodesΔ'))
for (const r of rows) {
  const t = r.trace || {}
  const c = t.categoriesMsPerSec || {}
  const tm = r.page?.timersPerSec || {}
  console.log(pad(r.label, 28), num(t.busyPct), num(r.metricsDelta.TaskBusyPct), num(r.profile?.busyPct), num(c.script), num(c.style), num(c.layout), num(c.paint), num(c.composite), num(c.other), num(c.gc), num(t.drawFramesPerSec), num(t.mainFramesPerSec), num(t.layoutPerSec), num(t.stylePerSec), num(t.tasksPerSec), num(t.longTasks), num(tm.timeoutFired?.toFixed(1)), num(tm.intervalFired?.toFixed(1)), num(tm.rafFired?.toFixed(1)), num((r.metricsDelta.HeapMBAfter - r.metricsDelta.HeapMBBefore).toFixed(1)), num(r.metricsDelta.NodesAfter - r.metricsDelta.NodesBefore))
}
console.log('\n== experiment deltas (busy% vs same scenario/gpu baseline) ==')
const base = new Map(rows.filter((r) => r.label === `${r.scenario}-${r.gpu}`).map((r) => [`${r.scenario}-${r.gpu}`, r]))
for (const r of rows.filter((r) => r.experiment !== 'none')) {
  const b = base.get(`${r.scenario}-${r.gpu}`)
  if (!b || !r.trace || !b.trace) continue
  const d = (k) => ((r.trace.categoriesMsPerSec[k] || 0) - (b.trace.categoriesMsPerSec[k] || 0)).toFixed(1)
  console.log(pad(r.label, 30), 'busy', num(b.trace.busyPct), '->', num(r.trace.busyPct), 'Δ', num((r.trace.busyPct - b.trace.busyPct).toFixed(1)), ' script', num(d('script')), 'style', num(d('style')), 'layout', num(d('layout')), 'paint', num(d('paint')), 'comp', num(d('composite')), ' taskB', num(b.metricsDelta.TaskBusyPct), '->', num(r.metricsDelta.TaskBusyPct))
}
console.log('\n== poll samples ==')
for (const r of rows.filter((r) => r.samples?.length)) { console.log(r.label); console.table(r.samples) }
console.log('\n== top functions per baseline ==')
for (const r of rows.filter((r) => r.experiment === 'none' && r.profile && r.dsf === 1)) {
  console.log(`-- ${r.label} (profile busy ${r.profile.busyPct}%)`)
  for (const f of r.profile.topFunctions.slice(0, 22)) console.log(`   ${num(f.ms, 8)}ms ${num(f.pct, 5)}%  ${f.key}`)
  console.log('   files:', r.profile.topFiles.slice(0, 10).map((f) => `${f.key}=${f.ms}ms`).join(' | '))
  console.log('   trace top events:', r.trace.topEventNames.slice(0, 14).map(([n, v]) => `${n}=${v}`).join(' | '))
  console.log('   dom:', JSON.stringify({ ...r.dom, animated: r.dom.animated, animatedPseudo: r.dom.animatedPseudo }))
  console.log('   intervals:', JSON.stringify(r.page.intervalFiresPerSec), 'events/s', r.page.eventsPerSec.toFixed(1))
}
