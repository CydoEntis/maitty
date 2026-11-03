import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { CircleAlert, LoaderCircle, TerminalSquare } from 'lucide-react'
import { toast } from 'sonner'
import type { ITheme, Terminal as XtermTerminal } from '@xterm/xterm'
import { Button } from '@/components/ui/button'
import { isDesktop, loadTerminalReplay, resizeTerminal, sendTerminalInput } from '@/api/daemon'
import { listenTerminalChunks, loadTerminalHistory, type TerminalChunk } from '@/api/terminalStream'
import type { Conversation } from '@/data/preview'
import type { TerminalHandle } from '@/features/terminal/handle'
import { findPathLinks } from '@/features/terminal/pathLinks'
import '@xterm/xterm/css/xterm.css'

const DEFAULT_SCROLLBACK_LINES = 10000
const LINE_HEIGHT = 1.6
const FONT_FAMILY = '"Geist Mono Variable", "Geist Mono", ui-monospace, monospace'
const DEFAULT_THEME: ITheme = { background: '#060d16', foreground: '#c9d6e3', cursor: '#3dd6c6' }

type TerminalBinding = Pick<Conversation, 'id'> & { generation: number }

/** How a pane renders; each one is applied to the open terminal in place, without remounting. */
export type TerminalLook = {
  cursorStyle?: 'block' | 'bar' | 'underline'
  scrollback?: number
  screenReader?: boolean
  theme?: ITheme
}

type TerminalViewProps = {
  conversation: Conversation
  desktop: boolean
  fontSize: number
  look?: TerminalLook
  /** The accessible name of the terminal region. */
  label?: string
  /** Typing is held while the daemon is disconnected. */
  inputDisabled?: boolean
  /** The session is gone: the output stays, and nothing more is asked of the daemon. */
  ended?: boolean
  /** Moves keyboard focus into the terminal when it becomes true. */
  focused?: boolean
  /** Told what the user types here, so a window can mirror it to other panes. */
  onInput?: (data: string | Uint8Array) => void
  /** A file path in the output was Cmd/Ctrl-clicked. */
  onOpenPath?: (path: string, line?: number) => void
  /** Receives the open terminal's controls, and null when it closes. */
  onHandle?: (handle: TerminalHandle | null) => void
}

type NativeConversation = Conversation & { generation: number; nativePane: number }

/** What Shift+Enter sends: Alt+Enter, which Claude Code and Codex take as a new line in their prompt. */
const NEW_LINE_KEYS = '\x1b\r'

export function TerminalView({ conversation, desktop, fontSize, look = {}, label, inputDisabled = false, ended = false, focused = false, onInput, onOpenPath, onHandle }: TerminalViewProps) {
  if (!desktop) return null
  if (!isDesktop()) return <div className="panel-empty"><TerminalSquare size={18} /><h2>Native terminal is not available</h2><p>Open Maitty Desktop to use the daemon-owned terminal.</p></div>
  if (!hasNativeTerminal(conversation)) return <NativeHandoffView conversation={conversation} />

  return <ConnectedTerminalView conversation={conversation} fontSize={fontSize} look={look} label={label ?? `Native ${conversation.provider} terminal`} inputDisabled={inputDisabled || ended} ended={ended} focused={focused} onInput={onInput} onOpenPath={onOpenPath} onHandle={onHandle} />
}

function NativeHandoffView({ conversation }: { conversation: Conversation }) {
  if (conversation.execution === 'Native') {
    return <div className="panel-empty" role="status">
      <CircleAlert size={18} />
      <h2>Native terminal needs recovery</h2>
      <p>The daemon no longer has the native {conversation.provider} terminal attached. Reopen it from the terminal app.</p>
    </div>
  }

  return <div className="panel-empty" role="status">
    <TerminalSquare size={18} />
    <h2>This is a chat session</h2>
    <p>A chat has no terminal. Start a terminal session to work in one.</p>
  </div>
}

type ConnectedProps = { conversation: NativeConversation; fontSize: number; look: TerminalLook; label: string; inputDisabled: boolean; ended: boolean; focused: boolean; onInput: ((data: string | Uint8Array) => void) | undefined; onOpenPath: ((path: string, line?: number) => void) | undefined; onHandle: ((handle: TerminalHandle | null) => void) | undefined }

