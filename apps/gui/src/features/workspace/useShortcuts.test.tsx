import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useShortcutKeys } from './useShortcuts'

function listen(canUndoClose: boolean) {
  const runShortcut = vi.fn()
  renderHook(() => {
    useShortcutKeys({
      runShortcut,
      canUndoClose,
      dialogOpen: false,
      onNewChatPage: false,
      overlayOpen: false,
      paletteOpen: false,
      focusMode: false,
      drawerShown: false,
      welcomeOpen: false,
      shortcutsOpen: false,
      setPaletteOpen: vi.fn(),
      setSettingsSection: vi.fn(),
      setNewChatOpen: vi.fn(),
      setDrawerOpen: vi.fn(),
      setFocusMode: vi.fn(),
      platform: 'windows',
    })
  })
  return runShortcut
}

function pressUndoIn(className: string) {
  const field = document.createElement('textarea')
  field.className = className
  document.body.append(field)
  field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Z', code: 'KeyZ', ctrlKey: true, shiftKey: true, bubbles: true, cancelable: true }))
}

describe('the undo close shortcut', () => {
  afterEach(() => { document.body.replaceChildren() })

  it('undoes a close while the keys are in a terminal', () => {
    const runShortcut = listen(true)
    pressUndoIn('xterm-helper-textarea')
    expect(runShortcut).toHaveBeenCalledWith('undo-close')
  })

  it('leaves undo to a text field', () => {
    const runShortcut = listen(true)
    pressUndoIn('')
    expect(runShortcut).not.toHaveBeenCalled()
  })

  it('leaves the key to the terminal when nothing was just closed', () => {
    const runShortcut = listen(false)
    pressUndoIn('xterm-helper-textarea')
    expect(runShortcut).not.toHaveBeenCalled()
  })
})
