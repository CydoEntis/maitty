import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SettingsPreferences } from '../preferences'
import { DesktopSettings, type DesktopSettingsSection } from './DesktopSettings'

const PREFS: SettingsPreferences = { accent: 'teal', spinner: 'braille', restoreTabs: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notifyError: true, notificationSound: false, terminalFontSize: 13, startFolder: '', groupBy: 'repo', theme: 'Midnight', cursorStyle: 'block', scrollback: 10000, effectWorking: true, effectNeedsYou: true, effectDone: true, density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, iconSize: 'medium', trayStyle: 'names', leadMode: 'focused', style: 'expressive', projectColors: {} }

function renderSettings(section: DesktopSettingsSection, overrides: Partial<Parameters<typeof DesktopSettings>[0]> = {}) {
  const onChange = vi.fn()
  const onSection = vi.fn()
  render(<DesktopSettings section={section} onSection={onSection} preferences={PREFS} onChange={onChange} systemReducedMotion={false} mac={false} daemon={{ online: true, sessionCount: 3 }} projects={[{ key: '/code/maitty', name: 'maitty', colorId: undefined }]} validateFolder={() => Promise.resolve(true)} onClose={vi.fn()} {...overrides} />)
  return { onChange, onSection }
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

  it('pins the current theme first and filters to light themes', async () => {
    const user = userEvent.setup()
    renderSettings('appearance', { preferences: { ...PREFS, theme: 'Nord' } })
    const list = screen.getByRole('listbox', { name: 'Themes' })
    expect(within(list).getAllByRole('option')[0]).toHaveTextContent('Nord')
    expect(screen.getByText('26 installed')).toBeInTheDocument()

    await user.click(screen.getByRole('radio', { name: 'Light' }))
    expect(within(list).getAllByRole('option').map((option) => option.textContent)).toEqual(expect.arrayContaining([expect.stringContaining('Paper')]))
    expect(screen.getByText(/Showing 6 of 26\./)).toBeInTheDocument()
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
})
