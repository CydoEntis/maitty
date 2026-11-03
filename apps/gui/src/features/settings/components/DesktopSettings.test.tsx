import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SettingsPreferences } from '../preferences'
import { DesktopSettings, type DesktopSettingsSection } from './DesktopSettings'

const PREFS: SettingsPreferences = { accent: 'teal', spinner: 'braille', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notifyError: true, notificationSound: false, terminalFontSize: 13, startFolder: '', groupBy: 'repo', themeMode: 'dark', darkTheme: 'Midnight', lightTheme: 'Paper', cursorStyle: 'block', scrollback: 10000, effectWorking: true, effectNeedsYou: true, effectDone: true, density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, iconSize: 'medium', trayStyle: 'names', leadMode: 'focused', iconColor: 'color', style: 'glass', projectColors: {} }

function renderSettings(section: DesktopSettingsSection, overrides: Partial<Parameters<typeof DesktopSettings>[0]> = {}) {
  const onChange = vi.fn()
  const onSection = vi.fn()
  const onPreviewTheme = vi.fn()
  const onShowWelcome = vi.fn()
  render(<DesktopSettings section={section} onSection={onSection} preferences={PREFS} onChange={onChange} systemReducedMotion={false} systemDark onPreviewTheme={onPreviewTheme} onShowWelcome={onShowWelcome} mac={false} daemon={{ online: true, sessionCount: 3 }} projects={[{ key: '/code/maitty', name: 'maitty', colorId: undefined }]} validateFolder={() => Promise.resolve(true)} onClose={vi.fn()} {...overrides} />)
  return { onChange, onSection, onPreviewTheme, onShowWelcome }
}

