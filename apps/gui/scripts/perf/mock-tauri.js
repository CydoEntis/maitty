// Injected with page.addInitScript before the app loads. Emulates the Tauri 2 host:
// window.isTauri, __TAURI_INTERNALS__.invoke/transformCallback/metadata, the event plugin,
// and a fake maittyd that answers list_sessions and streams output frames over the channel the view opens.
// Reads its configuration from window.__SCENARIO (set by an earlier init script).
(() => {
  const S = window.__SCENARIO || {}
  const sessions = S.sessions || [{ id: 1, status: 'Idle', stream: false }]
  const bytesPerSec = S.bytesPerSec || 20000
  const tickMs = S.tickMs || 50

  // Originals, so the fake daemon's own timers are not counted as app timers.
  const origSetInterval = window.setInterval.bind(window)
  const origSetTimeout = window.setTimeout.bind(window)
  const origClearInterval = window.clearInterval.bind(window)
  const origClearTimeout = window.clearTimeout.bind(window)
  const origRAF = window.requestAnimationFrame.bind(window)
  const origCAF = window.cancelAnimationFrame.bind(window)

  const perf = (window.__perf = {
    invokes: {},
    events: 0,
    eventBytes: 0,
    timers: { timeoutFired: 0, intervalFired: 0, rafFired: 0, timeoutSet: 0, intervalSet: 0, rafSet: 0 },
    intervals: new Map(), // id -> { ms, site }
    intervalFires: {}, // "ms@site" -> fires
    timeoutFires: {}, // "ms" -> fires
    timeoutSites: {}, // "ms@site" -> sets (only with timerStacks)
    blockedPeriods: new Set(),
    startedAt: performance.now(),
    reset() {
      this.invokes = {}
      this.events = 0
      this.eventBytes = 0
      this.timers = { timeoutFired: 0, intervalFired: 0, rafFired: 0, timeoutSet: 0, intervalSet: 0, rafSet: 0 }
      this.intervalFires = {}
      this.timeoutFires = {}
      this.timeoutSites = {}
      this.startedAt = performance.now()
    },
    snapshot() {
      const secs = (performance.now() - this.startedAt) / 1000
      return {
        seconds: secs,
        invokes: this.invokes,
        eventsPerSec: this.events / secs,
        eventKBPerSec: this.eventBytes / 1024 / secs,
        timersPerSec: Object.fromEntries(Object.entries(this.timers).map(([k, v]) => [k, v / secs])),
        activeIntervals: [...this.intervals.entries()].map(([id, v]) => ({ id, ms: v.ms, site: v.site })),
        intervalFiresPerSec: Object.fromEntries(Object.entries(this.intervalFires).map(([k, v]) => [k, v / secs])),
        timeoutFiresPerSec: Object.fromEntries(Object.entries(this.timeoutFires).map(([k, v]) => [k, v / secs])),
        timeoutSitesPerSec: Object.fromEntries(Object.entries(this.timeoutSites).sort((a, b) => b[1] - a[1]).slice(0, 15).map(([k, v]) => [k, +(v / secs).toFixed(1)])),
      }
    },
    // Experiment: stop every interval with this period, now and in future.
    blockPeriod(ms) {
      this.blockedPeriods.add(ms)
      for (const [id, v] of this.intervals) if (v.ms === ms) { origClearInterval(id); this.intervals.delete(id) }
    },
  })

  function site() {
    const stack = (new Error().stack || '').split('\n').slice(2)
    const line = stack.find((l) => l.includes('/assets/')) || stack[0] || ''
    const m = /assets\/([^:)]+):(\d+):(\d+)/.exec(line)
    return m ? `${m[1]}:${m[2]}:${m[3]}` : line.trim().slice(0, 80)
  }

  // ---- Timer wrappers (count firings) ----
  window.setInterval = function (fn, ms, ...rest) {
    perf.timers.intervalSet += 1
    const where = site()
    const period = Number(ms) || 0
    const key = `${period}@${where}`
    if (perf.blockedPeriods.has(period)) return origSetInterval(() => {}, 1e9)
    const id = origSetInterval(function (...a) {
      perf.timers.intervalFired += 1
      perf.intervalFires[key] = (perf.intervalFires[key] || 0) + 1
      return typeof fn === 'function' ? fn.apply(this, a) : undefined
    }, ms, ...rest)
    perf.intervals.set(id, { ms: period, site: where })
    return id
  }
  window.clearInterval = function (id) { perf.intervals.delete(id); return origClearInterval(id) }
  window.setTimeout = function (fn, ms, ...rest) {
    perf.timers.timeoutSet += 1
    const period = Number(ms) || 0
    if (S.timerStacks) {
      const key = `${period}@${site()}`
      perf.timeoutSites[key] = (perf.timeoutSites[key] || 0) + 1
    }
    return origSetTimeout(function (...a) {
      perf.timers.timeoutFired += 1
      perf.timeoutFires[period] = (perf.timeoutFires[period] || 0) + 1
      return typeof fn === 'function' ? fn.apply(this, a) : undefined
    }, ms, ...rest)
  }
  window.clearTimeout = origClearTimeout
  window.requestAnimationFrame = function (fn) {
    perf.timers.rafSet += 1
    return origRAF(function (t) { perf.timers.rafFired += 1; return fn(t) })
  }
  window.cancelAnimationFrame = origCAF

  // ---- Tauri internals ----
  let nextCallbackId = 1000
  const callbacks = new Map()
  const listeners = new Map() // event name -> Map(eventId -> handlerCallbackId)
  let nextEventId = 1

  function transformCallback(callback, once) {
    const id = nextCallbackId++
    callbacks.set(id, (payload) => {
      if (once) callbacks.delete(id)
      return callback(payload)
    })
    return id
  }

  let outputChannel = null
  // An experiment: keep generating output but deliver none, to see what the generator itself costs.
  let sinkOutput = false
  function dispatch(event, payload) {
    const subs = listeners.get(event)
    if (!subs || subs.size === 0) return
    // The real host serialises to JSON and evaluates it in the page; emulate the parse cost.
    const text = JSON.stringify(payload)
    perf.events += 1
    perf.eventBytes += text.length
    for (const [eventId, cbId] of subs) {
      const cb = callbacks.get(cbId)
      if (cb) cb({ event, id: eventId, payload: JSON.parse(text) })
    }
  }

  window.isTauri = true
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
    unregisterListener(event, eventId) {
      const subs = listeners.get(event)
      if (subs) subs.delete(eventId)
    },
  }

  // ---- Fake daemon state ----
  const encoder = new TextEncoder()
  function b64(bytes) {
    let s = ''
    for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000))
    return btoa(s)
  }
  const seqs = new Map()
  const startedAt = Date.now()

  function termRecord(s) {
    const working = s.status === 'Working'
    return {
      id: s.id,
      agent: s.agent === undefined ? 'claude' : s.agent,
      process: s.agent === null ? 'zsh' : 'claude',
      summary: working ? 'Editing src/features/workspace/DesktopApp.tsx' : '',
      status: s.status,
      cwd: s.cwd || `/home/vox/projects/app${s.id}`,
      root: s.cwd || `/home/vox/projects/app${s.id}`,
      branch: 'dev',
      label: '',
      pinned: false,
      archived: false,
      conversation_id: null,
      popup: false,
      remote: null,
      asking: '',
      interrupted: null,
      timeline: working ? [{ at: Math.floor(startedAt / 1000) - 30, kind: 'Working', detail: 'Started' }] : [],
      usage: { context: working ? 0.42 : null, cost: working ? 1.23 : null, tokens: working ? 12345 : null },
    }
  }

  function snapshot() {
    const terms = {}
    for (const s of sessions) terms[String(s.id)] = termRecord(s)
    const cwds = [...new Set(sessions.map((s) => s.cwd || `/home/vox/projects/app${s.id}`))]
    return {
      conversations: [],
      workspaces: cwds.map((cwd) => ({ cwd, git: { branch: 'dev', dirty: 3 } })),
      terms,
    }
  }

  const PROMPT = '\x1b[1;32mvox@box\x1b[0m:\x1b[1;34m~/projects/app\x1b[0m$ '
  function initialScreen(s) {
    const lines = []
    lines.push('\x1b[2J\x1b[H')
    for (let i = 0; i < 30; i += 1) lines.push(`${PROMPT}echo line ${i}\r\nline ${i} \x1b[33mwarning\x1b[0m something \x1b[36m/src/file${i}.ts:12\x1b[0m\r\n`)
    if (s.status === 'Working') lines.push('\x1b[38;5;214m✻\x1b[0m \x1b[1mWelcome to Claude Code\x1b[0m\r\n\r\n')
    else lines.push(PROMPT)
    return encoder.encode(lines.join(''))
  }

  const mockInvoke = async (cmd, args = {}) => {
    perf.invokes[cmd] = (perf.invokes[cmd] || 0) + 1
    switch (cmd) {
      case 'plugin:event|listen': {
        const id = nextEventId++
        if (!listeners.has(args.event)) listeners.set(args.event, new Map())
        listeners.get(args.event).set(id, args.handler)
        return id
      }
      case 'plugin:event|unlisten': {
        const subs = listeners.get(args.event)
        if (subs) subs.delete(args.eventId)
        return null
      }
      case 'plugin:event|emit':
      case 'plugin:event|emit_to':
        return null
      case 'list_sessions':
        return snapshot()
      case 'default_session_folder':
        return '/home/vox'
      case 'suggest_project_folders':
        return []
      case 'terminal_history': {
        // Raw bytes, as the host answers since the output moved to a channel: the sequence number
        // as 8 big-endian bytes, then the screen.
        const s = sessions.find((x) => x.id === args.term) || sessions[0]
        const screen = initialScreen(s)
        const reply = new Uint8Array(8 + screen.length)
        new DataView(reply.buffer).setBigUint64(0, BigInt(seqs.get(args.term) || 0))
        reply.set(screen, 8)
        return reply.buffer
      }
      case 'watch_terminal_output':
        outputChannel = args.onChunk
        return null
      case 'pane_terminal_replay':
      case 'terminal_replay': {
        const s = sessions.find((x) => x.id === args.term) || sessions[0]
        return Array.from(initialScreen(s))
      }
      case 'pane_terminal_screen':
      case 'terminal_screen':
        return ''
      case 'repo_query': {
        const q = args.question
        if (q === 'Status') return { Status: { branch: 'dev', ahead: 0, behind: 0, files: [{ path: 'src/a.ts', kind: 'Modified' }] } }
        if (q === 'Branches') return { Branches: [{ name: 'dev', current: true }, { name: 'main', current: false }] }
        if (q === 'Worktrees') return { Worktrees: [] }
        if (q === 'Snapshots') return { Snapshots: [] }
        if (q === 'References') return { References: [] }
        if (q && q.Diff) return { Diff: { files: [{ path: 'src/a.ts', additions: 12, deletions: 4, patch: '@@ -1,3 +1,4 @@\n+x\n', binary: false }], additions: 12, deletions: 4, total_files: 1, truncated: false } }
        if (q && q.Tree) return { Tree: [{ name: 'src', is_directory: true }, { name: 'README.md', is_directory: false }] }
        if (q && q.Log) return { Log: [] }
        if (q && q.File) return { File: 'hello' }
        if (q && q.Locate) return { Located: 'src/a.ts' }
        return { Status: { branch: 'dev', ahead: 0, behind: 0, files: [] } }
      }
      case 'terminal_input':
      case 'pane_terminal_resize':
      case 'resize_terminal':
      case 'mark_terminal_seen':
      case 'pin_terminal_session':
      case 'archive_terminal_session':
      case 'rename_terminal_session':
      case 'close_terminal_session':
      case 'start_shell_session':
      case 'start_terminal_session':
      case 'new_worktree_session':
        return null
      case 'search_panes':
        return []
      case 'daemon_restart_status':
        return { daemonProtocol: 1, guiProtocol: 1, canRestart: false, sessionCount: sessions.length, activeTurnCount: null }
      case 'plugin:app|version':
        return '0.62.1'
      case 'plugin:app|name':
        return 'Maitty'
      case 'plugin:updater|check':
        return null
      case 'plugin:notification|is_permission_granted':
        return true
      case 'plugin:notification|request_permission':
        return 'granted'
      case 'plugin:notification|notify':
        return null
      case 'plugin:dialog|open':
        return null
      case 'plugin:process|restart':
        return null
      case 'plugin:window|is_maximized':
      case 'plugin:window|is_fullscreen':
      case 'plugin:window|is_minimized':
        return false
      case 'plugin:window|is_focused':
        return true
      case 'plugin:window|scale_factor':
        return window.devicePixelRatio
      case 'plugin:window|inner_size':
        return { type: 'Physical', data: { width: window.innerWidth, height: window.innerHeight } }
      case 'plugin:window|theme':
        return 'dark'
      case 'plugin:webview|internal_toggle_devtools':
        return null
      default:
        if (cmd.startsWith('plugin:window|') || cmd.startsWith('plugin:webview|')) return null
        console.warn('[mock-tauri] unhandled invoke', cmd, JSON.stringify(args).slice(0, 200))
        return null
    }
  }

  window.__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main', windowLabel: 'main' },
      windows: [{ label: 'main' }],
      webviews: [{ label: 'main', windowLabel: 'main' }],
    },
    callbacks,
    transformCallback,
    unregisterCallback(id) { callbacks.delete(id) },
    runCallback(id, payload) { const cb = callbacks.get(id); if (cb) cb(payload) },
    invoke(cmd, args, options) {
      // Real IPC round-trips through the host; a resolved promise on a macrotask is the closest cheap emulation.
      return new Promise((resolve, reject) => {
        origSetTimeout(() => { mockInvoke(cmd, args, options).then(resolve, reject) }, 0)
      })
    },
    convertFileSrc: (p) => p,
  }

  // ---- Claude-Code-like output stream ----
  let seed = 12345
  const rand = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff }
  const SPIN = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
  const WORDS = ['const', 'function', 'return', 'session', 'layout', 'pane', 'terminal', 'daemon', 'status', 'render', 'useEffect', 'import', 'export', 'async', 'await', 'Promise', 'string', 'number']
  const COLORS = ['\x1b[32m', '\x1b[31m', '\x1b[36m', '\x1b[33m', '\x1b[35m', '\x1b[2m', '\x1b[1m', '\x1b[38;5;153m']
  function codeLine() {
    const n = 6 + Math.floor(rand() * 8)
    const parts = []
    for (let i = 0; i < n; i += 1) parts.push(`${COLORS[Math.floor(rand() * COLORS.length)]}${WORDS[Math.floor(rand() * WORDS.length)]}\x1b[0m`)
    const sign = rand() < 0.5 ? '\x1b[32m+ ' : rand() < 0.3 ? '\x1b[31m- ' : '  '
    return `${sign}${String(Math.floor(rand() * 400)).padStart(4)} │ ${parts.join(' ')}\x1b[0m\r\n`
  }
  let tick = 0
  const budgetPerTick = Math.round(bytesPerSec * tickMs / 1000)
  function generate(termId) {
    tick += 1
    const t = Math.floor((Date.now() - startedAt) / 1000)
    let out = `\r\x1b[2K\x1b[38;5;214m${SPIN[tick % SPIN.length]}\x1b[0m \x1b[1mThinking…\x1b[0m \x1b[2m(${t}s · ↓ ${(tick * 7) % 9000} tokens · esc to interrupt)\x1b[0m`
    if (tick % 10 === 0) {
      // Redraw a box of status lines in place, as Claude Code does with its tool panel.
      out += '\x1b[6A'
      for (let i = 0; i < 6; i += 1) out += `\x1b[2K\x1b[2m│\x1b[0m ${codeLine()}`
    }
    while (out.length < budgetPerTick) out += codeLine()
    return out
  }
  // One frame per pane per tick over the channel the view opened: the pane as 4 big-endian bytes,
  // the sequence number as 8, one replay byte, then the output (apps/gui/src/api/terminalStream.ts).
  function frameOf(term, seq, data) {
    const frame = new Uint8Array(13 + data.length)
    const view = new DataView(frame.buffer)
    view.setUint32(0, term)
    view.setBigUint64(4, BigInt(seq))
    frame[12] = 0
    frame.set(data, 13)
    return frame.buffer
  }
  const streaming = sessions.filter((s) => s.stream)
  if (streaming.length > 0) {
    origSetInterval(() => {
      for (const s of streaming) {
        if (!outputChannel) continue
        const seq = (seqs.get(s.id) || 0) + 1
        seqs.set(s.id, seq)
        const frame = frameOf(s.id, seq, encoder.encode(generate(s.id)))
        if (!sinkOutput) outputChannel.onmessage(frame)
      }
    }, tickMs)
  }
  window.__mock = { dispatch, listeners, sessions, dropStream: () => { outputChannel = null }, sinkOutput: () => { sinkOutput = true } }
})()
