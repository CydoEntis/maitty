import { act, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import type { TerminalChunk, TerminalHistory } from '@/api/terminalStream'
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
  },
  toastError: vi.fn(),
}))

vi.mock('sonner', () => ({ toast: { error: toastError } }))

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
    write(data: Uint8Array) { terminalState.writes.push(data) }
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
const chunk = (seq: number, text: string, replay = false): TerminalChunk => ({ term: 47, seq, replay, bytes: bytes(text) })
const written = () => terminalState.writes.map((part) => new TextDecoder().decode(part)).join('')

describe('terminal view', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    terminalState.writes = []
    terminalState.resets = 0
    terminalState.onData = undefined
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

    await waitFor(() => { expect(written()).toBe('hello new screen') })
    expect(terminalState.resets).toBe(1)
  })

  it('waits for the replay the host asked for over the stream when it holds no history yet', async () => {
    loadTerminalHistory.mockResolvedValue(null)
    renderTerminal()
    await waitFor(() => { expect(loadTerminalHistory).toHaveBeenCalledWith(47) })
    expect(loadTerminalReplay).not.toHaveBeenCalled()

    act(() => { terminalState.onChunk?.(chunk(1, 'replayed', true)) })

    await waitFor(() => { expect(written()).toBe('replayed') })
    expect(terminalState.resets).toBe(1)
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

  it('blinks the cursor only while the window is in front', async () => {
    render(<TerminalView conversation={conversation} desktop fontSize={13} focused />)
    await waitFor(() => { expect(terminalState.options?.cursorBlink).toBe(true) })

    vi.spyOn(document, 'hasFocus').mockReturnValue(false)
    act(() => { window.dispatchEvent(new Event('blur')) })
    expect(terminalState.options?.cursorBlink).toBe(false)

    vi.spyOn(document, 'hasFocus').mockReturnValue(true)
    act(() => { window.dispatchEvent(new Event('focus')) })
    expect(terminalState.options?.cursorBlink).toBe(true)
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
