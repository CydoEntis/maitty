// Lists the animations running in a scenario and measures what each one costs by cancelling it.
// Usage: node inspect-anims.mjs --scenario a [--gpu cpu] [--measure 8]
import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { chromium } from '/home/vox/.local/share/mise/installs/npm-playwright/latest/node_modules/playwright/index.mjs'

const HERE = path.dirname(new URL(import.meta.url).pathname)
const DIST = path.join(HERE, 'dist')
const CHROME = '/home/vox/.cache/ms-playwright/chromium-1223/chrome-linux64/chrome'
const args = Object.fromEntries(process.argv.slice(2).reduce((acc, a, i, arr) => { if (a.startsWith('--')) acc.push([a.slice(2), arr[i + 1] && !arr[i + 1].startsWith('--') ? arr[i + 1] : 'true']); return acc }, []))
const scenario = args.scenario || 'a'
const gpu = args.gpu || 'gpu'
const measureSec = Number(args.measure || 8)
const SESSIONS = {
  a: [{ id: 1, status: 'Idle', stream: false, agent: null }],
  b: [{ id: 1, status: 'Working', stream: true }],
  e: [{ id: 1, status: 'Working', stream: true }, { id: 2, status: 'Working', stream: false }, { id: 3, status: 'Idle', stream: false, agent: null }, { id: 4, status: 'Blocked', stream: false }],
}
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.map': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2' }
const server = http.createServer((req, res) => {
  let file = path.join(DIST, decodeURIComponent(new URL(req.url, 'http://x').pathname))
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(DIST, 'index.html')
  res.setHeader('Content-Type', MIME[path.extname(file)] || 'application/octet-stream')
  fs.createReadStream(file).pipe(res)
})
await new Promise((r) => server.listen(0, '127.0.0.1', r))
const base = `http://127.0.0.1:${server.address().port}`
const flags = ['--disable-dev-shm-usage', '--disable-background-timer-throttling', '--disable-renderer-backgrounding']
if (gpu === 'cpu') flags.push('--disable-gpu', '--disable-gpu-compositing', '--disable-software-rasterizer')
const browser = await chromium.launch({ headless: true, executablePath: CHROME, args: flags })
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark' })
const page = await context.newPage()
const layout = { tabs: [{ id: 'tab-1', panes: [{ id: 'pane-1', sessionId: 't1', col: 0 }], focus: 'pane-1', max: null }], activeTabId: 'tab-1' }
await page.addInitScript(({ sessions, layoutJson }) => {
  window.__SCENARIO = { sessions, bytesPerSec: 20000, tickMs: 50 }
  localStorage.setItem('maitty.gui.welcomed.v1', '1')
  localStorage.setItem('maitty.gui.settings.v1', '{}')
  localStorage.setItem('maitty.gui.layout.v1', layoutJson)
}, { sessions: SESSIONS[scenario], layoutJson: JSON.stringify(layout) })
await page.addInitScript({ path: path.join(HERE, 'mock-tauri.js') })
const cdp = await context.newCDPSession(page)
await cdp.send('Performance.enable', { timeDomain: 'timeTicks' })

async function load() {
  await page.goto(`${base}/`, { waitUntil: 'load' })
  await page.waitForSelector('.mx-pane .xterm-rows', { timeout: 20000 })
  await page.waitForFunction(() => !document.querySelector('.terminal-load-status'))
  await page.waitForTimeout(3000)
}
const listAnims = () => page.evaluate(() => document.getAnimations().map((a) => {
  const t = a.effect?.target
  const pseudo = a.effect?.pseudoElement || ''
  const kf = a.effect?.getKeyframes?.() || []
  const props = [...new Set(kf.flatMap((k) => Object.keys(k).filter((p) => !['offset', 'computedOffset', 'easing', 'composite'].includes(p))))]
  const timing = a.effect?.getTiming?.() || {}
  return { name: a.animationName || a.constructor.name, target: t ? `${t.tagName.toLowerCase()}.${[...t.classList].slice(0, 3).join('.')}${pseudo}` : '?', props, duration: timing.duration, iterations: timing.iterations, playState: a.playState }
}))
const asMap = (m) => Object.fromEntries(m.metrics.map((x) => [x.name, x.value]))
async function measure() {
  const b = asMap(await cdp.send('Performance.getMetrics'))
  const frames0 = await page.evaluate(() => window.__perf.timers.rafFired)
  await page.evaluate(() => { window.__rafCount = 0; const tick = () => { window.__rafCount += 1; window.__rafId = requestAnimationFrame(tick) }; window.__rafId = requestAnimationFrame(tick) })
  const t0 = Date.now()
  await page.waitForTimeout(measureSec * 1000)
  const secs = (Date.now() - t0) / 1000
  await page.evaluate(() => cancelAnimationFrame(window.__rafId))
  const a = asMap(await cdp.send('Performance.getMetrics'))
  const d = (k) => a[k] - b[k]
  void frames0
  return { taskBusyPct: +((d('TaskDuration') / secs) * 100).toFixed(2), scriptMsPerSec: +((d('ScriptDuration') / secs) * 1000).toFixed(1), styleMsPerSec: +((d('RecalcStyleDuration') / secs) * 1000).toFixed(1), layoutMsPerSec: +((d('LayoutDuration') / secs) * 1000).toFixed(1), stylesPerSec: +(d('RecalcStyleCount') / secs).toFixed(1), layoutsPerSec: +(d('LayoutCount') / secs).toFixed(1) }
}

await load()
const anims = await listAnims()
console.log(`== ${scenario}-${gpu}: running animations ==`)
for (const a of anims) console.log(`  ${a.name.padEnd(16)} ${String(a.iterations).padStart(8)}x ${String(a.duration).padStart(6)}ms ${a.playState.padEnd(8)} ${a.target.padEnd(50)} ${a.props.join(',')}`)
const baseline = await measure()
console.log('baseline', JSON.stringify(baseline))
const names = [...new Set(anims.map((a) => a.name))]
const results = []
for (const name of names) {
  await load()
  await page.evaluate((n) => { for (const a of document.getAnimations()) if (a.animationName === n) a.cancel() }, name)
  const m = await measure()
  results.push({ cancelled: name, ...m, dBusy: +(m.taskBusyPct - baseline.taskBusyPct).toFixed(2), dStyles: +(m.stylesPerSec - baseline.stylesPerSec).toFixed(1) })
}
await load()
await page.evaluate(() => { for (const a of document.getAnimations()) a.cancel() })
const all = await measure()
results.push({ cancelled: 'ALL', ...all, dBusy: +(all.taskBusyPct - baseline.taskBusyPct).toFixed(2), dStyles: +(all.stylesPerSec - baseline.stylesPerSec).toFixed(1) })
console.table(results)
fs.writeFileSync(path.join(HERE, 'results', `anims-${scenario}-${gpu}.json`), JSON.stringify({ anims, baseline, results }, null, 2))
await browser.close()
server.close()
