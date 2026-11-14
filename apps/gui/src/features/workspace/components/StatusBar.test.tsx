import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { StatusBar, type FocusMachine } from './StatusBar'

function show(machine: FocusMachine | null) {
  const none = vi.fn()
  render(
    <StatusBar
      online machine={machine} themeMode="dark" lightInUse={false} activeTheme="Catppuccin Mocha" corners="square" textSize={15} minTextSize={11}
      maxTextSize={20} shortcutChord={['Ctrl', '/']} portalContainer={null} onThemeMode={none} onPickTheme={none} onAllThemes={none}
      onCorners={none} onTextSize={none} onResetTextSize={none} onOpenShortcuts={none}
    />,
  )
}

describe('the status bar', () => {
  it('names the machine of the focused pane and its state when there is more than one machine', () => {
    show({ name: 'build', word: 'reconnecting', tone: 'bad' })
    expect(screen.getByRole('status', { name: 'Focused pane runs on build, reconnecting' })).toHaveTextContent('build')
  })

  it('says nothing about machines while this computer is the only one', () => {
    show(null)
    expect(screen.queryByText('This computer')).toBeNull()
  })
})
