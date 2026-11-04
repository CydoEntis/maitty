// Summarises a Chrome trace (devtools.timeline) and a V8 CPU profile.
import fs from 'node:fs'
import path from 'node:path'
import { TraceMap, originalPositionFor } from '/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/.pnpm/@jridgewell+trace-mapping@0.3.31/node_modules/@jridgewell/trace-mapping/dist/trace-mapping.mjs'

const CATEGORY = new Map([
  // scripting
  ['FunctionCall', 'script'], ['EvaluateScript', 'script'], ['EvaluateModule', 'script'], ['v8.compile', 'script'], ['v8.compileModule', 'script'],
  ['V8.Execute', 'script'], ['v8.run', 'script'], ['v8.callFunction', 'script'], ['EventDispatch', 'script'], ['TimerFire', 'script'],
  ['TimerInstall', 'script'], ['TimerRemove', 'script'], ['FireAnimationFrame', 'script'], ['RequestAnimationFrame', 'script'],
  ['CancelAnimationFrame', 'script'], ['FireIdleCallback', 'script'], ['RunMicrotasks', 'script'], ['ProfileCall', 'script'],
  ['XHRReadyStateChange', 'script'], ['XHRLoad', 'script'], ['ParseHTML', 'script'], ['ParseAuthorStyleSheet', 'script'],
  ['MajorGC', 'gc'], ['MinorGC', 'gc'], ['GCEvent', 'gc'], ['V8.GCScavenger', 'gc'], ['V8.GC_MC_INCREMENTAL', 'gc'], ['BlinkGC.AtomicPhase', 'gc'],
  ['V8.GCFinalizeMC', 'gc'], ['V8.GCIncrementalMarking', 'gc'], ['V8.GCIncrementalMarkingStart', 'gc'], ['V8.GCIncrementalMarkingFinalize', 'gc'],
  ['ResizeObserver', 'script'], ['IntersectionObserverController::computeIntersections', 'layout'],
  // style
  ['UpdateLayoutTree', 'style'], ['RecalculateStyles', 'style'], ['ScheduleStyleRecalculation', 'style'], ['StyleRecalcInvalidationTracking', 'style'],
  ['StyleInvalidatorInvalidationTracking', 'style'],
  // layout
  ['Layout', 'layout'], ['HitTest', 'layout'], ['UpdateLayerTree', 'layout'], ['PrePaint', 'layout'], ['InvalidateLayout', 'layout'], ['LayoutShift', 'layout'],
  ['LocalFrameView::performLayout', 'layout'], ['LocalFrameView::UpdateStyleAndLayout', 'layout'],
  // paint
  ['Paint', 'paint'], ['PaintImage', 'paint'], ['Decode Image', 'paint'], ['ImageDecodeTask', 'paint'], ['DecodeLazyPixelRef', 'paint'], ['PaintSetup', 'paint'],
  ['Layerize', 'paint'], ['UpdateLayer', 'paint'], ['PaintArtifactCompositor::Update', 'paint'],
  // composite (main-thread side of cc)
  ['CompositeLayers', 'composite'], ['Commit', 'composite'], ['ProxyMain::BeginMainFrame::commit', 'composite'], ['ProxyMain::BeginMainFrame', 'composite'],
  ['BeginMainThreadFrame', 'composite'], ['ActivateLayerTree', 'composite'], ['LayerTreeHost::DoUpdateLayers', 'composite'], ['UpdateLayerTree', 'composite'],
  ['Animation', 'composite'], ['ScheduleStyleRecalculation', 'style'],
])

function categoryOf(name, cat) {
  const c = CATEGORY.get(name)
  if (c) return c
  if (name.startsWith('v8') || name.startsWith('V8.')) return 'script'
  if (cat && cat.includes('v8')) return 'script'
  return 'other'
}

