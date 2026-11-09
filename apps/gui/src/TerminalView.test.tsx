import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import type { TerminalChunk, TerminalHistory } from '@/api/terminalStream'
import { clearScreens } from '@/features/terminal/screenCache'
import { TerminalView } from './TerminalView'

type Target = Pick<Conversation, 'id'> & { generation: number }

const { loadTerminalReplay, resizeTerminal, sendTerminalInput, loadTerminalHistory, listenTerminalChunks, terminalState, toastError } = vi.hoisted(() => ({
  loadTerminalReplay: vi.fn<(conversation: Target) => Promise<Uint8Array>>(),
  resizeTerminal: vi.fn<(conversation: Target, cols: number, rows: number) => Promise<void>>(),
  sendTerminalInput: vi.fn<(conversation: Target, input: string | Uint8Array) => Promise<void>>(),
  loadTerminalHistory: vi.fn<(term: number) => Promise<TerminalHistory | null>>(),
  listenTerminalChunks: vi.fn<(term: number, onChunk: (chunk: TerminalChunk) => void) => Promise<() => void>>(),
  terminalState: {
    writes: [] as Uint8Array[],
    resets: 0,
    onData: undefined as ((data: string) => void) | undefined,
    keyHandler: undefined as ((event: Pick<KeyboardEvent, 'type' | 'key' | 'shiftKey' | 'ctrlKey' | 'altKey' | 'metaKey'>) => boolean) | undefined,
    onResize: undefined as (() => void) | undefined,
    resolveResize: undefined as (() => void) | undefined,
    onChunk: undefined as ((chunk: TerminalChunk) => void) | undefined,
    cols: 80,
    rows: 24,
    options: undefined as { fontSize?: number; cursorBlink?: boolean } | undefined,
    linkProvider: undefined as { provideLinks: (y: number, done: (links: Array<{ text: string; activate: (event: MouseEvent) => void }> | undefined) => void) => void } | undefined,
    line: '',
    searches: [] as string[],
    oscHandlers: new Map<number, (data: string) => boolean | Promise<boolean>>(),
    /** Runs while a write is being parsed, where xterm answers the queries in it. */
    duringWrite: undefined as ((data: Uint8Array) => void) | undefined,
  },
  toastError: vi.fn(),
}))

vi.mock('@/lib/toast', () => ({ toast: { error: toastError } }))

vi.mock('@/api/daemon', () => ({
  isDesktop: () => true,
  loadTerminalReplay,
  resizeTerminal,
  sendTerminalInput,
}))

vi.mock('@/api/terminalStream', () => ({ listenTerminalChunks, loadTerminalHistory }))

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    options: { fontSize?: number; cursorBlink?: boolean }
    constructor(options: { fontSize?: number; cursorBlink?: boolean }) {
      this.options = options
      terminalState.options = this.options
    }
    get cols() { return terminalState.cols }
    get rows() { return terminalState.rows }
    loadAddon() {}
    open() {}
    focus() {}
    clearSelection() {}
    registerLinkProvider(provider: typeof terminalState.linkProvider) {
      terminalState.linkProvider = provider
      return { dispose: vi.fn() }
    }
    buffer = { active: { getLine: () => ({ translateToString: () => terminalState.line }) } }
    reset() { terminalState.resets += 1 }
    onData(callback: (data: string) => void) {
      terminalState.onData = callback
      return { dispose: vi.fn() }
    }
    onBinary() { return { dispose: vi.fn() } }
    attachCustomKeyEventHandler(handler: NonNullable<typeof terminalState.keyHandler>) { terminalState.keyHandler = handler }
    write(data: Uint8Array, done?: () => void) { terminalState.writes.push(data); terminalState.duringWrite?.(data); done?.() }
    parser = { registerOscHandler: (ident: number, handler: (data: string) => boolean | Promise<boolean>) => { terminalState.oscHandlers.set(ident, handler); return { dispose: vi.fn() } } }
    dispose() {}
  },
}))

vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }))
vi.mock('@xterm/addon-search', () => ({
  SearchAddon: class {
    findNext(query: string) { terminalState.searches.push(`next:${query}`); return query === 'present' }
    findPrevious(query: string) { terminalState.searches.push(`previous:${query}`); return true }
    clearDecorations() {}
  },
}))

const conversation: Conversation = {
  id: 't47',
  generation: 0,
  title: 'Shell',
  project: 'maitty',
  folder: '/home/casey/maitty',
  branch: 'main',
  execution: 'Native',
  kind: 'terminal',
  provider: 'Codex',
  status: 'idle',
  updated: null,
  model: 'Provider default',
  permission: 'Terminal session',
  context: null,
  nativePane: 47,
  hasDiff: false,
}

const binding = { id: conversation.id, generation: conversation.generation }
const bytes = (text: string) => new TextEncoder().encode(text)
const chunk = (seq: number, text: string, replay = false): TerminalChunk => ({ machine: 0, term: 47, seq, replay, bytes: bytes(text) })
const RESET = '\x1bc'
const written = () => terminalState.writes.map((part) => new TextDecoder().decode(part)).join('')

describe('terminal view', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    clearScreens()
    terminalState.writes = []
    terminalState.resets = 0
    terminalState.onData = undefined
    terminalState.duringWrite = undefined
    terminalState.onResize = undefined
    terminalState.resolveResize = undefined
    terminalState.onChunk = undefined
    terminalState.cols = 80
    terminalState.rows = 24
    terminalState.options = undefined
    terminalState.linkProvider = undefined
    terminalState.line = ''
    terminalState.searches = []
    vi.stubGlobal('ResizeObserver', class {
      private readonly callback: () => void
      constructor(callback: () => void) { this.callback = callback }
      observe() {
        terminalState.onResize = this.callback
        this.callback()
      }
      disconnect() {}
    })
    // Output is written once per frame; run frames on the next tick so tests can wait for them.
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => window.setTimeout(() => { callback(0) }, 0))
    vi.stubGlobal('cancelAnimationFrame', (id: number) => { window.clearTimeout(id) })
    // jsdom has no WebGL, so the view takes xterm's DOM renderer, as a web view without GPU access does.
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)
    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    listenTerminalChunks.mockImplementation((_term, onChunk) => {
      terminalState.onChunk = onChunk
      return Promise.resolve(() => { terminalState.onChunk = undefined })
    })
    loadTerminalHistory.mockResolvedValue({ seq: 3, bytes: bytes('hello ') })
    loadTerminalReplay.mockResolvedValue(bytes('fresh'))
    resizeTerminal.mockResolvedValue()
    sendTerminalInput.mockResolvedValue()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('writes nothing while it is paused and the whole screen once it shows', async () => {
    const view = render(renderPaused(true))
    await waitFor(() => { expect(terminalState.onChunk).toBeDefined() })
    await waitFor(() => { expect(resizeTerminal).toHaveBeenCalledWith(binding, 80, 24) })
    act(() => { terminalState.onChunk?.(chunk(4, 'world')) })
    await new Promise((resolve) => { window.setTimeout(resolve, 20) })
    expect(written()).toBe('')

    view.rerender(renderPaused(false))

    await waitFor(() => { expect(written()).toBe(`${RESET}hello world`) })
    act(() => { terminalState.onChunk?.(chunk(5, '!')) })
    await waitFor(() => { expect(written()).toBe(`${RESET}hello world!`) })
  })

  it('keeps what it showed and adds what came while a chat covered it', async () => {
    const view = render(renderPaused(false))
    await waitFor(() => { expect(written()).toBe('hello ') })

    view.rerender(renderPaused(true))
    act(() => { terminalState.onChunk?.(chunk(4, 'world')) })
    await new Promise((resolve) => { window.setTimeout(resolve, 20) })
    expect(written()).toBe('hello ')

    view.rerender(renderPaused(false))

    await waitFor(() => { expect(written()).toBe(`hello ${RESET}hello world`) })
  })

  it('asks the host for the screen when the kept one is gone', async () => {
    const view = render(renderPaused(false))
    await waitFor(() => { expect(written()).toBe('hello ') })

    view.rerender(renderPaused(true))
    act(() => { terminalState.onChunk?.(chunk(4, 'world')) })
    clearScreens()
    loadTerminalHistory.mockResolvedValue({ seq: 4, bytes: bytes('all of it') })
    view.rerender(renderPaused(false))

    await waitFor(() => { expect(written()).toBe(`hello ${RESET}all of it`) })
  })

  it('writes the history, then live output once, and sends typing to the exact session', async () => {
    renderTerminal()

    expect(await screen.findByRole('region', { name: 'Native Codex terminal' })).toBeInTheDocument()
    await waitFor(() => { expect(written()).toBe('hello ') })
    expect(loadTerminalHistory).toHaveBeenCalledWith(47)
    expect(resizeTerminal).toHaveBeenCalledWith(binding, 80, 24)

    act(() => {
      terminalState.onChunk?.(chunk(3, 'repeat'))
      terminalState.onChunk?.(chunk(4, 'world'))
    })
    await waitFor(() => { expect(written()).toBe('hello world') })

    terminalState.onData?.('\u0003')
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, '\u0003') })
  })

  it('does not send what xterm answers to the queries in a replay, but answers the same queries live', async () => {
    const DEVICE_ATTRIBUTES_REPLY = '\x1b[?1;2c'
    renderTerminal()
    await waitFor(() => { expect(written()).toBe('hello ') })
    // xterm answers a device-attributes query while it parses the write that holds one.
    terminalState.duringWrite = (data) => {
      if (new TextDecoder().decode(data).includes('\x1b[c')) terminalState.onData?.(DEVICE_ATTRIBUTES_REPLY)
    }

    act(() => { terminalState.onChunk?.(chunk(5, 'old screen \x1b[c', true)) })
    await waitFor(() => { expect(written()).toContain('old screen') })
    expect(sendTerminalInput).not.toHaveBeenCalledWith(binding, DEVICE_ATTRIBUTES_REPLY)

    act(() => { terminalState.onChunk?.(chunk(6, 'live \x1b[c')) })
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, DEVICE_ATTRIBUTES_REPLY) })
    expect(sendTerminalInput).toHaveBeenCalledTimes(1)
  })

  it("answers a program's background query with the theme colour, not the see-through canvas", async () => {
    renderTerminal()
    expect(await screen.findByRole('region', { name: 'Native Codex terminal' })).toBeInTheDocument()
    await waitFor(() => { expect(terminalState.oscHandlers.has(11)).toBe(true) })

    expect(terminalState.oscHandlers.get(11)?.('?')).toBe(true)
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, '\u001b]11;rgb:0606/0d0d/1616\u001b\\') })
    expect(terminalState.oscHandlers.get(10)?.('?')).toBe(true)
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, '\u001b]10;rgb:c9c9/d6d6/e3e3\u001b\\') })
    // Setting a colour is xterm's business.
    expect(terminalState.oscHandlers.get(11)?.('#000000')).toBe(false)
  })

  it('sends Alt+Enter for Shift+Enter so an agent starts a new line, and leaves Enter alone', async () => {
    renderTerminal()
    await screen.findByRole('region', { name: 'Native Codex terminal' })
    await waitFor(() => { expect(terminalState.keyHandler).toBeDefined() })
    const key = (extra: Partial<KeyboardEvent>) => ({ type: 'keydown', key: 'Enter', shiftKey: false, ctrlKey: false, altKey: false, metaKey: false, ...extra })

    expect(terminalState.keyHandler?.(key({ shiftKey: true }))).toBe(false)
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, '\u001b\r') })
    expect(sendTerminalInput).toHaveBeenCalledTimes(1)

    expect(terminalState.keyHandler?.(key({ shiftKey: true, type: 'keyup' }))).toBe(false)
    expect(terminalState.keyHandler?.(key({}))).toBe(true)
    expect(terminalState.keyHandler?.(key({ shiftKey: true, ctrlKey: true }))).toBe(true)
    expect(sendTerminalInput).toHaveBeenCalledTimes(1)
  })

  it('writes output at once while the window is hidden, since no animation frame will come', async () => {
    renderTerminal()
    await screen.findByRole('region', { name: 'Native Codex terminal' })
    await waitFor(() => { expect(written()).toBe('hello ') })
    Object.defineProperty(document, 'hidden', { configurable: true, value: true })
    try {
      act(() => { terminalState.onChunk?.(chunk(4, 'while away')) })

      expect(written()).toBe('hello while away')
    } finally {
      Object.defineProperty(document, 'hidden', { configurable: true, value: false })
    }
  })

  it('tries typing again after a pause instead of waiting for a click, and never sends keys out of order', async () => {
    renderTerminal()
    await screen.findByRole('region', { name: 'Native Codex terminal' })
    await waitFor(() => { expect(terminalState.onData).toBeDefined() })
    sendTerminalInput.mockRejectedValueOnce(new Error('daemon gone'))
    const now = vi.spyOn(Date, 'now')
    try {
      now.mockReturnValue(1_000)
      terminalState.onData?.('a')
      await waitFor(() => { expect(screen.getByRole('button', { name: /Try typing again/ })).toBeInTheDocument() })

      terminalState.onData?.('b')
      expect(sendTerminalInput).toHaveBeenCalledTimes(1)

      now.mockReturnValue(5_000)
      terminalState.onData?.('c')
      await waitFor(() => { expect(sendTerminalInput).toHaveBeenLastCalledWith(binding, 'c') })
      expect(sendTerminalInput).not.toHaveBeenCalledWith(binding, 'b')
    } finally {
      now.mockRestore()
    }
  })

  it('sends each key at once, without waiting for the host to answer the one before', async () => {
    renderTerminal()
    await screen.findByRole('region', { name: 'Native Codex terminal' })
    await waitFor(() => { expect(terminalState.onData).toBeDefined() })
    // No reply ever comes: a reply can take as long as the web view's next event.
    sendTerminalInput.mockReturnValue(new Promise<void>(() => undefined))

    terminalState.onData?.('h')
    terminalState.onData?.('i')
    terminalState.onData?.('\x7f')

    expect(sendTerminalInput.mock.calls.map(([, data]) => data)).toEqual(['h', 'i', '\x7f'])
  })

  it('holds output that arrives while the history loads, so nothing is lost or repeated', async () => {
    let resolveHistory: (history: TerminalHistory) => void = () => {}
    loadTerminalHistory.mockReturnValue(new Promise<TerminalHistory>((resolve) => { resolveHistory = resolve }))
    renderTerminal()
    await waitFor(() => { expect(terminalState.onChunk).toBeDefined() })

    act(() => {
      terminalState.onChunk?.(chunk(3, 'already in history'))
      terminalState.onChunk?.(chunk(4, 'after '))
    })
    expect(written()).toBe('')

    await act(async () => {
      resolveHistory({ seq: 3, bytes: bytes('before ') })
      await Promise.resolve()
    })
    await waitFor(() => { expect(written()).toBe('before after ') })
  })

  it('clears the screen before writing a replay', async () => {
    renderTerminal()
    await waitFor(() => { expect(written()).toBe('hello ') })

    act(() => { terminalState.onChunk?.(chunk(4, 'new screen', true)) })

    // The reset travels in the same write as the replay, so nothing queued before it can land after it.
    await waitFor(() => { expect(written()).toBe(`hello ${RESET}new screen`) })
  })

  it('waits for the replay the host asked for over the stream when it holds no history yet', async () => {
    loadTerminalHistory.mockResolvedValue(null)
    renderTerminal()
    await waitFor(() => { expect(loadTerminalHistory).toHaveBeenCalledWith(47) })
    expect(loadTerminalReplay).not.toHaveBeenCalled()

    act(() => { terminalState.onChunk?.(chunk(1, 'replayed', true)) })

    await waitFor(() => { expect(written()).toBe(`${RESET}replayed`) })
  })

  it('asks the daemon for a fresh replay itself when the host has no stream to ask over', async () => {
    loadTerminalHistory.mockRejectedValue(new Error('The terminal is not connected to the daemon'))
    renderTerminal()

    await waitFor(() => { expect(loadTerminalReplay).toHaveBeenCalledWith(binding) })
    await waitFor(() => { expect(written()).toBe('fresh') })
  })

  it('uses the saved font size and applies a change to the open terminal without remounting it', async () => {
    const { rerender } = render(<TerminalView conversation={conversation} desktop fontSize={12} />)

    await waitFor(() => { expect(terminalState.options?.fontSize).toBe(12) })
    const mounted = terminalState.options

    rerender(<TerminalView conversation={conversation} desktop fontSize={15} />)

    expect(terminalState.options).toBe(mounted)
    expect(terminalState.options?.fontSize).toBe(15)
    expect(loadTerminalHistory).toHaveBeenCalledTimes(1)
  })

  it('sends the latest terminal size after an earlier resize finishes', async () => {
    renderTerminal()

    await waitFor(() => { expect(resizeTerminal).toHaveBeenCalledTimes(1) })
    await Promise.resolve()
    resizeTerminal.mockImplementationOnce(() => new Promise<void>((resolve) => { terminalState.resolveResize = resolve }))
    terminalState.cols = 120
    terminalState.rows = 40
    terminalState.onResize?.()
    await waitFor(() => { expect(resizeTerminal).toHaveBeenCalledTimes(2) })

    terminalState.cols = 80
    terminalState.rows = 24
    terminalState.onResize?.()

    terminalState.resolveResize?.()
    await waitFor(() => { expect(resizeTerminal).toHaveBeenLastCalledWith(binding, 80, 24) })
  })

  it('shows one inline resize error and does not toast repeatedly for the same size', async () => {
    resizeTerminal.mockRejectedValue(new Error('Native pane is no longer attached'))
    renderTerminal()

    expect(await screen.findByText('Terminal size could not be updated. Check the daemon connection and retry.')).toBeInTheDocument()
    terminalState.onResize?.()
    terminalState.onResize?.()
    await Promise.resolve()

    expect(resizeTerminal).toHaveBeenCalledTimes(1)
    expect(toastError).not.toHaveBeenCalled()
  })

  it('stops typing and offers to try again when the daemon cannot be reached', async () => {
    sendTerminalInput.mockRejectedValueOnce(new Error('The terminal is not connected to the daemon'))
    renderTerminal()
    await waitFor(() => { expect(terminalState.onData).toBeDefined() })

    terminalState.onData?.('a')

    expect(await screen.findByText('Input stopped because the daemon could not be reached.')).toBeInTheDocument()
    terminalState.onData?.('b')
    expect(sendTerminalInput).toHaveBeenCalledTimes(1)
  })

  it('turns a file name in the output into a link that opens only on Cmd or Ctrl click', async () => {
    const onOpenPath = vi.fn()
    render(<TerminalView conversation={conversation} desktop fontSize={12} onOpenPath={onOpenPath} />)
    await waitFor(() => { expect(terminalState.linkProvider).toBeDefined() })
    terminalState.line = 'error in src/main.rs:42:7 today'

    const links = await new Promise<Array<{ text: string; activate: (event: MouseEvent) => void }> | undefined>((resolve) => { terminalState.linkProvider?.provideLinks(1, resolve) })
    expect(links?.map((link) => link.text)).toEqual(['src/main.rs:42:7'])

    links?.[0]?.activate({ metaKey: false, ctrlKey: false } as MouseEvent)
    expect(onOpenPath).not.toHaveBeenCalled()
    links?.[0]?.activate({ metaKey: false, ctrlKey: true } as MouseEvent)
    expect(onOpenPath).toHaveBeenCalledWith('src/main.rs', 42)
  })

  it('hands the window a find handle while the terminal is open and takes it back when it closes', async () => {
    const onHandle = vi.fn()
    const view = render(<TerminalView conversation={conversation} desktop fontSize={12} onHandle={onHandle} />)
    await waitFor(() => { expect(onHandle).toHaveBeenCalled() })

    const handle = onHandle.mock.calls[0]?.[0] as { find: (query: string, direction: 'next' | 'previous') => boolean }
    expect(handle.find('present', 'next')).toBe(true)
    expect(handle.find('absent', 'next')).toBe(false)
    expect(handle.find('', 'next')).toBe(false)
    handle.find('again', 'previous')
    expect(terminalState.searches).toEqual(['next:present', 'next:absent', 'previous:again'])

    view.unmount()
    expect(onHandle).toHaveBeenLastCalledWith(null)
  })

  it('keeps the cursor steady unless blinking is asked for', async () => {
    render(<TerminalView conversation={conversation} desktop fontSize={13} focused />)
    await waitFor(() => { expect(terminalState.options).toBeDefined() })
    expect(terminalState.options?.cursorBlink).toBe(false)
  })

  it('blinks the cursor only while the window is in front', async () => {
    render(<TerminalView conversation={conversation} desktop fontSize={13} look={{ cursorBlink: true }} focused />)
    await waitFor(() => { expect(terminalState.options?.cursorBlink).toBe(true) })

    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    act(() => { window.dispatchEvent(new Event('blur')) })
    expect(terminalState.options?.cursorBlink).toBe(false)

    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    act(() => { window.dispatchEvent(new Event('focus')) })
    expect(terminalState.options?.cursorBlink).toBe(true)
  })

  it('writes everything that arrives with the history in one write', async () => {
    let resolveHistory: (history: TerminalHistory) => void = () => {}
    loadTerminalHistory.mockReturnValue(new Promise<TerminalHistory>((resolve) => { resolveHistory = resolve }))
    renderTerminal()
    await waitFor(() => { expect(terminalState.onChunk).toBeDefined() })
    act(() => {
      terminalState.onChunk?.(chunk(4, 'one '))
      terminalState.onChunk?.(chunk(5, 'two '))
      terminalState.onChunk?.(chunk(6, 'three'))
    })
    await act(async () => {
      resolveHistory({ seq: 3, bytes: bytes('line1\nline2\nline3 ') })
      await Promise.resolve()
    })

    await waitFor(() => { expect(written()).toBe('line1\nline2\nline3 one two three') })
    expect(terminalState.writes).toHaveLength(1)
  })

  it('writes a replay and the output queued before it as one write', async () => {
    renderTerminal()
    await waitFor(() => { expect(written()).toBe('hello ') })

    act(() => {
      terminalState.onChunk?.(chunk(4, 'dropped'))
      terminalState.onChunk?.(chunk(5, 'new screen', true))
    })

    await waitFor(() => { expect(written()).toBe(`hello ${RESET}new screen`) })
    expect(terminalState.writes).toHaveLength(2)
  })

  it('shows the last screen of a session at once when it is opened again, and rewrites it only if it changed', async () => {
    const first = renderTerminal()
    await waitFor(() => { expect(written()).toBe('hello ') })
    act(() => { terminalState.onChunk?.(chunk(4, 'world')) })
    await waitFor(() => { expect(written()).toBe('hello world') })
    first.unmount()

    terminalState.writes = []
    loadTerminalHistory.mockReturnValue(new Promise<TerminalHistory>(() => {}))
    renderTerminal()

    // The history has not answered, and the screen is already there.
    await waitFor(() => { expect(written()).toBe('hello world') })

    terminalState.writes = []
    loadTerminalHistory.mockResolvedValue({ seq: 4, bytes: bytes('hello world') })
    renderTerminal()
    await waitFor(() => { expect(loadTerminalHistory).toHaveBeenCalledTimes(3) })
    await new Promise((resolve) => { window.setTimeout(resolve, 20) })
    // The kept screen ended where the history does, so only the kept screen was written.
    expect(written()).toBe('hello world')
  })

  it('replaces the last screen with the history when the pane moved on', async () => {
    const first = renderTerminal()
    await waitFor(() => { expect(written()).toBe('hello ') })
    first.unmount()

    terminalState.writes = []
    loadTerminalHistory.mockResolvedValue({ seq: 9, bytes: bytes('newer') })
    renderTerminal()

    await waitFor(() => { expect(written()).toContain(`${RESET}newer`) })
  })

  describe('while attaching', () => {
    beforeEach(() => { vi.useFakeTimers() })
    afterEach(() => { vi.useRealTimers() })

    it('never shows a loader for a pane on this computer', async () => {
      loadTerminalHistory.mockReturnValue(new Promise<TerminalHistory>(() => {}))
      renderTerminal()

      expect(screen.queryByRole('status')).not.toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(2_000) })
      expect(screen.queryByRole('status')).not.toBeInTheDocument()
      expect(screen.queryByText(/Connecting|Catching up/)).not.toBeInTheDocument()
    })

    it('shows the catching-up strip for another machine only after 300 ms, and removes it once live', async () => {
      let resolveHistory: (history: TerminalHistory) => void = () => {}
      loadTerminalHistory.mockReturnValue(new Promise<TerminalHistory>((resolve) => { resolveHistory = resolve }))
      renderTerminal({ ...conversation, machine: 2, machineName: 'devbox' })

      await act(async () => { await vi.advanceTimersByTimeAsync(299) })
      expect(screen.queryByText(/Catching up/)).not.toBeInTheDocument()
      await act(async () => { await vi.advanceTimersByTimeAsync(1) })
      expect(screen.getByRole('status')).toHaveTextContent(/Catching up from devbox · \d+ ms/)

      await act(async () => {
        resolveHistory({ seq: 3, bytes: bytes('hello ') })
        await vi.advanceTimersByTimeAsync(0)
      })
      expect(screen.queryByText(/Catching up/)).not.toBeInTheDocument()
    })

    it('shows nothing for another machine that attaches within 300 ms', async () => {
      renderTerminal({ ...conversation, machine: 2, machineName: 'devbox' })
      await act(async () => { await vi.advanceTimersByTimeAsync(50) })
      await act(async () => { await vi.advanceTimersByTimeAsync(1_000) })

      expect(screen.queryByText(/Catching up/)).not.toBeInTheDocument()
    })
  })

  it('says a chat session has no terminal instead of offering to switch it', () => {
    renderTerminal({ ...conversation, execution: 'Structured', nativePane: null })

    expect(screen.getByRole('heading', { name: 'This is a chat session' })).toBeInTheDocument()
    expect(screen.queryByRole('button')).not.toBeInTheDocument()
  })

  it('explains that a native terminal needs recovery when no pane is attached', () => {
    renderTerminal({ ...conversation, nativePane: null })

    expect(screen.getByRole('heading', { name: 'Native terminal needs recovery' })).toBeInTheDocument()
    expect(screen.queryByRole('region', { name: 'Native Codex terminal' })).not.toBeInTheDocument()
  })
})

function renderTerminal(selected = conversation, fontSize = 12) {
  return render(<TerminalView conversation={selected} desktop fontSize={fontSize} />)
}

function renderPaused(paused: boolean) {
  return <TerminalView conversation={conversation} desktop fontSize={12} paused={paused} />
}
