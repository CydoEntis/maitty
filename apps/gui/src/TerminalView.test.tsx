import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { TerminalView } from './TerminalView'

const { loadTerminalReplay, loadTerminalScreen, resizeTerminal, sendTerminalInput, terminalState, toastError } = vi.hoisted(() => ({
  loadTerminalScreen: vi.fn<(conversation: Pick<Conversation, 'id'> & { generation: number }) => Promise<string>>(),
  loadTerminalReplay: vi.fn<(conversation: Pick<Conversation, 'id'> & { generation: number }) => Promise<Uint8Array>>(),
  resizeTerminal: vi.fn<(conversation: Pick<Conversation, 'id'> & { generation: number }, cols: number, rows: number) => Promise<void>>(),
  sendTerminalInput: vi.fn<(conversation: Pick<Conversation, 'id'> & { generation: number }, input: string | Uint8Array) => Promise<string>>(),
  terminalState: {
    writes: [] as Array<string | Uint8Array>,
    onData: undefined as ((data: string) => void) | undefined,
    onResize: undefined as (() => void) | undefined,
    resolveResize: undefined as (() => void) | undefined,
    cols: 80,
    rows: 24,
  },
  toastError: vi.fn(),
}))

vi.mock('sonner', () => ({ toast: { error: toastError } }))

vi.mock('@/api/daemon', () => ({
  isDesktop: () => true,
  loadTerminalReplay,
  loadTerminalScreen,
  resizeTerminal,
  sendTerminalInput,
}))

vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    get cols() { return terminalState.cols }
    get rows() { return terminalState.rows }
    loadAddon() {}
    open() {}
    onData(callback: (data: string) => void) {
      terminalState.onData = callback
      return { dispose: vi.fn() }
    }
    onBinary() { return { dispose: vi.fn() } }
    write(data: string | Uint8Array, callback?: () => void) {
      terminalState.writes.push(data)
      callback?.()
    }
    dispose() {}
  },
}))

vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }))

const conversation: Conversation = {
  id: '47',
  generation: 2,
  title: 'Resume Codex in its native terminal',
  project: 'maitty',
  folder: 'C:/dev/maitty',
  branch: 'gui/native-terminal',
  execution: 'Native',
  provider: 'Codex',
  status: 'idle',
  updated: null,
  model: 'Codex default',
  permission: 'Provider permissions',
  context: null,
  nativePane: 47,
  hasDiff: false,
}

const binding = { id: conversation.id, generation: conversation.generation }

describe('native terminal view', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    terminalState.writes = []
    terminalState.onData = undefined
    terminalState.onResize = undefined
    terminalState.resolveResize = undefined
    terminalState.cols = 80
    terminalState.rows = 24
    vi.stubGlobal('ResizeObserver', class {
      private readonly callback: () => void
      constructor(callback: () => void) { this.callback = callback }
      observe() {
        terminalState.onResize = this.callback
        this.callback()
      }
      disconnect() {}
    })
    loadTerminalScreen.mockResolvedValue('\u001b[0m\u001b[2J\u001b[HCodex terminal ready')
    loadTerminalReplay.mockResolvedValue(Uint8Array.from([27, 91, 50, 74, 67, 111, 100, 101, 120]))
    resizeTerminal.mockResolvedValue()
    sendTerminalInput.mockResolvedValue('\u001b[0m\u001b[2J\u001b[HCodex terminal ready')
  })

  afterEach(() => { vi.unstubAllGlobals() })

  it('renders ANSI screen snapshots and sends terminal-emulator input to the exact conversation', async () => {
    renderTerminal()

    expect(await screen.findByRole('region', { name: 'Native Codex terminal' })).toBeInTheDocument()
    await waitFor(() => { expect(loadTerminalReplay).toHaveBeenCalledWith(binding) })
    await waitFor(() => { expect(terminalState.writes.some((write) => write instanceof Uint8Array)).toBe(true) })
    await waitFor(() => { expect(terminalState.writes).toContain('\u001b[0m\u001b[2J\u001b[HCodex terminal ready') })
    expect(resizeTerminal).toHaveBeenCalledWith(binding, 80, 24)

    terminalState.onData?.('\u0003')
    await waitFor(() => { expect(sendTerminalInput).toHaveBeenCalledWith(binding, '\u0003') })
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

function renderTerminal(selected = conversation) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}><TerminalView conversation={selected} desktop /></QueryClientProvider>)
}