export function analyzeTrace(events, wallMs) {
  const byPidThreads = new Map()
  const threadNames = new Map()
  for (const e of events) {
    if (e.ph === 'M' && e.name === 'thread_name') threadNames.set(`${e.pid}:${e.tid}`, e.args?.name)
  }
  // Pick the renderer main thread with the most events.
  const counts = new Map()
  for (const e of events) {
    const key = `${e.pid}:${e.tid}`
    if (threadNames.get(key) === 'CrRendererMain') counts.set(key, (counts.get(key) || 0) + 1)
  }
  let mainKey = null
  let best = -1
  for (const [k, v] of counts) if (v > best) { best = v; mainKey = k }
  if (!mainKey) return { error: 'no CrRendererMain thread found', threads: [...new Set(threadNames.values())] }
  const [mainPid, mainTid] = mainKey.split(':').map(Number)

  // Build complete events for main thread (X events; pair B/E).
  const main = []
  const openB = new Map()
  for (const e of events) {
    if (e.pid !== mainPid || e.tid !== mainTid) continue
    if (e.ph === 'X') main.push({ name: e.name, cat: e.cat, ts: e.ts, dur: e.dur || 0, args: e.args })
    else if (e.ph === 'B') { const k = e.name; if (!openB.has(k)) openB.set(k, []); openB.get(k).push(e) }
    else if (e.ph === 'E') { const stack = openB.get(e.name); const b = stack && stack.pop(); if (b) main.push({ name: e.name, cat: e.cat, ts: b.ts, dur: e.ts - b.ts, args: b.args }) }
  }
  main.sort((a, b) => a.ts - b.ts || b.dur - a.dur)
  const tsMin = main.length ? main[0].ts : 0
  let tsMax = 0
  for (const e of main) tsMax = Math.max(tsMax, e.ts + e.dur)
  const spanMs = wallMs || (tsMax - tsMin) / 1000

  // Self time per category via a stack.
  const self = {}
  const byName = {}
  const stack = []
  let topLevelBusy = 0
  const longTasks = []
  const topLevelDurs = []
  for (const e of main) {
    while (stack.length && stack[stack.length - 1].ts + stack[stack.length - 1].dur <= e.ts) stack.pop()
    if (stack.length === 0) {
      topLevelBusy += e.dur
      topLevelDurs.push(e.dur)
      if (e.dur > 50_000) longTasks.push({ name: e.name, ms: e.dur / 1000 })
    } else {
      const parent = stack[stack.length - 1]
      parent.childDur += e.dur
    }
    e.childDur = 0
    stack.push(e)
  }
  for (const e of main) {
    const s = Math.max(0, e.dur - e.childDur)
    const c = categoryOf(e.name, e.cat)
    self[c] = (self[c] || 0) + s
    byName[e.name] = (byName[e.name] || 0) + s
  }
  const toMsPerSec = (us) => us / 1000 / (spanMs / 1000)
  const categoriesMsPerSec = Object.fromEntries(Object.entries(self).map(([k, v]) => [k, +toMsPerSec(v).toFixed(1)]))
  const topNames = Object.entries(byName).sort((a, b) => b[1] - a[1]).slice(0, 25).map(([n, v]) => [n, +toMsPerSec(v).toFixed(2)])

  // Frames: DrawFrame on the renderer's compositor thread; BeginMainThreadFrame on main.
  let drawFrames = 0
  let mainFrames = 0
  let rasterUs = 0
  let compositorUs = 0
  for (const e of events) {
    if (e.pid !== mainPid) continue
    const tname = threadNames.get(`${e.pid}:${e.tid}`)
    if (e.name === 'DrawFrame' && (e.ph === 'I' || e.ph === 'X' || e.ph === 'n')) drawFrames += 1
    if (e.name === 'BeginMainThreadFrame') mainFrames += 1
    if (e.ph === 'X' && tname && tname.startsWith('CompositorTileWorker') ) rasterUs += e.dur || 0
    if (e.ph === 'X' && tname === 'Compositor') compositorUs += e.dur || 0
  }
  // Compositor thread self time: approximate as top-level only (count events with no overlap is costly); use RunTask sum.
  const layoutCount = main.filter((e) => e.name === 'Layout').length
  const styleCount = main.filter((e) => e.name === 'UpdateLayoutTree').length
  const paintCount = main.filter((e) => e.name === 'Paint').length
  const taskCount = topLevelDurs.length
  return {
    mainThread: `${mainPid}:${mainTid}`,
    spanSec: +(spanMs / 1000).toFixed(1),
    busyPct: +((topLevelBusy / 1000 / spanMs) * 100).toFixed(1),
    categoriesMsPerSec,
    tasksPerSec: +(taskCount / (spanMs / 1000)).toFixed(1),
    longTasks: longTasks.length,
    longTaskList: longTasks.slice(0, 10),
    layoutPerSec: +(layoutCount / (spanMs / 1000)).toFixed(1),
    stylePerSec: +(styleCount / (spanMs / 1000)).toFixed(1),
    paintPerSec: +(paintCount / (spanMs / 1000)).toFixed(1),
    drawFramesPerSec: +(drawFrames / (spanMs / 1000)).toFixed(1),
    mainFramesPerSec: +(mainFrames / (spanMs / 1000)).toFixed(1),
    rasterMsPerSec: +toMsPerSec(rasterUs).toFixed(1),
    compositorThreadMsPerSec: +toMsPerSec(compositorUs).toFixed(1),
    topEventNames: topNames,
  }
}

