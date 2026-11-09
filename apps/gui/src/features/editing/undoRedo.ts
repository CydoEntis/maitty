import { detectPlatform } from '@/features/workspace/model/shortcuts'

export type EditStep = 'undo' | 'redo'

type KeyPress = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'shiftKey' | 'altKey' | 'metaKey'>
type Field = HTMLTextAreaElement | HTMLInputElement
type Snapshot = { value: string; start: number; end: number }
type History = { undo: Snapshot[]; redo: Snapshot[]; last: Snapshot; changedAt: number }

/** Typing within this long of the last keystroke is one undo step, as in a native text box. */
const TYPING_PAUSE_MS = 1000
const MAX_STEPS = 200
const TYPING_KINDS = new Set(['insertText', 'deleteContentBackward', 'deleteContentForward'])

/** Which edit a key press asks for: Ctrl+Z undoes, Ctrl+Shift+Z and Ctrl+Y redo. */
export function editStepOf(event: KeyPress): EditStep | null {
  if (!event.ctrlKey || event.altKey || event.metaKey) return null
  const key = event.key.toLowerCase()
  if (key === 'z') return event.shiftKey ? 'redo' : 'undo'
  if (key === 'y' && !event.shiftKey) return 'redo'
  return null
}

/** A box the user types prose in. A terminal's own input is left out: its Ctrl+Z is the program's. */
export function isTextField(target: EventTarget | null): target is Field {
  if (!(target instanceof HTMLElement) || target.closest('.xterm') !== null) return false
  return target instanceof HTMLTextAreaElement || (target instanceof HTMLInputElement && /^(text|search|url|email)$/.test(target.type))
}

const snapshotOf = (field: Field): Snapshot => ({ value: field.value, start: field.selectionStart ?? field.value.length, end: field.selectionEnd ?? field.value.length })

/**
 * The window keeps its own undo history for text boxes. WebKitGTK's is empty for the chat box: the
 * app sets its value as React renders, which drops the web view's stack, so Ctrl+Z did nothing. Each
 * box's history is read from `input` events, and a value set from outside (a sent message clearing the
 * box) starts it afresh.
 */
export class EditHistories {
  private readonly all = new WeakMap<Field, History>()
  private restoring = false

  /** Before a change lands: the state the next undo should go back to. */
  beforeChange(field: Field): void {
    const known = this.all.get(field)
    if (known === undefined || known.last.value !== field.value) this.all.set(field, { undo: [], redo: [], last: snapshotOf(field), changedAt: 0 })
  }

  /** After a change: one step back is kept, joined to the last when it is the same run of typing. */
  changed(field: Field, kind: string, now: number): void {
    if (this.restoring) return
    const history = this.all.get(field)
    if (history === undefined || history.last.value === field.value) return
    const joins = TYPING_KINDS.has(kind) && now - history.changedAt < TYPING_PAUSE_MS && history.undo.length > 0
    if (!joins) {
      history.undo.push(history.last)
      if (history.undo.length > MAX_STEPS) history.undo.shift()
    }
    history.redo = []
    history.last = snapshotOf(field)
    history.changedAt = now
  }

  /** Goes one step back or forward; false when there is none. */
  step(field: Field, step: EditStep): boolean {
    this.beforeChange(field)
    const history = this.all.get(field)
    const target = history === undefined ? undefined : (step === 'undo' ? history.undo : history.redo).pop()
    if (history === undefined || target === undefined) return false
    ;(step === 'undo' ? history.redo : history.undo).push(snapshotOf(field))
    this.restore(field, target)
    history.last = target
    history.changedAt = 0
    return true
  }

  private restore(field: Field, to: Snapshot): void {
    // The value goes through the prototype's setter, not the field's own, so React's tracking sees a change and calls `onChange`.
    this.restoring = true
    try {
      Reflect.set(Object.getPrototypeOf(field) as object, 'value', to.value, field)
      field.setSelectionRange(to.start, to.end)
      field.dispatchEvent(new Event('input', { bubbles: true }))
    } finally {
      this.restoring = false
    }
  }
}

/**
 * Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y in the window's text boxes on Linux, whose web view has no working
 * undo there. Windows and macOS web views undo natively and are left alone. Returns the stop function.
 */
export function installUndoRedo(doc: Document, userAgent: string): () => void {
  if (detectPlatform(userAgent) !== 'linux') return () => undefined
  const histories = new EditHistories()
  const onBeforeInput = (event: Event) => { if (isTextField(event.target)) histories.beforeChange(event.target) }
  const onInput = (event: Event) => {
    if (isTextField(event.target)) histories.changed(event.target, event instanceof InputEvent ? event.inputType : '', Date.now())
  }
  const onKeyDown = (event: KeyboardEvent) => {
    const step = editStepOf(event)
    if (step === null || event.defaultPrevented || !isTextField(event.target)) return
    event.preventDefault()
    histories.step(event.target, step)
  }
  doc.addEventListener('beforeinput', onBeforeInput, true)
  doc.addEventListener('input', onInput, true)
  doc.addEventListener('keydown', onKeyDown, true)
  return () => {
    doc.removeEventListener('beforeinput', onBeforeInput, true)
    doc.removeEventListener('input', onInput, true)
    doc.removeEventListener('keydown', onKeyDown, true)
  }
}
