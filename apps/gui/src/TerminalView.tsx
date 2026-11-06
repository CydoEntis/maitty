import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { CircleAlert, LoaderCircle, TerminalSquare } from 'lucide-react'
import { toast } from 'sonner'
import type { ITheme, Terminal as XtermTerminal } from '@xterm/xterm'
import type { WebglAddon } from '@xterm/addon-webgl'
import { Button } from '@/components/ui/button'
import { isDesktop, loadTerminalReplay, resizeTerminal, sendTerminalInput } from '@/api/daemon'
import { listenTerminalChunks, loadTerminalHistory, type TerminalChunk, type TerminalHistory } from '@/api/terminalStream'
import type { Conversation } from '@/data/preview'
import type { TerminalHandle } from '@/features/terminal/handle'
import { isPasteChord, listenForImagePaste } from '@/features/terminal/clipboardImage'
import { findPathLinks } from '@/features/terminal/pathLinks'
import { appendToScreen, cachedScreen, replaceScreen, screenKey } from '@/features/terminal/screenCache'
import { isWindowActive, useWindowActive } from '@/lib/window-activity'
import { hasWebgl2 } from '@/lib/gpu'
import { oscColor } from '@/lib/osc-color'
import { beginFlush, countKeyReply, forgetRenderer, reportRenderer } from '@/lib/perf-meter'
import { DEFAULT_FONT_WEIGHT, DEFAULT_FONT_WEIGHT_BOLD, fontStackFor, type TerminalFontWeight } from '@/lib/terminalFont'
import '@xterm/xterm/css/xterm.css'

const DEFAULT_SCROLLBACK_LINES = 10000
const DEFAULT_SCROLL_SPEED = 1
const LINE_HEIGHT = 1.6
const DEFAULT_THEME: ITheme = { background: '#060d16', foreground: '#c9d6e3', cursor: '#3dd6c6' }
const TRANSPARENT = 'rgba(0,0,0,0)'
/**
 * Writes land at most this often: every write redraws the terminal, and under a steady stream the
 * host sends a batch every 12 ms, so without a gap the pane would redraw on every frame. A key's
 * echo waits at most this long, and only while output is already streaming.
 */
const FLUSH_GAP_MS = 32
const NO_MINIMUM_CONTRAST = 1

type Renderer = 'webgl' | 'dom'

/**
 * The pane's colours. A see-through pane (Settings › Appearance, WebGL only) lets the card show
 * behind the text; otherwise the theme's opaque terminal ground, which the DOM renderer needs
 * since each row it repaints would blend through the card's gradient and shadow
 * (docs/PERFORMANCE.md).
 */
function terminalTheme(look: TerminalLook, seeThrough: boolean): ITheme {
  const theme = look.theme ?? DEFAULT_THEME
  return seeThrough ? { ...theme, background: TRANSPARENT } : theme
}

/** Only WebGL can afford a see-through canvas; the DOM renderer always sits on the opaque ground. */
function seeThroughOf(look: TerminalLook, renderer: Renderer): boolean {
  return renderer === 'webgl' && (look.seeThrough ?? false)
}

/** The slots a program's colour query names (`ESC ] 11 ; ? ST` asks for the background). */
const OSC_FOREGROUND = 10
const OSC_BACKGROUND = 11
const OSC_QUERY = '?'

/**
 * Draws the terminal with WebGL, which paints rows on the GPU instead of rebuilding them as DOM
 * elements. False when the addon could not start; `onContextLoss` fires if the GPU context goes
 * away later, once the addon has let go of the terminal.
 */
async function loadWebglRenderer(terminal: XtermTerminal, onContextLoss: () => void): Promise<boolean> {
  let addon: WebglAddon
  try {
    const { WebglAddon } = await import('@xterm/addon-webgl')
    addon = new WebglAddon()
    terminal.loadAddon(addon)
  } catch (error) {
    console.warn('The terminal falls back to the DOM renderer: WebGL could not start', error)
    return false
  }
  addon.onContextLoss(() => {
    addon.dispose()
    onContextLoss()
  })
  return true
}

type TerminalBinding = Pick<Conversation, 'id'> & { generation: number }