function ConnectedTerminalView({ conversation, fontSize, look, label, inputDisabled, ended, focused, onInput, onOpenPath, onHandle }: ConnectedProps) {
  const term = conversation.nativePane
  const binding: TerminalBinding = useMemo(() => ({ id: conversation.id, generation: conversation.generation }), [conversation.id, conversation.generation])
  const inputQueue = useRef(Promise.resolve())
  const terminalHost = useRef<HTMLDivElement>(null)
  const terminal = useRef<XtermTerminal | null>(null)
  const inputFailedRef = useRef(false)
  const fontSizeRef = useRef(fontSize)
  const lookRef = useRef(look)
  const inputDisabledRef = useRef(inputDisabled)
  const focusedRef = useRef(focused)
  const endedRef = useRef(ended)
  useEffect(() => { endedRef.current = ended })
  const onInputRef = useRef(onInput)
  useEffect(() => { onInputRef.current = onInput })
  const onOpenPathRef = useRef(onOpenPath)
  const onHandleRef = useRef(onHandle)
  useEffect(() => {
    onOpenPathRef.current = onOpenPath
    onHandleRef.current = onHandle
  })
  const refitTerminal = useRef<(() => void) | null>(null)
  const [connecting, setConnecting] = useState(true)
  const [loadError, setLoadError] = useState(false)
  const [inputFailed, setInputFailed] = useState(false)
  const [resizeError, setResizeError] = useState('')

  const send = useCallback((data: string | Uint8Array) => {
    if (!data || inputFailedRef.current || inputDisabledRef.current) return
    onInputRef.current?.(data)
    inputQueue.current = inputQueue.current.then(async () => {
      // The daemon echoes what was typed through the stream, so there is nothing to write here.
      await sendTerminalInput(binding, data)
    }).catch(() => {
      inputFailedRef.current = true
      setInputFailed(true)
    })
  }, [binding])

  useEffect(() => {
    const host = terminalHost.current
    if (!host) return
    const hostElement = host
    let isMounted = true
    // Read through a function: after the first check TypeScript would treat isMounted as always true.
    const stillMounted = () => isMounted
    let emulator: XtermTerminal | null = null
    let resizeObserver: ResizeObserver | null = null
    let stopListening: (() => void) | null = null
    let disposeInput: (() => void) | null = null
    let disposeBinary: (() => void) | null = null
    let disposeCopy: (() => void) | null = null
    let frame = 0
    let queued: Uint8Array[] = []

    // Output is written once per animation frame, so a burst repaints once instead of per chunk.
    const flush = () => {
      frame = 0
      if (!emulator || queued.length === 0) return
      const bytes = new Uint8Array(queued.reduce((total, chunk) => total + chunk.length, 0))
      let offset = 0
      for (const chunk of queued) {
        bytes.set(chunk, offset)
        offset += chunk.length
      }
      queued = []
      emulator.write(bytes)
    }
    const enqueue = (bytes: Uint8Array) => {
      queued.push(bytes)
      if (frame === 0) frame = requestAnimationFrame(flush)
    }

    async function mountTerminal() {
      try {
        const [{ Terminal: TerminalEmulator }, { FitAddon }, { SearchAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
          import('@xterm/addon-search'),
        ])
        if (!isMounted) return

        emulator = new TerminalEmulator({
          allowTransparency: true,
          cursorBlink: focusedRef.current,
          cursorStyle: lookRef.current.cursorStyle ?? 'block',
          disableStdin: inputDisabledRef.current,
          fontFamily: FONT_FAMILY,
          fontSize: fontSizeRef.current,
          lineHeight: LINE_HEIGHT,
          scrollback: lookRef.current.scrollback ?? DEFAULT_SCROLLBACK_LINES,
          screenReaderMode: lookRef.current.screenReader ?? false,
          theme: lookRef.current.theme ?? DEFAULT_THEME,
        })
        const terminalInstance = emulator
        const fit = new FitAddon()
        terminalInstance.loadAddon(fit)
        const search = new SearchAddon()
        terminalInstance.loadAddon(search)
        terminalInstance.open(hostElement)
        terminal.current = terminalInstance
        // Cmd/Ctrl-click on a file name in the output opens it in the Inspect panel.
        terminalInstance.registerLinkProvider({
          provideLinks: (y, callback) => {
            const text = terminalInstance.buffer.active.getLine(y - 1)?.translateToString(true)
            if (!text || !onOpenPathRef.current) {
              callback(undefined)
              return
            }
            const links = findPathLinks(text).map((link) => ({
              range: { start: { x: link.start + 1, y }, end: { x: link.end, y } },
              text: text.slice(link.start, link.end),
              activate: (event: MouseEvent) => {
                if (event.metaKey || event.ctrlKey) onOpenPathRef.current?.(link.path, link.line)
              },
            }))
            callback(links.length > 0 ? links : undefined)
          },
        })
        onHandleRef.current?.({
          find: (query, direction) => (query === '' ? false : direction === 'next' ? search.findNext(query) : search.findPrevious(query)),
          clearFind: () => {
            search.clearDecorations()
            terminalInstance.clearSelection()
          },
          focus: () => { terminalInstance.focus() },
          paste: (text) => { terminalInstance.paste(text) },
        })

        let resizeInFlight = false
        let resizePending = false
        let lastSize = ''
        let lastFailedSize = ''
        const fitAndResize = () => {
          if (!isMounted) return
          fit.fit()
          // An ended session has no terminal left to resize.
          if (endedRef.current) return
          const cols = terminalInstance.cols
          const rows = terminalInstance.rows
          if (cols < 1 || rows < 1) return
          const size = `${String(cols)}x${String(rows)}`
          if (resizeInFlight) {
            resizePending = true
            return
          }
          if (size === lastSize || size === lastFailedSize) return
          resizeInFlight = true
          resizePending = false
          void resizeTerminal(binding, cols, rows)
            .then(() => {
              lastSize = size
              lastFailedSize = ''
              setResizeError('')
            })
            .catch(() => {
              resizePending = false
              if (!isMounted) return
              lastFailedSize = size
              setResizeError('Terminal size could not be updated. Check the daemon connection and retry.')
            })
            .finally(() => {
              resizeInFlight = false
              if (resizePending && isMounted) {
                resizePending = false
                fitAndResize()
              }
            })
        }
        refitTerminal.current = () => {
          lastFailedSize = ''
          fitAndResize()
        }

        // Follow the live stream first, hold what arrives while the history loads, then write
        // the history and continue from the next chunk: no gap and no repeat.
        let lastSeq = 0
        let caughtUp = false
        const pending: TerminalChunk[] = []
        const apply = (chunk: TerminalChunk) => {
          if (chunk.seq <= lastSeq) return
          lastSeq = chunk.seq
          if (chunk.replay) {
            flush()
            terminalInstance.reset()
          }
          enqueue(chunk.bytes)
        }
        stopListening = await listenTerminalChunks(term, (chunk) => {
          if (caughtUp) apply(chunk)
          else pending.push(chunk)
        })
        if (!stillMounted()) {
          stopListening()
          return
        }
        const history = await loadTerminalHistory(term)
        if (!stillMounted()) return
        if (history) {
          lastSeq = history.seq
          enqueue(history.bytes)
        } else {
          // The shell dropped an overlong history: ask the daemon for a fresh replay instead.
          enqueue(await loadTerminalReplay(binding))
          if (!stillMounted()) return
        }
        caughtUp = true
        for (const chunk of pending) apply(chunk)
        setConnecting(false)

        resizeObserver = new ResizeObserver(fitAndResize)
        resizeObserver.observe(hostElement)
        fitAndResize()
        // Geist Mono may still be loading; measure again once it is, so the grid fits the font.
        const fonts = (document as { fonts?: FontFaceSet }).fonts
        void fonts?.ready.then(() => { refitTerminal.current?.() })

        // A terminal sends the same byte for Shift+Enter as for Enter. Agents read ESC then Enter (Alt+Enter) as "new line", so that is what goes.
        terminalInstance.attachCustomKeyEventHandler((event) => {
          if (event.key !== 'Enter' || !event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return true
          if (event.type === 'keydown') send(NEW_LINE_KEYS)
          return false
        })
        const inputSubscription = terminalInstance.onData(send)
        const binarySubscription = terminalInstance.onBinary((data) => {
          send(Uint8Array.from(data, (character) => character.charCodeAt(0)))
        })
        disposeInput = () => { inputSubscription.dispose() }
        disposeBinary = () => { binarySubscription.dispose() }

        // Selecting text copies it, once the mouse is released (not on every drag step).
        const copySelection = () => {
          if (!terminalInstance.hasSelection()) return
          navigator.clipboard.writeText(terminalInstance.getSelection()).then(
            () => { toast.success('Copied', { id: 'terminal-copy', duration: 1500 }) },
            () => { toast.error('Could not copy the selection') },
          )
        }
        hostElement.addEventListener('mouseup', copySelection)
        disposeCopy = () => { hostElement.removeEventListener('mouseup', copySelection) }
      } catch (error) {
        if (isMounted) {
          setLoadError(true)
          setConnecting(false)
          toast.error(error instanceof Error ? error.message : 'Could not start the terminal emulator')
        }
      }
    }

    void mountTerminal()
    return () => {
      isMounted = false
      onHandleRef.current?.(null)
      if (frame !== 0) cancelAnimationFrame(frame)
      refitTerminal.current = null
      stopListening?.()
      resizeObserver?.disconnect()
      disposeInput?.()
      disposeBinary?.()
      disposeCopy?.()
      emulator?.dispose()
      terminal.current = null
    }
  }, [binding, term, send])

  // The emulator is created once per pane, so a font change is applied to it in place.
  useEffect(() => {
    fontSizeRef.current = fontSize
    const emulator = terminal.current
    if (!emulator || emulator.options.fontSize === fontSize) return
    emulator.options.fontSize = fontSize
    refitTerminal.current?.()
  }, [fontSize])

  useEffect(() => {
    lookRef.current = look
    const emulator = terminal.current
    if (!emulator) return
    if (look.cursorStyle && emulator.options.cursorStyle !== look.cursorStyle) emulator.options.cursorStyle = look.cursorStyle
    if (look.scrollback && emulator.options.scrollback !== look.scrollback) emulator.options.scrollback = look.scrollback
    if (look.screenReader !== undefined && emulator.options.screenReaderMode !== look.screenReader) emulator.options.screenReaderMode = look.screenReader
    if (look.theme && emulator.options.theme !== look.theme) emulator.options.theme = look.theme
  }, [look])

  useEffect(() => {
    inputDisabledRef.current = inputDisabled
    const emulator = terminal.current
    if (emulator) emulator.options.disableStdin = inputDisabled
  }, [inputDisabled])

  useEffect(() => {
    focusedRef.current = focused
    // A blinking cursor repaints its pane twice a second, so only the pane you are in blinks.
    if (terminal.current) terminal.current.options.cursorBlink = focused
    if (focused && !connecting) terminal.current?.focus()
  }, [focused, connecting])

  return <div className="terminal-view">
    {connecting && !loadError && <div className="terminal-load-status" role="status"><LoaderCircle size={14} />Connecting to terminal</div>}
    {loadError && <div className="terminal-input-error" role="status"><p>Could not load the terminal. Check the daemon connection and reopen this session.</p></div>}
    <div className="terminal-screen" ref={terminalHost} role="region" aria-label={label} />
    {inputFailed && <div className="terminal-input-error" role="status"><p>Input stopped because the daemon could not be reached.</p><Button variant="outline" size="sm" onClick={() => { inputFailedRef.current = false; setInputFailed(false) }}>Try typing again</Button></div>}
    {resizeError && <div className="terminal-input-error" role="status"><p>{resizeError}</p><Button variant="outline" size="sm" onClick={() => { refitTerminal.current?.() }}>Retry resize</Button></div>}
  </div>
}

function hasNativeTerminal(conversation: Conversation): conversation is NativeConversation {
  return typeof conversation.generation === 'number' && typeof conversation.nativePane === 'number'
}
