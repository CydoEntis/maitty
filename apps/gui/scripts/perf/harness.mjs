// Runs the built Maitty GUI in headless Chromium with a mocked Tauri host and measures it.
// Usage: node harness.mjs --scenario a|b|c|d --gpu gpu|cpu [--measure 20] [--warm 5] [--exp name]
//        [--poll] (sample heap/nodes every 5 s instead of tracing) [--dsf 2]
import fs from 'node:fs'
import http from 'node:http'
import path from 'node:path'
import { chromium } from '/home/vox/.local/share/mise/installs/npm-playwright/latest/node_modules/playwright/index.mjs'
import { analyzeTrace, analyzeProfile, analyzeInvalidations } from './analyze.mjs'

const HERE = path.dirname(new URL(import.meta.url).pathname)
const DIST = path.join(HERE, 'dist')
const OUT = path.join(HERE, 'results')
fs.mkdirSync(OUT, { recursive: true })
const CHROME = '/home/vox/.cache/ms-playwright/chromium-1223/chrome-linux64/chrome'

const args = Object.fromEntries(process.argv.slice(2).reduce((acc, a, i, arr) => {
  if (a.startsWith('--')) acc.push([a.slice(2), arr[i + 1] && !arr[i + 1].startsWith('--') ? arr[i + 1] : 'true'])
  return acc
}, []))
const scenario = args.scenario || 'a'
const gpu = args.gpu || 'gpu'
const measureSec = Number(args.measure || 20)
const warmSec = Number(args.warm || 5)
const experiment = args.exp || 'none'
const dsf = Number(args.dsf || 1)
const poll = args.poll === 'true'
const stackTrace = args.stacktrace === 'true'
const label = `${scenario}-${gpu}${experiment !== 'none' ? `-${experiment}` : ''}${dsf !== 1 ? `-dsf${dsf}` : ''}${poll ? '-poll' : ''}${args.stacks === 'true' ? '-stacks' : ''}${stackTrace ? '-st' : ''}`

const SCENARIOS = {
  a: { sessions: [{ id: 1, status: 'Idle', stream: false, agent: null }], panes: ['t1'] },
  b: { sessions: [{ id: 1, status: 'Working', stream: true }], panes: ['t1'] },
  c: {
    sessions: [
      { id: 1, status: 'Working', stream: true },
      { id: 2, status: 'Working', stream: true },
      { id: 3, status: 'Idle', stream: false, agent: null },
      { id: 4, status: 'Idle', stream: false },
    ],
    panes: ['t1', 't2', 't3', 't4'],
  },
  d: { sessions: [{ id: 1, status: 'Working', stream: true }], panes: ['t1'], resize: true },
  // 4 sessions listed, one pane shown: isolates sidebar/tab cost from pane cost
  e: {
    sessions: [
      { id: 1, status: 'Working', stream: true },
      { id: 2, status: 'Working', stream: false },
      { id: 3, status: 'Idle', stream: false, agent: null },
      { id: 4, status: 'Blocked', stream: false },
    ],
    panes: ['t1'],
  },
}
const sc = SCENARIOS[scenario]
if (!sc) throw new Error(`unknown scenario ${scenario}`)

// ---- static server ----
const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.map': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.json': 'application/json' }
const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://x')
  let file = path.join(DIST, decodeURIComponent(url.pathname))
  if (!fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(DIST, 'index.html')
  res.setHeader('Content-Type', MIME[path.extname(file)] || 'application/octet-stream')
  res.setHeader('Cache-Control', 'no-cache')
  fs.createReadStream(file).pipe(res)
})
await new Promise((r) => server.listen(0, '127.0.0.1', r))
const base = `http://127.0.0.1:${server.address().port}`

// ---- browser ----
const flags = ['--disable-dev-shm-usage', '--no-first-run', '--disable-background-timer-throttling', '--disable-renderer-backgrounding', '--disable-backgrounding-occluded-windows']
if (gpu === 'cpu') flags.push('--disable-gpu', '--disable-gpu-compositing', '--disable-software-rasterizer', '--disable-accelerated-2d-canvas')
const browser = await chromium.launch({ headless: true, executablePath: CHROME, args: flags })
const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: dsf, colorScheme: 'dark' })
const page = await context.newPage()
const consoleLines = []
page.on('console', (m) => { consoleLines.push(`[${m.type()}] ${m.text().slice(0, 300)}`) })
page.on('pageerror', (e) => { consoleLines.push(`[pageerror] ${e.message}`) })