describe('desktop settings', () => {
  it('groups the sections the way the design does', () => {
    renderSettings('general')
    const nav = screen.getByRole('navigation', { name: 'Settings sections' })
    expect(nav).toHaveTextContent('ESSENTIALSGeneralGTERMINALText & cursorTAppearanceAWORKSPACENotificationsNHotkeys?ADVANCEDDaemonD')
    expect(within(nav).getByRole('button', { name: /General/ })).toHaveAttribute('aria-current', 'page')
  })

  it('has one switch each for needs you, done and error notifications', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('notifications')

    await user.click(screen.getByRole('switch', { name: 'Notify when a session errors' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, notifyError: false })
    await user.click(screen.getByRole('switch', { name: 'Notify when a session finishes' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, notifyFinished: false })
    await user.click(screen.getByRole('switch', { name: 'Notify when a session needs you' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, notifyNeedsYou: false })
  })

  it('sets the text size, cursor and scrollback for every pane', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('terminal')

    expect(screen.getByText('Geist Mono · 13px')).toBeInTheDocument()
    expect(within(screen.getByRole('radiogroup', { name: 'Terminal text size' })).getAllByRole('radio').map((radio) => radio.textContent)).toEqual(['12', '13', '14', '15', '16'])
    await user.click(screen.getByRole('radio', { name: '15' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, terminalFontSize: 15 })
    await user.click(screen.getByRole('radio', { name: 'Underline' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, cursorStyle: 'underline' })
    await user.click(screen.getByRole('radio', { name: '50k' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, scrollback: 50000 })
  })

  it('picks a working spinner from cards showing the TUI frames', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')
    const spinners = screen.getByRole('radiogroup', { name: 'Working spinner' })
    expect(within(spinners).getByRole('radio', { name: 'Braille' })).toHaveAttribute('aria-checked', 'true')
    expect(spinners).toHaveTextContent('10 frames · 80 ms')
    expect(spinners).toHaveTextContent('8 frames · 90 ms')
    await user.click(within(spinners).getByRole('radio', { name: 'Snake' }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, spinner: 'snake' })
  })

  it('picks a dark and a light theme separately, from searchable dropdowns that list the one in use first', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, darkTheme: 'Nord' } })
    expect(screen.getByText('27 installed')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Dark theme: Nord' }))
    const dark = screen.getByRole('listbox', { name: 'Dark themes' })
    expect(within(dark).getAllByRole('option')[0]).toHaveTextContent('Nord')
    expect(within(dark).queryByRole('option', { name: /Paper/ })).not.toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: 'Search dark themes' }), 'dracula')
    expect(within(dark).getAllByRole('option')).toHaveLength(1)
    await user.click(within(dark).getByRole('option', { name: /Dracula/ }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ darkTheme: 'Dracula' }))
    expect(screen.queryByRole('listbox', { name: 'Dark themes' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Light theme: Paper' }))
    expect(within(screen.getByRole('listbox', { name: 'Light themes' })).getByRole('option', { name: /GitHub Light/ })).toBeInTheDocument()
  })

  it('switches between System, Dark and Light, and says which one the system picks', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, themeMode: 'system' } })
    expect(screen.getByText('Follows your OS. Right now: dark.')).toBeInTheDocument()

    await user.click(within(screen.getByRole('radiogroup', { name: 'Appearance' })).getByRole('radio', { name: 'Light' }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ themeMode: 'light' }))
  })

  it('previews a hovered theme only for the slot that is in use, and ends the preview on leaving', async () => {
    const user = userEvent.setup()
    const { onPreviewTheme } = renderSettings('appearance', { preferences: { ...PREFS, themeMode: 'dark' } })

    await user.click(screen.getByRole('button', { name: 'Dark theme: Midnight' }))
    await user.hover(within(screen.getByRole('listbox', { name: 'Dark themes' })).getByRole('option', { name: /Nord/ }))
    expect(onPreviewTheme).toHaveBeenLastCalledWith('Nord')
    await user.unhover(screen.getByRole('listbox', { name: 'Dark themes' }))
    expect(onPreviewTheme).toHaveBeenLastCalledWith(null)

    await user.keyboard('{Escape}')
    await user.click(screen.getByRole('button', { name: 'Light theme: Paper' }))
    onPreviewTheme.mockClear()
    await user.hover(within(screen.getByRole('listbox', { name: 'Light themes' })).getByRole('option', { name: /GitHub Light/ }))
    expect(onPreviewTheme).not.toHaveBeenCalledWith('GitHub Light')
  })

  it('keeps the status effect switches off while the system reduces motion', () => {
    renderSettings('appearance', { systemReducedMotion: true })
    for (const name of ['Working', 'Needs you', 'Done']) {
      expect(screen.getByRole('switch', { name })).toBeDisabled()
      expect(screen.getByRole('switch', { name })).toHaveAttribute('aria-checked', 'false')
    }
  })

  it('shows only what maittyd reports about itself', () => {
    renderSettings('daemon', { daemon: { online: false, sessionCount: 3 } })
    expect(screen.getByRole('status')).toHaveTextContent('Disconnected')
    expect(screen.getByText('Sessions').parentElement?.parentElement).toHaveTextContent('3')
    expect(screen.queryByText('Version')).not.toBeInTheDocument()
    expect(screen.queryByText('Socket')).not.toBeInTheDocument()
  })

  it('gives a project a colour from a set without amber or red, and takes it back on a second click', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')
    const swatches = screen.getByRole('radiogroup', { name: 'Color for maitty' })

    expect(within(swatches).getAllByRole('radio').map((radio) => radio.getAttribute('aria-label'))).toEqual(['Teal', 'Violet', 'Blue', 'Green', 'Pink', 'Orange'])
    await user.click(within(swatches).getByRole('radio', { name: 'Violet' }))

    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ projectColors: { '/code/maitty': 'violet' } }))
  })

  it('sets how big the sidebar icons are', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')

    await user.click(within(screen.getByRole('radiogroup', { name: 'Sidebar icon size' })).getByRole('radio', { name: 'Small' }))

    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ iconSize: 'small' }))
  })

  it('offers Flat, Tiled and Glass as the window style, with the current one checked', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, style: 'glass' } })
    const styles = screen.getByRole('radiogroup', { name: 'Window style' })

    expect(within(styles).getAllByRole('radio').map((radio) => radio.textContent)).toEqual([
      expect.stringContaining('Flat'),
      expect.stringContaining('Tiled'),
      expect.stringContaining('Glass'),
    ])
    expect(within(styles).getByRole('radio', { name: /Glass/ })).toBeChecked()

    await user.click(within(styles).getByRole('radio', { name: /Tiled/ }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ style: 'tiled' }))
  })

  it('replays the welcome from General', async () => {
    const user = userEvent.setup()
    const { onShowWelcome } = renderSettings('general')

    await user.click(screen.getByRole('button', { name: 'Show the welcome again' }))

    expect(onShowWelcome).toHaveBeenCalledTimes(1)
  })
})