/** How a pane renders; each one is applied to the open terminal in place, without remounting. */
export type TerminalLook = {
  cursorStyle?: 'block' | 'bar' | 'underline'
  /** One family name put in front of the bundled font; empty or absent is the bundled font alone. */
  fontFamily?: string
  fontWeight?: TerminalFontWeight
  fontWeightBold?: TerminalFontWeight
  scrollback?: number
  /** Lines per wheel notch as a multiple of xterm's default. */
  scrollSpeed?: number
  /** The card shows behind the text (WebGL only); programs' own backgrounds then show as boxes. */
  seeThrough?: boolean
  /** xterm's own cursor blink, for the focused pane only. Off unless asked for. */
  cursorBlink?: boolean
  /** Draw with the DOM renderer even where WebGL 2 exists (Settings › Performance). Read when the view opens. */
  plainDrawing?: boolean
  screenReader?: boolean
  theme?: ITheme
  /** xterm's own floor for the contrast of any foreground a program picks; 1 leaves colours as asked. */
  minimumContrast?: number
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
  /** The clipboard held an image and no text when the user pasted here. */
  onPasteImage?: (image: File) => void
}

type NativeConversation = Conversation & { generation: number; nativePane: number }

/** After typing failed, the next key waits this long before it tries the daemon again. */
const INPUT_RETRY_MS = 1500

/** How long a view waits for the replay the host asked the daemon for before it fetches one itself. */
const REPLAY_WAIT_MS = 3000

/** A remote attach slower than this shows the "Catching up" strip; a local one never shows anything. */
const CATCH_UP_DELAY_MS = 300
/** How often the strip's milliseconds update while it shows. */
const CATCH_UP_TICK_MS = 100

/**
 * Full reset (RIS) sent in the output itself. xterm's `reset()` takes effect at once, ahead of
 * output still waiting to be parsed, so a screen that replaces another goes through the same write.
 */
const RESET_SCREEN = new TextEncoder().encode('\x1bc')

/** What Shift+Enter sends: Alt+Enter, which Claude Code and Codex take as a new line in their prompt. */
const NEW_LINE_KEYS = '\x1b\r'

