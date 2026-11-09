import { afterEach, describe, expect, it } from 'vitest'
import { editStepOf, installUndoRedo, isTextField } from './undoRedo'

const LINUX = 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15'
const press = (key: string, extra: Partial<KeyboardEventInit> = {}) => ({ key, ctrlKey: true, shiftKey: false, altKey: false, metaKey: false, ...extra })

describe('which edit a key asks for', () => {
  it('reads Ctrl+Z as undo and Ctrl+Shift+Z or Ctrl+Y as redo', () => {
    expect(editStepOf(press('z'))).toBe('undo')
    expect(editStepOf(press('Z', { shiftKey: true }))).toBe('redo')
    expect(editStepOf(press('y'))).toBe('redo')
  })

  it('ignores other chords', () => {
    expect(editStepOf(press('z', { altKey: true }))).toBeNull()
    expect(editStepOf(press('z', { ctrlKey: false }))).toBeNull()
    expect(editStepOf(press('y', { shiftKey: true }))).toBeNull()
    expect(editStepOf(press('v'))).toBeNull()
  })
})

describe('undo and redo in a text box on Linux', () => {
  let stop: () => void = () => undefined
  let field: HTMLTextAreaElement
  let inputs: string[]

  const setup = (userAgent = LINUX) => {
    field = document.createElement('textarea')
    document.body.append(field)
    inputs = []
    field.addEventListener('input', () => { inputs.push(field.value) })
    stop = installUndoRedo(document, userAgent)
  }
  afterEach(() => { stop(); field.remove() })

  /** What typing does: the event before, the new value, the event after. */
  const type = (value: string, inputType = 'insertText') => {
    field.dispatchEvent(new InputEvent('beforeinput', { bubbles: true, inputType }))
    field.value = value
    field.dispatchEvent(new InputEvent('input', { bubbles: true, inputType }))
  }
  const key = (name: string, extra: Partial<KeyboardEventInit> = {}) => {
    const event = new KeyboardEvent('keydown', { key: name, ctrlKey: true, bubbles: true, cancelable: true, ...extra })
    field.dispatchEvent(event)
    return event
  }

  it('undoes a run of typing in one step and redoes it', () => {
    setup()
    type('h')
    type('he')
    type('hey')
    expect(key('z').defaultPrevented).toBe(true)
    expect(field.value).toBe('')
    key('y')
    expect(field.value).toBe('hey')
    key('z')
    key('Z', { shiftKey: true })
    expect(field.value).toBe('hey')
  })

  it('tells the page, so a controlled box follows', () => {
    setup()
    type('hello')
    key('z')
    expect(inputs.at(-1)).toBe('')
  })

  it('makes a paste a step of its own', () => {
    setup()
    type('a')
    type('a long pasted text', 'insertFromPaste')
    key('z')
    expect(field.value).toBe('a')
  })

  it('starts afresh when the value is set from outside', () => {
    setup()
    type('sent message')
    field.value = ''
    type('n')
    key('z')
    expect(field.value).toBe('')
    key('z')
    expect(field.value).toBe('')
  })

  it('has nothing to undo in an untouched box', () => {
    setup()
    field.value = 'kept'
    expect(key('z').defaultPrevented).toBe(true)
    expect(field.value).toBe('kept')
  })

  it('leaves a terminal alone', () => {
    setup()
    const screen = document.createElement('div')
    screen.className = 'xterm'
    const input = document.createElement('textarea')
    screen.append(input)
    document.body.append(screen)
    expect(isTextField(input)).toBe(false)
    const event = new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true, cancelable: true })
    input.dispatchEvent(event)
    expect(event.defaultPrevented).toBe(false)
    screen.remove()
  })

  it('does nothing on Windows and macOS, whose web views undo natively', () => {
    setup('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')
    type('hello')
    expect(key('z').defaultPrevented).toBe(false)
    expect(field.value).toBe('hello')
  })
})