// Who invalidates style/layout, and which layouts are forced from script (needs the .stack category).
export function analyzeInvalidations(events, distDir, mainThread) {
  const [pid, tid] = mainThread.split(':').map(Number)
  const frameKey = (f) => {
    if (!f) return '(no stack)'
    let file = f.url ? f.url.split('/').pop() : ''
    let line = (f.lineNumber ?? 0) + 1
    let name = f.functionName || '(anon)'
    if (f.url) {
      const tm = loadMap(distDir, f.url)
      if (tm) {
        const pos = originalPositionFor(tm, { line: (f.lineNumber ?? 0) + 1, column: f.columnNumber ?? 0 })
        if (pos.source) {
          file = pos.source.replace(/^.*?\/node_modules\/\.pnpm\//, 'pnpm/').replace(/^(\.\.\/)+/, '').replace(/^home\/vox\/personal\/maitty-worktrees\/swift-lynx\/apps\/gui\//, '')
          line = pos.line
          if (pos.name) name = pos.name
          const vendor = VENDOR_MAPS.find((v) => pos.source.endsWith(v.suffix))
          if (vendor) {
            const tm2 = loadVendorMap(vendor.map)
            const pos2 = tm2 && originalPositionFor(tm2, { line: pos.line, column: pos.column })
            if (pos2 && pos2.source) { file = `xterm/${pos2.source.replace(/^.*?\/src\//, 'src/')}`; line = pos2.line; if (pos2.name) name = pos2.name }
          }
        }
      }
    }
    return `${name} @ ${file}:${line}`
  }
  const appFrame = (stack) => {
    if (!stack || !stack.length) return null
    // First frame that is in app or xterm code (skip react-dom internals for readability but keep as fallback).
    const pick = stack.find((f) => f.url && !/react-dom|scheduler|react\.|react-jsx/.test(frameKey(f))) || stack[0]
    return pick
  }
  const counters = { scheduleStyle: new Map(), invalidateLayout: new Map(), forcedLayout: new Map(), forcedStyle: new Map() }
  const bump = (m, k, v = 1) => m.set(k, (m.get(k) || 0) + v)
  let forcedLayouts = 0
  let forcedLayoutUs = 0
  let totalLayouts = 0
  let totalStyle = 0
  let forcedStyleCount = 0
  const sorted = events.filter((e) => e.pid === pid && e.tid === tid && (e.ph === 'X' || e.ph === 'I' || e.ph === 'B' || e.ph === 'E')).sort((a, b) => a.ts - b.ts)
  for (const e of sorted) {
    const data = e.args?.data || e.args?.beginData || {}
    const stack = data.stackTrace
    if (e.name === 'ScheduleStyleRecalculation') bump(counters.scheduleStyle, frameKey(appFrame(stack)))
    else if (e.name === 'InvalidateLayout') bump(counters.invalidateLayout, frameKey(appFrame(stack)))
    else if (e.name === 'Layout' && (e.ph === 'X' || e.ph === 'B')) {
      totalLayouts += 1
      if (stack && stack.length) { forcedLayouts += 1; forcedLayoutUs += e.dur || 0; bump(counters.forcedLayout, frameKey(appFrame(stack))) }
    } else if (e.name === 'UpdateLayoutTree' && (e.ph === 'X' || e.ph === 'B')) {
      totalStyle += 1
      if (stack && stack.length) { forcedStyleCount += 1; bump(counters.forcedStyle, frameKey(appFrame(stack)), 1) }
    }
  }
  const top = (m, n = 12) => [...m.entries()].sort((a, b) => b[1] - a[1]).slice(0, n)
  return { totalLayouts, forcedLayouts, forcedLayoutMs: +(forcedLayoutUs / 1000).toFixed(1), totalStyleRecalcs: totalStyle, forcedStyleRecalcs: forcedStyleCount, scheduleStyleBy: top(counters.scheduleStyle), invalidateLayoutBy: top(counters.invalidateLayout), forcedLayoutBy: top(counters.forcedLayout), forcedStyleBy: top(counters.forcedStyle) }
}

// ---- CPU profile ----
const VENDOR_MAPS = [
  { suffix: '@xterm/xterm/lib/xterm.mjs', map: '/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/@xterm/xterm/lib/xterm.mjs.map' },
  { suffix: '@xterm/addon-fit/lib/addon-fit.mjs', map: '/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/@xterm/addon-fit/lib/addon-fit.mjs.map' },
  { suffix: '@xterm/addon-search/lib/addon-search.mjs', map: '/home/vox/personal/maitty-worktrees/swift-lynx/apps/gui/node_modules/@xterm/addon-search/lib/addon-search.mjs.map' },
]
const vendorCache = new Map()
function loadVendorMap(p) {
  if (vendorCache.has(p)) return vendorCache.get(p)
  let tm = null
  if (fs.existsSync(p)) tm = new TraceMap(JSON.parse(fs.readFileSync(p, 'utf8')))
  vendorCache.set(p, tm)
  return tm
}
const mapCache = new Map()
function loadMap(distDir, url) {
  const base = url.split('/').pop()
  if (!base) return null
  if (mapCache.has(base)) return mapCache.get(base)
  const p = path.join(distDir, 'assets', `${base}.map`)
  let tm = null
  if (fs.existsSync(p)) tm = new TraceMap(JSON.parse(fs.readFileSync(p, 'utf8')))
  mapCache.set(base, tm)
  return tm
}

export function analyzeProfile(profile, distDir) {
  const nodes = new Map()
  for (const n of profile.nodes) nodes.set(n.id, n)
  const selfUs = new Map()
  const { samples, timeDeltas } = profile
  for (let i = 0; i < samples.length; i += 1) {
    const d = timeDeltas[i] || 0
    selfUs.set(samples[i], (selfUs.get(samples[i]) || 0) + d)
  }
  const total = (profile.endTime - profile.startTime)
  const byFn = new Map()
  const byFile = new Map()
  const byRaw = new Map()
  for (const [id, us] of selfUs) {
    const n = nodes.get(id)
    const cf = n.callFrame
    let file = cf.url ? cf.url.split('/').pop() : cf.functionName
    let line = cf.lineNumber + 1
    let name = cf.functionName || '(anonymous)'
    let origin = ''
    if (cf.url) {
      const tm = loadMap(distDir, cf.url)
      if (tm) {
        const pos = originalPositionFor(tm, { line: cf.lineNumber + 1, column: cf.columnNumber })
        if (pos.source) {
          file = pos.source.replace(/^.*?\/node_modules\/\.pnpm\//, 'pnpm/').replace(/^(\.\.\/)+/, '')
          line = pos.line
          if (pos.name) name = `${pos.name}${cf.functionName && cf.functionName !== pos.name ? ` (${cf.functionName})` : ''}`
          origin = file
          // xterm ships a minified bundle with its own map: resolve one level further.
          const vendor = VENDOR_MAPS.find((v) => pos.source.endsWith(v.suffix))
          if (vendor) {
            const tm2 = loadVendorMap(vendor.map)
            const pos2 = tm2 && originalPositionFor(tm2, { line: pos.line, column: pos.column })
            if (pos2 && pos2.source) {
              file = `xterm/${pos2.source.replace(/^.*?\/src\//, 'src/')}`
              line = pos2.line
              if (pos2.name) name = `${pos2.name} (${cf.functionName || 'anon'})`
              origin = file
            }
          }
        }
      }
    } else {
      file = cf.functionName // (program), (idle), (garbage collector), (root)
      line = 0
    }
    const key = `${name} @ ${file}:${line}`
    byFn.set(key, (byFn.get(key) || 0) + us)
    const fileKey = origin || file
    byFile.set(fileKey, (byFile.get(fileKey) || 0) + us)
    const rawKey = `${cf.functionName || '(anon)'} ${cf.url ? cf.url.split('/').pop() : ''}:${cf.lineNumber + 1}`
    byRaw.set(rawKey, (byRaw.get(rawKey) || 0) + us)
  }
  const fmt = (m, n) => [...m.entries()].sort((a, b) => b[1] - a[1]).slice(0, n).map(([k, v]) => ({ key: k, ms: +(v / 1000).toFixed(1), pct: +((v / total) * 100).toFixed(1) }))
  const idle = selfUs.get([...nodes.values()].find((n) => n.callFrame.functionName === '(idle)')?.id) || 0
  return {
    totalSec: +(total / 1e6).toFixed(1),
    idlePct: +((idle / total) * 100).toFixed(1),
    busyPct: +(((total - idle) / total) * 100).toFixed(1),
    topFunctions: fmt(byFn, 25),
    topFiles: fmt(byFile, 20),
    topRaw: fmt(byRaw, 15),
  }
}