export function TerminalView({ conversation, desktop, fontSize, look = {}, label, inputDisabled = false, ended = false, focused = false, onInput, onOpenPath, onHandle, onPasteImage }: TerminalViewProps) {
  if (!desktop) return null
  if (!isDesktop()) return <div className="panel-empty"><TerminalSquare size={18} /><h2>Native terminal is not available</h2><p>Open Maitty Desktop to use the daemon-owned terminal.</p></div>
  if (!hasNativeTerminal(conversation)) return <NativeHandoffView conversation={conversation} />

  return <ConnectedTerminalView conversation={conversation} fontSize={fontSize} look={look} label={label ?? `Native ${conversation.provider} terminal`} inputDisabled={inputDisabled || ended} ended={ended} focused={focused} onInput={onInput} onOpenPath={onOpenPath} onHandle={onHandle} onPasteImage={onPasteImage} />
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

type ConnectedProps = { conversation: NativeConversation; fontSize: number; look: TerminalLook; label: string; inputDisabled: boolean; ended: boolean; focused: boolean; onInput: ((data: string | Uint8Array) => void) | undefined; onOpenPath: ((path: string, line?: number) => void) | undefined; onHandle: ((handle: TerminalHandle | null) => void) | undefined; onPasteImage: ((image: File) => void) | undefined }

function ConnectedTerminalView({ conversation, fontSize, look, label, inputDisabled, ended, focused, onInput, onOpenPath, onHandle, onPasteImage }: ConnectedProps) {
  const term = conversation.nativePane
  // Each machine numbers its own panes: the stream and its history are kept by machine and pane.
  const machine = conversation.machine ?? 0
  const binding: TerminalBinding = useMemo(() => ({ id: conversation.id, generation: conversation.generation }), [conversation.id, conversation.generation])
  const terminalHost = useRef<HTMLDivElement>(null)
  const terminal = useRef<XtermTerminal | null>(null)
  const inputFailedRef = useRef(false)
  const inputFailedAt = useRef(0)
  const fontSizeRef = useRef(fontSize)
  const lookRef = useRef(look)
  const inputDisabledRef = useRef(inputDisabled)
  const focusedRef = useRef(focused)
  const windowActive = useWindowActive()
  const rendererRef = useRef<Renderer>('dom')
  const endedRef = useRef(ended)
  useEffect(() => { endedRef.current = ended })
  const onInputRef = useRef(onInput)
  useEffect(() => { onInputRef.current = onInput })
  const onOpenPathRef = useRef(onOpenPath)
  const onHandleRef = useRef(onHandle)
  const onPasteImageRef = useRef(onPasteImage)
  useEffect(() => {
    onOpenPathRef.current = onOpenPath
    onHandleRef.current = onHandle
    onPasteImageRef.current = onPasteImage
  })
  const refitTerminal = useRef<(() => void) | null>(null)
  const [connecting, setConnecting] = useState(true)
  const [catchUpMs, setCatchUpMs] = useState<number | null>(null)
  const [loadError, setLoadError] = useState(false)
  const [inputFailed, setInputFailed] = useState(false)
  const [resizeError, setResizeError] = useState('')
  const [transparent, setTransparent] = useState(() => hasWebgl2() && !(look.plainDrawing ?? false) && (look.seeThrough ?? false))

  const send = useCallback((data: string | Uint8Array) => {
    if (!data || inputDisabledRef.current) return
    if (inputFailedRef.current) {
      // The daemon may be back: after a pause the next key tries again, instead of waiting for a click.
      if (Date.now() - inputFailedAt.current < INPUT_RETRY_MS) return
      inputFailedRef.current = false
      setInputFailed(false)
    }
    onInputRef.current?.(data)
    // A key goes out at once and is not held for the one before it to be answered: a reply can take
    // as long as the web view's next event, and each letter waited for it. The host takes keys in the
    // order they arrive (`terminal_input` is not async), so nothing is reordered. The daemon echoes
    // what was typed through the stream, so there is nothing to write here.
    const sentAt = performance.now()
    sendTerminalInput(binding, data).then(() => {
      countKeyReply(performance.now() - sentAt)
    }).catch(() => {
      inputFailedRef.current = true
      inputFailedAt.current = Date.now()
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
    let disposePasteImage: (() => void) | null = null
    let frame = 0
    let flushWait = 0
    let queued: Uint8Array[] = []
    // xterm answers every query it parses, and a replay or history is full of old ones. While a
    // write that carries one is being parsed, what xterm sends back is not input (ticket 40).
    let queuedReplay = false
    let replayWrites = 0
    let replayWait = 0
    let lastFlushAt = 0
    const screen = screenKey(machine, term, binding.generation)

    const writeQueued = () => {
      if (frame !== 0) cancelAnimationFrame(frame)
      frame = 0
      window.clearTimeout(flushWait)
      flushWait = 0
      if (!emulator || queued.length === 0) return
      const bytes = new Uint8Array(queued.reduce((total, chunk) => total + chunk.length, 0))
      let offset = 0
      for (const chunk of queued) {
        bytes.set(chunk, offset)
        offset += chunk.length
      }
      queued = []
      const replaying = queuedReplay
      queuedReplay = false
      if (replaying) replayWrites += 1
      const flushed = beginFlush()
      emulator.write(bytes, () => {
        flushed()
        if (replaying) replayWrites -= 1
      })
      lastFlushAt = performance.now()
    }
    // Output is written once per animation frame, so a burst repaints once instead of per chunk, and
    // a frame that comes too soon after the last write waits for the next.
    const flushOnFrame = () => {
      frame = 0
      const sinceLastFlush = performance.now() - lastFlushAt
      if (sinceLastFlush < FLUSH_GAP_MS) {
        // Sleep out the gap rather than asking for every frame until it has passed.
        flushWait = window.setTimeout(() => {
          flushWait = 0
          if (frame === 0) frame = requestAnimationFrame(flushOnFrame)
        }, FLUSH_GAP_MS - sinceLastFlush)
        return
      }
      writeQueued()
    }
    const enqueue = (...parts: Uint8Array[]) => {
      queued.push(...parts)
      // A hidden window gets no animation frames, so output would pile up and then land in one freeze.
      if (document.hidden) {
        writeQueued()
        return
      }
      if (frame === 0 && flushWait === 0) frame = requestAnimationFrame(flushOnFrame)
    }

    // A screen that replaces what the pane shows: history, a replay, the kept screen.
    const enqueueReplay = (...parts: Uint8Array[]) => {
      queuedReplay = true
      enqueue(...parts)
    }

    async function mountTerminal() {
      try {
        const [{ Terminal: TerminalEmulator }, { FitAddon }, { SearchAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
          import('@xterm/addon-search'),
        ])
        if (!isMounted) return

        const wantsWebgl = hasWebgl2() && !(lookRef.current.plainDrawing ?? false)
        const startRenderer: Renderer = wantsWebgl ? 'webgl' : 'dom'
        const startSeeThrough = seeThroughOf(lookRef.current, startRenderer)
        emulator = new TerminalEmulator({
          allowTransparency: startSeeThrough,
          cursorBlink: (lookRef.current.cursorBlink ?? false) && focusedRef.current && isWindowActive(),
          cursorStyle: lookRef.current.cursorStyle ?? 'block',
          disableStdin: inputDisabledRef.current,
          fontFamily: fontStackFor(lookRef.current.fontFamily ?? ''),
          fontWeight: lookRef.current.fontWeight ?? DEFAULT_FONT_WEIGHT,
          fontWeightBold: lookRef.current.fontWeightBold ?? DEFAULT_FONT_WEIGHT_BOLD,
          fontSize: fontSizeRef.current,
          lineHeight: LINE_HEIGHT,
          scrollback: lookRef.current.scrollback ?? DEFAULT_SCROLLBACK_LINES,
          scrollSensitivity: lookRef.current.scrollSpeed ?? DEFAULT_SCROLL_SPEED,
          screenReaderMode: lookRef.current.screenReader ?? false,
          theme: terminalTheme(lookRef.current, startSeeThrough),
          minimumContrastRatio: lookRef.current.minimumContrast ?? NO_MINIMUM_CONTRAST,
        })
        const terminalInstance = emulator
        const fit = new FitAddon()
        terminalInstance.loadAddon(fit)
        const search = new SearchAddon()
        terminalInstance.loadAddon(search)
        terminalInstance.open(hostElement)
        terminal.current = terminalInstance
        const fallBackToDomRenderer = () => {
          rendererRef.current = 'dom'
          reportRenderer(terminalInstance, 'dom')
          terminalInstance.options.allowTransparency = false
          terminalInstance.options.theme = terminalTheme(lookRef.current, false)
          if (isMounted) setTransparent(false)
        }
        if (wantsWebgl) {
          if (await loadWebglRenderer(terminalInstance, fallBackToDomRenderer)) {
            rendererRef.current = 'webgl'
            reportRenderer(terminalInstance, 'webgl')
          } else {
            fallBackToDomRenderer()
          }
          if (!stillMounted()) return
        } else {
          reportRenderer(terminalInstance, 'dom')
        }
        // On a see-through canvas xterm's own answer to a program's colour query would be black
        // (it drops the alpha): a program that paints with the colour it was told, as Claude Code
        // does, would then leave black bars on the glass. Answer with the theme's colours instead;
        // anything but a query falls through to xterm.
        const answerColorQuery = (slot: 'foreground' | 'background') => (data: string) => {
          if (data !== OSC_QUERY) return false
          if (replayWrites > 0) return true
          const color = oscColor((lookRef.current.theme ?? DEFAULT_THEME)[slot] ?? '')
          if (color === null) return false
          send(`\x1b]${String(slot === 'background' ? OSC_BACKGROUND : OSC_FOREGROUND)};${color}\x1b\\`)
          return true
        }
        terminalInstance.parser.registerOscHandler(OSC_FOREGROUND, answerColorQuery('foreground'))
        terminalInstance.parser.registerOscHandler(OSC_BACKGROUND, answerColorQuery('background'))
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

        // Show the last screen this pane had at once; the daemon's history confirms or replaces it.
        let lastSeq = 0
        let shown: number | null = null
        const kept = cachedScreen(screen)
        if (kept) {
          fit.fit()
          enqueueReplay(kept.bytes)
          lastSeq = kept.seq
          shown = kept.seq
        }
        // Follow the live stream first, hold what arrives while the history loads, then write
        // the history and continue from the next chunk: no gap and no repeat.
        let caughtUp = false
        const pending: TerminalChunk[] = []
        const apply = (chunk: TerminalChunk) => {
          if (chunk.seq <= lastSeq) return
          lastSeq = chunk.seq
          if (chunk.replay) {
            window.clearTimeout(replayWait)
            // The replay replaces the screen, so older output still waiting is dropped and the
            // reset travels with the replay: one write for the frame.
            queued = []
            replaceScreen(screen, chunk.seq, chunk.bytes)
            enqueueReplay(RESET_SCREEN, chunk.bytes)
            setConnecting(false)
            return
          }
          appendToScreen(screen, chunk.seq, chunk.bytes)
          enqueue(chunk.bytes)
        }
        // This computer's panes are asked for exactly as before machines: a machine is named only when it is another.
        const machineArg: [] | [number] = machine === 0 ? [] : [machine]
        stopListening = await listenTerminalChunks(term, (chunk) => {
          if (caughtUp) apply(chunk)
          else pending.push(chunk)
        }, ...machineArg)
        if (!stillMounted()) {
          stopListening()
          return
        }
        // The host answers with what it holds, or with nothing once it has asked the daemon for
        // the pane's replay over the stream (it arrives as the next chunk). Without a stream it
        // refuses, and the replay is fetched from the daemon directly.
        let history: TerminalHistory | null
        let fetched = false
        try {
          history = await loadTerminalHistory(term, ...machineArg)
        } catch {
          history = null
          if (!stillMounted()) return
          const replay = await loadTerminalReplay(binding)
          if (!stillMounted()) return
          fetched = true
          // A fetched replay has no sequence number: it is only a screen to start from.
          replaceScreen(screen, lastSeq, replay)
          enqueueReplay(...(shown === null ? [replay] : [RESET_SCREEN, replay]))
        }
        if (!stillMounted()) return
        if (history) {
          // History that ends where the kept screen did changes nothing on screen.
          if (history.seq !== shown) {
            // Drop an older screen still waiting; the history replaces it in the same write.
            if (shown !== null) queued = []
            enqueueReplay(...(shown === null ? [history.bytes] : [RESET_SCREEN, history.bytes]))
          }
          lastSeq = history.seq
          replaceScreen(screen, history.seq, history.bytes)
        } else if (!fetched) {
          // The replay that is coming is not bound by the kept screen's numbering.
          lastSeq = 0
          // The stream may drop between the host's request and the daemon's answer; then the
          // pane would stay blank, so after a while the replay is fetched directly.
          replayWait = window.setTimeout(() => {
            loadTerminalReplay(binding).then(
              (bytes) => { if (stillMounted()) { replaceScreen(screen, lastSeq, bytes); enqueueReplay(RESET_SCREEN, bytes); setConnecting(false) } },
              () => { if (stillMounted()) { setLoadError(true); setConnecting(false) } },
            )
          }, REPLAY_WAIT_MS)
        }
        caughtUp = true
        for (const chunk of pending) apply(chunk)
        // Without history, the replay is still on its way over the stream: attaching has not finished.
        if (history || fetched) setConnecting(false)

        resizeObserver = new ResizeObserver(fitAndResize)
        resizeObserver.observe(hostElement)
        fitAndResize()
        // Geist Mono may still be loading; measure again once it is, so the grid fits the font.
        const fonts = (document as { fonts?: FontFaceSet }).fonts
        void fonts?.ready.then(() => { refitTerminal.current?.() })

        // A terminal sends the same byte for Shift+Enter as for Enter. Agents read ESC then Enter (Alt+Enter) as "new line", so that is what goes.
        // Ctrl+Shift+V is left to the browser, whose paste event is how an image on the clipboard is found (ticket 19). Plain Ctrl+V still reaches the program as ^V.
        terminalInstance.attachCustomKeyEventHandler((event) => {
          if (isPasteChord(event, navigator.userAgent)) return false
          if (event.key !== 'Enter' || !event.shiftKey || event.ctrlKey || event.altKey || event.metaKey) return true
          if (event.type === 'keydown') send(NEW_LINE_KEYS)
          return false
        })
        const inputSubscription = terminalInstance.onData((data) => { if (replayWrites === 0) send(data) })
        const binarySubscription = terminalInstance.onBinary((data) => {
          if (replayWrites > 0) return
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
        // A paste of an image (with no text) goes to the window; text pastes as it always has.
        disposePasteImage = listenForImagePaste(
          hostElement,
          (image) => { onPasteImageRef.current?.(image) },
          (message) => { toast.error(message) },
        )
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
      window.clearTimeout(flushWait)
      window.clearTimeout(replayWait)
      refitTerminal.current = null
      stopListening?.()
      resizeObserver?.disconnect()
      disposeInput?.()
      disposeBinary?.()
      disposeCopy?.()
      disposePasteImage?.()
      if (emulator) forgetRenderer(emulator)
      emulator?.dispose()
      terminal.current = null
    }
  }, [binding, term, machine, send])

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
    // A new face or weight changes the cell size, so the grid is measured again.
    const family = fontStackFor(look.fontFamily ?? '')
    const weight = look.fontWeight ?? DEFAULT_FONT_WEIGHT
    const weightBold = look.fontWeightBold ?? DEFAULT_FONT_WEIGHT_BOLD
    if (emulator.options.fontFamily !== family || emulator.options.fontWeight !== weight || emulator.options.fontWeightBold !== weightBold) {
      emulator.options.fontFamily = family
      emulator.options.fontWeight = weight
      emulator.options.fontWeightBold = weightBold
      refitTerminal.current?.()
    }
    if (look.scrollback && emulator.options.scrollback !== look.scrollback) emulator.options.scrollback = look.scrollback
    if (look.scrollSpeed && emulator.options.scrollSensitivity !== look.scrollSpeed) emulator.options.scrollSensitivity = look.scrollSpeed
    if (look.screenReader !== undefined && emulator.options.screenReaderMode !== look.screenReader) emulator.options.screenReaderMode = look.screenReader
    const seeThrough = seeThroughOf(look, rendererRef.current)
    if (emulator.options.allowTransparency !== seeThrough) emulator.options.allowTransparency = seeThrough
    if (look.theme) emulator.options.theme = terminalTheme(look, seeThrough)
    const minimumContrast = look.minimumContrast ?? NO_MINIMUM_CONTRAST
    if (emulator.options.minimumContrastRatio !== minimumContrast) emulator.options.minimumContrastRatio = minimumContrast
    setTransparent(seeThrough)
  }, [look])

  useEffect(() => {
    inputDisabledRef.current = inputDisabled
    const emulator = terminal.current
    if (emulator) emulator.options.disableStdin = inputDisabled
  }, [inputDisabled])

  // Only another machine can be slow to attach; this computer's panes show nothing while they connect.
  useEffect(() => {
    if (machine === 0 || !connecting) return
    const started = performance.now()
    let tick = 0
    const show = window.setTimeout(() => {
      setCatchUpMs(Math.round(performance.now() - started))
      tick = window.setInterval(() => { setCatchUpMs(Math.round(performance.now() - started)) }, CATCH_UP_TICK_MS)
    }, CATCH_UP_DELAY_MS)
    return () => {
      window.clearTimeout(show)
      window.clearInterval(tick)
    }
  }, [machine, connecting])

  useEffect(() => {
    focusedRef.current = focused
    if (focused && !connecting) terminal.current?.focus()
  }, [focused, connecting])

  useEffect(() => {
    // A blinking cursor repaints its pane twice a second, so it is opt-in, only the pane you are in
    // blinks, and only while the window is in front.
    if (terminal.current) terminal.current.options.cursorBlink = (look.cursorBlink ?? false) && focused && windowActive
  }, [focused, windowActive, connecting, look.cursorBlink])

  return <div className={`terminal-view${transparent ? ' is-transparent' : ''}`}>
    {machine !== 0 && connecting && catchUpMs !== null && !loadError && (
      <div className="terminal-catch-up" role="status">
        <LoaderCircle size={13} aria-hidden="true" />
        <span>Catching up from {conversation.machineName ?? 'the machine'} · {catchUpMs} ms</span>
      </div>
    )}
    {loadError && <div className="terminal-input-error" role="status"><p>Could not load the terminal. Check the daemon connection and reopen this session.</p></div>}
    <div className="terminal-screen" ref={terminalHost} role="region" aria-label={label} />
    {inputFailed && <div className="terminal-input-error" role="status"><p>Input stopped because the daemon could not be reached.</p><Button variant="outline" size="sm" onClick={() => { inputFailedRef.current = false; setInputFailed(false) }}>Try typing again</Button></div>}
    {resizeError && <div className="terminal-input-error" role="status"><p>{resizeError}</p><Button variant="outline" size="sm" onClick={() => { refitTerminal.current?.() }}>Retry resize</Button></div>}
  </div>
}

function hasNativeTerminal(conversation: Conversation): conversation is NativeConversation {
  return typeof conversation.generation === 'number' && typeof conversation.nativePane === 'number'
}
