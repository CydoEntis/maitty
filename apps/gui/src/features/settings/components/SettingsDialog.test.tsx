import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SettingsDialog } from './SettingsDialog'

describe('SettingsDialog', () => {
  it('lets the user change the theme and notification preferences', async () => {
    const user = userEvent.setup()
    const onThemeChange = vi.fn()
    const onPreferencesChange = vi.fn()
    const onNotificationSoundChange = vi.fn(() => true)
    const onTestNotification = vi.fn()
    render(<SettingsDialog open onClose={vi.fn()} theme="night" onThemeChange={onThemeChange} preferences={{ accent: 'teal', reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false }} onPreferencesChange={onPreferencesChange} onNotificationSoundChange={onNotificationSoundChange} onTestNotification={onTestNotification} daemonConnectionState="connected" onReconnect={vi.fn()} codexExecutableState="preview" codexExecutablePath={undefined} onCodexRecheck={vi.fn()} claudeExecutableState="preview" claudeExecutablePath={undefined} onClaudeRecheck={vi.fn()} primaryModifier="Ctrl" />)

    await user.click(screen.getByRole('button', { name: /paper/i }))
    expect(onThemeChange).toHaveBeenCalledWith('day')
    await user.click(screen.getByRole('button', { name: 'Violet' }))
    expect(onPreferencesChange).toHaveBeenCalledWith(expect.objectContaining({ accent: 'violet' }))

    await user.click(screen.getByRole('tab', { name: 'Notifications' }))
    await user.click(screen.getByRole('checkbox', { name: /needs your attention/i }))
    expect(onPreferencesChange).toHaveBeenCalledWith({ accent: 'teal', reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: false, notifyFinished: true, notificationSound: false })
    await user.click(screen.getByRole('button', { name: 'Send a test' }))
    expect(onTestNotification).toHaveBeenCalledOnce()
    await user.click(screen.getByRole('checkbox', { name: /Sound/ }))
    expect(onNotificationSoundChange).toHaveBeenCalledWith(true)
  })

  it('offers daemon reconnect when disconnected', async () => {
    const user = userEvent.setup()
    const onReconnect = vi.fn()
    render(<SettingsDialog open onClose={vi.fn()} theme="night" onThemeChange={vi.fn()} preferences={{ accent: 'teal', reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false }} onPreferencesChange={vi.fn()} onNotificationSoundChange={vi.fn(() => true)} onTestNotification={vi.fn()} daemonConnectionState="disconnected" onReconnect={onReconnect} codexExecutableState="unavailable" codexExecutablePath={undefined} onCodexRecheck={vi.fn()} claudeExecutableState="available" claudeExecutablePath="C:\\Users\\casey\\.local\\bin\\claude.exe" onClaudeRecheck={vi.fn()} primaryModifier="Ctrl" />)

    await user.click(screen.getByRole('tab', { name: 'Daemon & providers' }))
    expect(screen.getByText('Codex was not found on this machine.')).toBeInTheDocument()
    expect(screen.getByText('CLI found. Claude Code manages sign-in on this machine.')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Reconnect' }))
    expect(onReconnect).toHaveBeenCalledOnce()
  })

  it('lists the project folder shortcut with the current platform modifier', async () => {
    const user = userEvent.setup()
    render(<SettingsDialog open onClose={vi.fn()} theme="night" onThemeChange={vi.fn()} preferences={{ accent: 'teal', reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false }} onPreferencesChange={vi.fn()} onNotificationSoundChange={vi.fn(() => true)} onTestNotification={vi.fn()} daemonConnectionState="connected" onReconnect={vi.fn()} codexExecutableState="preview" codexExecutablePath={undefined} onCodexRecheck={vi.fn()} claudeExecutableState="preview" claudeExecutablePath={undefined} onClaudeRecheck={vi.fn()} primaryModifier="Ctrl" />)

    await user.click(screen.getByRole('tab', { name: 'Keyboard' }))

    expect(screen.getByText('Open a project folder').parentElement).toHaveTextContent('CtrlO')
  })

  it('moves between Settings tabs with arrows in the visible direction', async () => {
    const user = userEvent.setup()
    const originalWidth = window.innerWidth
    render(<SettingsDialog open onClose={vi.fn()} theme="night" onThemeChange={vi.fn()} preferences={{ accent: 'teal', reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notificationSound: false }} onPreferencesChange={vi.fn()} onNotificationSoundChange={vi.fn(() => true)} onTestNotification={vi.fn()} daemonConnectionState="connected" onReconnect={vi.fn()} codexExecutableState="preview" codexExecutablePath={undefined} onCodexRecheck={vi.fn()} claudeExecutableState="preview" claudeExecutablePath={undefined} onClaudeRecheck={vi.fn()} primaryModifier="Ctrl" />)

    try {
      const appearanceTab = screen.getByRole('tab', { name: 'Appearance' })
      expect(screen.getByRole('tablist')).toHaveAttribute('aria-orientation', 'vertical')
      expect(appearanceTab).toHaveAttribute('tabindex', '0')
      expect(screen.getByRole('tab', { name: 'Notifications' })).toHaveAttribute('tabindex', '-1')
      appearanceTab.focus()
      await user.keyboard('{ArrowDown}')
      const notificationsTab = screen.getByRole('tab', { name: 'Notifications' })
      expect(notificationsTab).toHaveAttribute('aria-selected', 'true')
      expect(notificationsTab).toHaveFocus()

      Object.defineProperty(window, 'innerWidth', { configurable: true, value: 600 })
      fireEvent.resize(window)
      expect(screen.getByRole('tablist')).toHaveAttribute('aria-orientation', 'horizontal')
      await user.keyboard('{ArrowRight}')
      expect(screen.getByRole('tab', { name: 'Daemon & providers' })).toHaveAttribute('aria-selected', 'true')
    } finally {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: originalWidth })
      fireEvent.resize(window)
    }
  })
})