const layout = {
  tabs: [{ id: 'tab-1', panes: sc.panes.map((sid, i) => ({ id: `pane-${i + 1}`, sessionId: sid, col: sc.panes.length === 4 ? (i < 2 ? 0 : 1) : i })), focus: 'pane-1', max: null }],
  activeTabId: 'tab-1',
}
await page.addInitScript(({ scenarioConfig, layoutJson }) => {
  window.__SCENARIO = scenarioConfig
  try {
    localStorage.setItem('maitty.gui.welcomed.v1', '1')
    localStorage.setItem('maitty.gui.settings.v1', '{}')
    localStorage.setItem('maitty.gui.layout.v1', layoutJson)
  } catch {}
}, { scenarioConfig: { sessions: sc.sessions, bytesPerSec: 20000, tickMs: 50, timerStacks: args.stacks === 'true' }, layoutJson: JSON.stringify(layout) })
await page.addInitScript({ path: path.join(HERE, 'mock-tauri.js') })

const cdp = await context.newCDPSession(page)
await cdp.send('Performance.enable', { timeDomain: 'timeTicks' })
await cdp.send('Profiler.enable')
await cdp.send('Profiler.setSamplingInterval', { interval: 500 })

await page.goto(`${base}/`, { waitUntil: 'load' })
await page.waitForSelector('.mx-pane .xterm-rows', { timeout: 20000 })
await page.waitForFunction(() => !document.querySelector('.terminal-load-status'), null, { timeout: 20000 })

// ---- experiments ----
const EXPERIMENTS = {
  none: async () => {},
  noanim: async () => { await page.addStyleTag({ content: '*, *::before, *::after { animation: none !important; transition: none !important; }' }) },
  nocursorblink: async () => { await page.addStyleTag({ content: '.xterm .xterm-cursor-blink, .xterm .xterm-cursor-blink::before, .xterm-cursor.xterm-cursor-blink { animation: none !important; }' }) },
  hidesidebar: async () => { await page.addStyleTag({ content: '.mx-sidebar { display: none !important; }' }) },
  hidetabbar: async () => { await page.addStyleTag({ content: '.mx-tabbar { display: none !important; }' }) },
  hidestatusbar: async () => { await page.addStyleTag({ content: '.mx-statusbar { display: none !important; }' }) },
  hidepanehead: async () => { await page.addStyleTag({ content: '.mx-pane-head { display: none !important; }' }) },
  hidetitlebar: async () => { await page.addStyleTag({ content: '.title-bar { display: none !important; }' }) },
  hidechrome: async () => { await page.addStyleTag({ content: '.mx-sidebar, .mx-tabbar, .mx-statusbar, .mx-pane-head, .title-bar { display: none !important; }' }) },
  hideterminal: async () => { await page.addStyleTag({ content: '.terminal-view { visibility: hidden !important; }' }) },
  noshadow: async () => { await page.addStyleTag({ content: '* { box-shadow: none !important; filter: none !important; backdrop-filter: none !important; text-shadow: none !important; }' }) },
  nogradient: async () => { await page.addStyleTag({ content: '*, *::before, *::after { background-image: none !important; }' }) },
  flatstyle: async () => { await page.evaluate(() => { localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ style: 'flat', effectWorking: false, effectNeedsYou: false, effectDone: false })) }); await page.reload({ waitUntil: 'load' }); await page.waitForSelector('.mx-pane .xterm-rows') },
  reducemotion: async () => { await page.evaluate(() => { localStorage.setItem('maitty.gui.settings.v1', JSON.stringify({ reduceMotion: true })) }); await page.reload({ waitUntil: 'load' }); await page.waitForSelector('.mx-pane .xterm-rows') },
  stop1s: async () => { await page.evaluate(() => { window.__perf.blockPeriod(1000) }) },
  stop80ms: async () => { await page.evaluate(() => { window.__perf.blockPeriod(80) }) },
  stop1500: async () => { await page.evaluate(() => { window.__perf.blockPeriod(1500) }) },
  stop30s: async () => { await page.evaluate(() => { window.__perf.blockPeriod(30000) }) },
  stopalltimers: async () => { await page.evaluate(() => { for (const ms of [80, 1000, 1500, 8000, 30000]) window.__perf.blockPeriod(ms) }) },
  nostream: async () => { await page.evaluate(() => { window.__mock.listeners.get('terminal-chunk')?.clear() }) },
  // Only the xterm DOM output: hide everything but the terminal screen and stop timers.
  bare: async () => {
    await page.addStyleTag({ content: '.mx-sidebar, .mx-tabbar, .mx-statusbar, .mx-pane-head, .title-bar { display: none !important; } *, *::before, *::after { animation: none !important; transition: none !important; }' })
    await page.evaluate(() => { for (const ms of [80, 1000, 1500, 8000, 30000]) window.__perf.blockPeriod(ms) })
  },
}
if (!EXPERIMENTS[experiment]) throw new Error(`unknown experiment ${experiment}`)
await EXPERIMENTS[experiment]()

