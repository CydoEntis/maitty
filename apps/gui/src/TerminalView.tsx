import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { CircleAlert, LoaderCircle, TerminalSquare } from 'lucide-react'
import { toast } from 'sonner'
import type { Terminal as XtermTerminal } from '@xterm/xterm'
import { Button } from '@/components/ui/button'
import { handoffToNative, isDesktop, loadTerminalReplay, loadTerminalScreen, resizeTerminal, sendTerminalInput } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import '@xterm/xterm/css/xterm.css'

const TERMINAL_REFRESH_INTERVAL_MS = 250
const TERMINAL_SCROLLBACK_LINES = 5000

type TerminalBinding = Pick<Conversation, 'id'> & { generation: number }

type TerminalViewProps = {
  conversation: Conversation
  desktop: boolean
}

type NativeConversation = Conversation & { generation: number; nativePane: number }

export function TerminalView({ conversation, desktop }: TerminalViewProps) {
  if (!desktop) return null
  if (!isDesktop()) return <div className="panel-empty"><TerminalSquare size={18} /><h2>Native terminal is not available</h2><p>Open Maitty Desktop to use the daemon-owned terminal.</p></div>
  if (!hasNativeTerminal(conversation)) return <NativeHandoffView conversation={conversation} />

  return <ConnectedTerminalView conversation={conversation} />
}

function NativeHandoffView({ conversation }: { conversation: Conversation }) {
  const queryClient = useQueryClient()
  const [isHandingOff, setIsHandingOff] = useState(false)
  const canHandOff = conversation.status === 'idle' || conversation.status === 'done'

  if (conversation.execution === 'Native') {
    return <div className="panel-empty" role="status">
      <CircleAlert size={18} />
      <h2>Native terminal needs recovery</h2>
      <p>The daemon no longer has the native {conversation.provider} terminal attached. Resume requires checking that its process has stopped before chat can take ownership.</p>
    </div>
  }

  async function handOff() {
    setIsHandingOff(true)
    try {
      await handoffToNative(conversation)
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['conversations'] }),
        queryClient.invalidateQueries({ queryKey: ['conversation-history', conversation.id] }),
      ])
      toast.success('Conversation resumed in the native terminal')
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Could not hand off this conversation')
    } finally {
      setIsHandingOff(false)
    }
  }

  return <div className="panel-empty">
    <TerminalSquare size={18} />
    <h2>Continue in the native terminal</h2>
    <p>Maitty will save the handoff, stop the chat runtime, then resume this exact {conversation.provider} session in a daemon-owned terminal.</p>
    <Button variant="outline" size="sm" disabled={!canHandOff || isHandingOff} onClick={() => { void handOff() }}>
      {isHandingOff ? <LoaderCircle size={14} /> : <TerminalSquare size={14} />}
      {isHandingOff ? 'Handing off' : 'Resume in native terminal'}
    </Button>
  </div>
}