await page.waitForTimeout(warmSec * 1000)
await page.evaluate(() => { window.__perf.reset() })

const metricsBefore = await cdp.send('Performance.getMetrics')
const asMap = (m) => Object.fromEntries(m.metrics.map((x) => [x.name, x.value]))
const before = asMap(metricsBefore)

let resizeTimer = null
if (sc.resize) {
  let flip = false
  resizeTimer = setInterval(() => {
    flip = !flip
    page.setViewportSize({ width: flip ? 1200 : 1440, height: flip ? 760 : 900 }).catch(() => {})
  }, 500)
}

const samples = []
let traceResult = null
let profileResult = null
if (poll) {
  const t0 = Date.now()
  while (Date.now() - t0 < measureSec * 1000) {
    const m = asMap(await cdp.send('Performance.getMetrics'))
    const extra = await page.evaluate(() => ({ xtermRows: document.querySelectorAll('.xterm-rows > div').length, spans: document.querySelectorAll('.xterm-rows span').length }))
    samples.push({ t: +((Date.now() - t0) / 1000).toFixed(0), heapMB: +(m.JSHeapUsedSize / 1048576).toFixed(1), nodes: m.Nodes, listeners: m.JSEventListeners, layoutCount: m.LayoutCount, styleCount: m.RecalcStyleCount, ...extra })
    await page.waitForTimeout(5000)
  }
} else {
  await cdp.send('Profiler.start')
  const chunks = []
  const traceDone = new Promise((resolve) => {
    cdp.on('Tracing.tracingComplete', async (ev) => {
      if (ev.stream) {
        for (;;) {
          const r = await cdp.send('IO.read', { handle: ev.stream, size: 1 << 22 })
          chunks.push(r.base64Encoded ? Buffer.from(r.data, 'base64').toString('utf8') : r.data)
          if (r.eof) break
        }
        await cdp.send('IO.close', { handle: ev.stream })
      }
      resolve()
    })
  })
  await cdp.send('Tracing.start', {
    traceConfig: { includedCategories: ['devtools.timeline', 'disabled-by-default-devtools.timeline', 'disabled-by-default-devtools.timeline.frame', 'v8.execute', 'blink.user_timing', 'toplevel', 'cc', 'disabled-by-default-cc.debug.scheduler.frames', ...(stackTrace ? ['disabled-by-default-devtools.timeline.stack', 'disabled-by-default-devtools.timeline.invalidationTracking'] : [])], excludedCategories: ['*'] },
    transferMode: 'ReturnAsStream',
    streamFormat: 'json',
  })
  const t0 = Date.now()
  await page.waitForTimeout(measureSec * 1000)
  const wallMs = Date.now() - t0
  await cdp.send('Tracing.end')
  await traceDone
  const prof = await cdp.send('Profiler.stop')
  const traceText = chunks.join('')
  const parsed = JSON.parse(traceText)
  const events = parsed.traceEvents || parsed
  fs.writeFileSync(path.join(OUT, `${label}.trace.json`), traceText)
  fs.writeFileSync(path.join(OUT, `${label}.cpuprofile`), JSON.stringify(prof.profile))
  traceResult = analyzeTrace(events, wallMs)
  profileResult = analyzeProfile(prof.profile, DIST)
  if (stackTrace && traceResult.mainThread) traceResult.invalidations = analyzeInvalidations(events, DIST, traceResult.mainThread)
}
if (resizeTimer) clearInterval(resizeTimer)

const after = asMap(await cdp.send('Performance.getMetrics'))
const perfSnap = await page.evaluate(() => window.__perf.snapshot())
const dom = await page.evaluate(() => ({
  nodes: document.querySelectorAll('*').length,
  xtermRows: document.querySelectorAll('.xterm-rows > div').length,
  xtermSpans: document.querySelectorAll('.xterm-rows span').length,
  panes: document.querySelectorAll('.mx-pane').length,
  animated: [...document.querySelectorAll('*')].filter((el) => { const cs = getComputedStyle(el); return cs.animationName !== 'none' && cs.animationDuration !== '0s' }).map((el) => `${el.tagName.toLowerCase()}.${[...el.classList].slice(0, 3).join('.')}:${getComputedStyle(el).animationName}`),
  animatedPseudo: [...document.querySelectorAll('*')].filter((el) => ['::before', '::after'].some((p) => { const cs = getComputedStyle(el, p); return cs.animationName !== 'none' && cs.animationDuration !== '0s' })).map((el) => `${el.tagName.toLowerCase()}.${[...el.classList].slice(0, 3).join('.')}`),
  backdrop: [...document.querySelectorAll('*')].filter((el) => getComputedStyle(el).backdropFilter !== 'none').length,
  filters: [...document.querySelectorAll('*')].filter((el) => getComputedStyle(el).filter !== 'none').length,
  shadows: [...document.querySelectorAll('*')].filter((el) => getComputedStyle(el).boxShadow !== 'none').length,
  gradients: [...document.querySelectorAll('*')].filter((el) => getComputedStyle(el).backgroundImage.includes('gradient')).length,
  pseudoGradients: [...document.querySelectorAll('*')].filter((el) => ['::before', '::after'].some((p) => getComputedStyle(el, p).backgroundImage.includes('gradient'))).length,
  willChange: [...document.querySelectorAll('*')].filter((el) => getComputedStyle(el).willChange !== 'auto').length,
  cursorBlinkEls: document.querySelectorAll('.xterm-cursor-blink').length,
}))
await page.screenshot({ path: path.join(OUT, `${label}.png`) })