function ConnectedTerminalView({ conversation }: { conversation: NativeConversation }) {
  const term = conversation.nativePane
  const binding: TerminalBinding = useMemo(() => ({ id: conversation.id, generation: conversation.generation }), [conversation.id, conversation.generation])
  const queryClient = useQueryClient()
  const inputQueue = useRef(Promise.resolve())
  const terminalHost = useRef<HTMLDivElement>(null)
  const terminal = useRef<XtermTerminal | null>(null)
  const inputFailedRef = useRef(false)
  const [inputFailed, setInputFailed] = useState(false)
  const replayQuery = useQuery({
    queryKey: ['terminal-replay', binding.id, binding.generation],
    queryFn: () => loadTerminalReplay(binding),
    staleTime: Infinity,
  })
  const screenQuery = useQuery({
    queryKey: ['terminal-screen', binding.id, binding.generation],
    queryFn: () => loadTerminalScreen(binding),
    enabled: replayQuery.isSuccess,
    refetchInterval: TERMINAL_REFRESH_INTERVAL_MS,
  })

  const send = useCallback((data: string | Uint8Array) => {
    if (!data || inputFailedRef.current) return
    inputQueue.current = inputQueue.current.then(async () => {
      const screen = await sendTerminalInput(binding, data)
      terminal.current?.write(screen)
      inputFailedRef.current = false
      setInputFailed(false)
      queryClient.setQueryData(['terminal-screen', binding.id, binding.generation], screen)
    }).catch(() => {
      inputFailedRef.current = true
      setInputFailed(true)
      toast.error('Terminal input could not reach Maitty. Reconnect before typing again.')
    })
  }, [binding, queryClient])

  useEffect(() => {
    const host = terminalHost.current
    const replay = replayQuery.data
    if (!host || !replay) return
    const hostElement = host
    const replayBytes = replay.slice()
    let isMounted = true
    let emulator: XtermTerminal | null = null
    let resizeObserver: ResizeObserver | null = null
    let disposeInput: (() => void) | null = null
    let disposeBinary: (() => void) | null = null

    async function mountTerminal() {
      try {
        const [{ Terminal: TerminalEmulator }, { FitAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
        ])
        if (!isMounted) return

        emulator = new TerminalEmulator({
          cursorBlink: true,
          fontFamily: '"Geist Mono", monospace',
          fontSize: 12,
          scrollback: TERMINAL_SCROLLBACK_LINES,
          screenReaderMode: true,
          theme: { background: '#060d16', foreground: '#c9d6e3', cursor: '#3dd6c6' },
        })
        const terminalInstance = emulator
        const fit = new FitAddon()
        terminalInstance.loadAddon(fit)
        terminalInstance.open(hostElement)
        terminal.current = terminalInstance

        let resizeInFlight = false
        let resizePending = false
        let lastSize = ''
        const fitAndResize = () => {
          fit.fit()
          const cols = terminalInstance.cols
          const rows = terminalInstance.rows
          const size = `${String(cols)}x${String(rows)}`
          if (resizeInFlight) {
            resizePending = true
            return
          }
          if (size === lastSize) return
          resizeInFlight = true
          resizePending = false
          void resizeTerminal(binding, cols, rows)
            .then(() => { lastSize = size })
            .catch((error: unknown) => {
              resizePending = false
              inputFailedRef.current = true
              setInputFailed(true)
              toast.error(error instanceof Error ? error.message : 'Could not resize the native terminal')
            })
            .finally(() => {
              resizeInFlight = false
              if (resizePending && isMounted) {
                resizePending = false
                fitAndResize()
              }
            })
        }
        terminalInstance.write(replayBytes, () => {
          if (!isMounted) return
          resizeObserver = new ResizeObserver(fitAndResize)
          resizeObserver.observe(hostElement)
          fitAndResize()

          const inputSubscription = terminalInstance.onData(send)
          const binarySubscription = terminalInstance.onBinary((data) => {
            send(Uint8Array.from(data, (character) => character.charCodeAt(0)))
          })
          disposeInput = () => { inputSubscription.dispose() }
          disposeBinary = () => { binarySubscription.dispose() }
        })
      } catch (error) {
        if (isMounted) {
          setInputFailed(true)
          toast.error(error instanceof Error ? error.message : 'Could not start the terminal emulator')
        }
      }
    }

    void mountTerminal()
    return () => {
      isMounted = false
      resizeObserver?.disconnect()
      disposeInput?.()
      disposeBinary?.()
      emulator?.dispose()
      terminal.current = null
    }
  }, [binding, replayQuery.data, send])

  useEffect(() => {
    if (screenQuery.data) terminal.current?.write(screenQuery.data)
  }, [screenQuery.data])

  async function reconnectTerminal() {
    const result = await screenQuery.refetch()
    if (result.isSuccess) {
      inputFailedRef.current = false
      setInputFailed(false)
    }
  }

  return <div className="terminal-view">
    <div className="terminal-heading"><span className="terminal-live-dot" />Native {conversation.provider} terminal · pane {term}<span className="terminal-mode-hint">Keys go straight to {conversation.provider} · Ctrl+` to return</span></div>
    {(replayQuery.isLoading || screenQuery.isLoading) && <div className="terminal-load-status" role="status"><LoaderCircle size={14} />Connecting to terminal</div>}
    {(replayQuery.isError || screenQuery.isError) && <div className="terminal-input-error" role="status"><p>Could not load the terminal. Check the daemon connection and retry.</p><Button variant="outline" size="sm" onClick={() => { void (replayQuery.isError ? replayQuery.refetch() : screenQuery.refetch()) }}>Retry</Button></div>}
    <div className="terminal-screen" ref={terminalHost} role="region" aria-label={`Native ${conversation.provider} terminal`} aria-describedby="terminal-instructions" />
    <p className="terminal-footnote" id="terminal-instructions">Click the terminal and type. Maitty sends keys to the selected daemon-owned pane.</p>
    {inputFailed && <div className="terminal-input-error" role="status"><p>Input stopped because the daemon could not be reached. Retry the terminal connection before typing.</p><Button variant="outline" size="sm" onClick={() => { void reconnectTerminal() }}>Reconnect</Button></div>}
  </div>
}

function hasNativeTerminal(conversation: Conversation): conversation is NativeConversation {
  return typeof conversation.generation === 'number' && typeof conversation.nativePane === 'number'
}