const delta = (k) => +(after[k] - before[k]).toFixed(3)
const result = {
  label, scenario, gpu, experiment, dsf, measureSec, warmSec,
  metricsDelta: {
    TaskDurationSec: delta('TaskDuration'), ScriptDurationSec: delta('ScriptDuration'), LayoutDurationSec: delta('LayoutDuration'), RecalcStyleDurationSec: delta('RecalcStyleDuration'),
    LayoutCount: delta('LayoutCount'), RecalcStyleCount: delta('RecalcStyleCount'), HeapMBBefore: +(before.JSHeapUsedSize / 1048576).toFixed(1), HeapMBAfter: +(after.JSHeapUsedSize / 1048576).toFixed(1),
    NodesBefore: before.Nodes, NodesAfter: after.Nodes, ListenersBefore: before.JSEventListeners, ListenersAfter: after.JSEventListeners,
    TaskBusyPct: +((delta('TaskDuration') / measureSec) * 100).toFixed(1),
  },
  trace: traceResult,
  profile: profileResult,
  page: perfSnap,
  dom,
  samples,
  console: consoleLines.slice(0, 40),
}
fs.writeFileSync(path.join(OUT, `${label}.json`), JSON.stringify(result, null, 2))
const t = traceResult
console.log(`== ${label} ==`)
console.log(`metrics: task busy ${result.metricsDelta.TaskBusyPct}%  script ${result.metricsDelta.ScriptDurationSec}s layout ${result.metricsDelta.LayoutDurationSec}s (${result.metricsDelta.LayoutCount}) style ${result.metricsDelta.RecalcStyleDurationSec}s (${result.metricsDelta.RecalcStyleCount}) heap ${result.metricsDelta.HeapMBBefore}->${result.metricsDelta.HeapMBAfter}MB nodes ${before.Nodes}->${after.Nodes}`)
if (t) console.log(`trace: busy ${t.busyPct}% cats(ms/s) ${JSON.stringify(t.categoriesMsPerSec)} tasks/s ${t.tasksPerSec} long ${t.longTasks} frames/s draw ${t.drawFramesPerSec} main ${t.mainFramesPerSec} raster ${t.rasterMsPerSec}ms/s compositor ${t.compositorThreadMsPerSec}ms/s layouts/s ${t.layoutPerSec} styles/s ${t.stylePerSec} paints/s ${t.paintPerSec}`)
console.log(`timers/s: ${JSON.stringify(perfSnap.timersPerSec)} events/s ${perfSnap.eventsPerSec.toFixed(1)} (${perfSnap.eventKBPerSec.toFixed(1)} KB/s) invokes ${JSON.stringify(perfSnap.invokes)}`)
console.log(`intervals: ${JSON.stringify(perfSnap.intervalFiresPerSec)}`)
if (perfSnap.timeoutSitesPerSec && Object.keys(perfSnap.timeoutSitesPerSec).length) console.log(`timeout sites/s: ${JSON.stringify(perfSnap.timeoutSitesPerSec)}`)
console.log(`dom: ${JSON.stringify({ ...dom, animated: dom.animated.length, animatedPseudo: dom.animatedPseudo.length })}`)
if (profileResult) { console.log(`profile: busy ${profileResult.busyPct}%`); for (const f of profileResult.topFunctions.slice(0, 12)) console.log(`  ${f.ms}ms ${f.pct}% ${f.key}`) }
if (samples.length) console.table(samples)
if (traceResult?.invalidations) console.log('invalidations:', JSON.stringify(traceResult.invalidations, null, 1))
if (consoleLines.length) console.log('console:', consoleLines.slice(0, 8))
await browser.close()
server.close()
