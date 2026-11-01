import { useEffect, useState, type KeyboardEvent } from 'react'
import { Bell, Keyboard, Monitor, Radio, Settings2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import type { ThemePreference } from '@/data/themePreference'
import type { SettingsPreferences } from '../preferences'

type SettingsSection = 'appearance' | 'notifications' | 'daemon' | 'keyboard'
type SettingsNavOrientation = 'horizontal' | 'vertical'
type CodexExecutableState = 'checking' | 'available' | 'unavailable' | 'preview'
type ClaudeExecutableState = 'checking' | 'available' | 'unavailable' | 'preview'
type DaemonConnectionState = 'connected' | 'connecting' | 'disconnected' | 'preview'

const SETTINGS_NAV_HORIZONTAL_MAX_WIDTH_PX = 620

type SettingsDialogProps = {
  open: boolean
  onClose: () => void
  theme: ThemePreference
  onThemeChange: (theme: ThemePreference) => void
  preferences: SettingsPreferences
  onPreferencesChange: (preferences: SettingsPreferences) => void
  onNotificationSoundChange: (enabled: boolean) => boolean
  onTestNotification: () => void
  daemonConnectionState: DaemonConnectionState
  onReconnect: () => void
  codexExecutableState: CodexExecutableState
  codexExecutablePath: string | undefined
  onCodexRecheck: () => void
  claudeExecutableState: ClaudeExecutableState
  claudeExecutablePath: string | undefined
  onClaudeRecheck: () => void
  primaryModifier: '⌘' | 'Ctrl'
}

const sections: { id: SettingsSection; label: string; icon: typeof Monitor }[] = [
  { id: 'appearance', label: 'Appearance', icon: Monitor },
  { id: 'notifications', label: 'Notifications', icon: Bell },
  { id: 'daemon', label: 'Daemon & providers', icon: Radio },
  { id: 'keyboard', label: 'Keyboard', icon: Keyboard },
]

function SettingsDialog({ open, onClose, theme, onThemeChange, preferences, onPreferencesChange, onNotificationSoundChange, onTestNotification, daemonConnectionState, onReconnect, codexExecutableState, codexExecutablePath, onCodexRecheck, claudeExecutableState, claudeExecutablePath, onClaudeRecheck, primaryModifier }: SettingsDialogProps) {
  const [section, setSection] = useState<SettingsSection>('appearance')
  const [navOrientation, setNavOrientation] = useState<SettingsNavOrientation>(() => window.innerWidth <= SETTINGS_NAV_HORIZONTAL_MAX_WIDTH_PX ? 'horizontal' : 'vertical')
  useEffect(() => {
    const updateNavOrientation = () => { setNavOrientation(window.innerWidth <= SETTINGS_NAV_HORIZONTAL_MAX_WIDTH_PX ? 'horizontal' : 'vertical') }
    window.addEventListener('resize', updateNavOrientation)
    return () => { window.removeEventListener('resize', updateNavOrientation) }
  }, [])

  function handleNavigationKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const direction = navOrientation === 'horizontal'
      ? event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
      : event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0
    const tabs = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="tab"]'))
    const activeIndex = tabs.findIndex((tab) => tab === event.target)
    const nextIndex = event.key === 'Home'
      ? 0
      : event.key === 'End'
        ? tabs.length - 1
        : direction === 0 || activeIndex < 0
          ? -1
          : (activeIndex + direction + tabs.length) % tabs.length
    const nextTab = tabs[nextIndex]
    if (!nextTab) return
    event.preventDefault()
    const nextSection = sections[nextIndex]
    if (!nextSection) return
    setSection(nextSection.id)
    nextTab.focus()
  }

  const updatePreference = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => {
    onPreferencesChange({ ...preferences, [key]: value })
  }

  return <Dialog open={open} onOpenChange={(nextOpen) => { if (!nextOpen) onClose() }}>
    <DialogContent className="settings-dialog">
      <DialogHeader className="settings-heading">
        <DialogTitle>Settings</DialogTitle>
        <DialogDescription>Personalize Maitty Desktop.</DialogDescription>
      </DialogHeader>
      <div className="settings-layout">
        <div className="settings-nav" role="tablist" aria-label="Settings sections" aria-orientation={navOrientation} onKeyDown={handleNavigationKeyDown}>
          {sections.map(({ id, label, icon: Icon }) => <button key={id} type="button" id={`settings-tab-${id}`} role="tab" aria-selected={section === id} aria-controls="settings-panel" tabIndex={section === id ? 0 : -1} className={`settings-nav-item ${section === id ? 'is-active' : ''}`} onClick={() => { setSection(id) }}><Icon size={15} />{label}</button>)}
        </div>
        <section className="settings-content" role="tabpanel" id="settings-panel" aria-labelledby={`settings-tab-${section}`} tabIndex={0}>
          {section === 'appearance' && <>
            <div className="settings-section-title"><h3>Appearance</h3><p>Choose the color theme for this device.</p></div>
            <div className="theme-options" role="group" aria-label="Color theme">
              <button type="button" className={`theme-option theme-option-night ${theme === 'night' ? 'is-selected' : ''}`} aria-pressed={theme === 'night'} onClick={() => { onThemeChange('night') }}><span className="theme-swatch theme-swatch-night" /><span><strong>Midnight</strong><small>Navy and sea glass</small></span></button>
              <button type="button" className={`theme-option theme-option-day ${theme === 'day' ? 'is-selected' : ''}`} aria-pressed={theme === 'day'} onClick={() => { onThemeChange('day') }}><span className="theme-swatch theme-swatch-day" /><span><strong>Paper</strong><small>Light and easy to read</small></span></button>
            </div>
            <div className="settings-section-title settings-subsection-title"><h3>Accent</h3><p>Choose the color used for focus and active controls.</p></div>
            <div className="accent-options" role="group" aria-label="Accent color">
              {(['teal', 'blue', 'violet'] as const).map((accent) => <button key={accent} type="button" className={`accent-option accent-${accent} ${preferences.accent === accent ? 'is-selected' : ''}`} aria-pressed={preferences.accent === accent} onClick={() => { updatePreference('accent', accent) }}><span aria-hidden="true" />{accent === 'teal' ? 'Sea glass' : accent === 'blue' ? 'Blue' : 'Violet'}</button>)}
            </div>
            <label className="settings-row"><span><strong>Reduce motion</strong><small>Stop animated transitions and activity indicators.</small></span><input type="checkbox" checked={preferences.reduceMotion} onChange={(event) => { updatePreference('reduceMotion', event.target.checked) }} /></label>
            <label className="settings-row"><span><strong>Animate sidebar activity</strong><small>Show motion while a conversation is working.</small></span><input type="checkbox" checked={preferences.animateSidebarActivity} disabled={preferences.reduceMotion} onChange={(event) => { updatePreference('animateSidebarActivity', event.target.checked) }} /></label>
            <label className="settings-row"><span><strong>Compact sidebar</strong><small>Keep the conversation list collapsed.</small></span><input type="checkbox" checked={preferences.compactSidebar} onChange={(event) => { updatePreference('compactSidebar', event.target.checked) }} /></label>
          </>}
          {section === 'notifications' && <>
            <div className="settings-section-title"><h3>Notifications</h3><p>Choose which conversation updates appear as desktop toasts.</p></div>
            <label className="settings-row"><span><strong>Needs your attention</strong><small>When a conversation asks for input or approval.</small></span><input type="checkbox" checked={preferences.notifyNeedsYou} onChange={(event) => { updatePreference('notifyNeedsYou', event.target.checked) }} /></label>
            <label className="settings-row"><span><strong>Conversation finished</strong><small>When an active response completes.</small></span><input type="checkbox" checked={preferences.notifyFinished} onChange={(event) => { updatePreference('notifyFinished', event.target.checked) }} /></label>
            <div className="settings-toast-position"><div className="settings-row-label"><strong>Toast position</strong><small>Choose a corner, then send a test.</small></div><div className="toast-position-grid" role="group" aria-label="Toast position">
              {(['top-left', 'top-right', 'bottom-left', 'bottom-right'] as const).map((position) => <button key={position} type="button" className={`toast-position-option ${preferences.toastPosition === position ? 'is-selected' : ''}`} aria-label={position.replace('-', ' ')} aria-pressed={preferences.toastPosition === position} onClick={() => { updatePreference('toastPosition', position) }}><span className={`toast-position-preview toast-${position}`}><i /></span></button>)}
            </div><Button variant="outline" size="sm" onClick={onTestNotification}>Send a test</Button></div>
            <label className="settings-row"><span><strong>Sound</strong><small>Play a quiet tone for conversation updates.</small></span><input type="checkbox" checked={preferences.notificationSound} onChange={(event) => { if (onNotificationSoundChange(event.target.checked)) updatePreference('notificationSound', event.target.checked) }} /></label>
          </>}
          {section === 'daemon' && <>
            <div className="settings-section-title"><h3>Daemon & providers</h3><p>Maitty’s daemon owns your conversations and keeps them running.</p></div>
            <div className="daemon-status-row"><span className={`connection-dot ${daemonConnectionState === 'connected' ? 'is-connected' : ''}`} /><span><strong>Maitty daemon</strong><small>{daemonConnectionState === 'connected' ? 'Connected on this device' : daemonConnectionState === 'connecting' ? 'Connecting to this device…' : daemonConnectionState === 'preview' ? 'Connection status is available in the desktop app.' : 'Disconnected'}</small></span>{daemonConnectionState === 'disconnected' && <Button variant="outline" size="sm" onClick={onReconnect}>Reconnect</Button>}</div>
            <div className="provider-info"><strong>Codex</strong><span>{codexExecutableState === 'available' ? 'Ready on this machine.' : codexExecutableState === 'checking' ? 'Checking this machine…' : codexExecutableState === 'unavailable' ? 'Codex was not found on this machine.' : 'Run the desktop app to check Codex availability.'}</span>{codexExecutablePath && <code>{codexExecutablePath}</code>}{codexExecutableState === 'unavailable' && <Button variant="link" size="sm" onClick={onCodexRecheck}>Recheck</Button>}<small>Codex sign-in and provider settings come from your machine.</small></div>
            <div className="provider-info"><strong>Claude Code</strong><span>{claudeExecutableState === 'available' ? 'CLI found. Claude Code manages sign-in on this machine.' : claudeExecutableState === 'checking' ? 'Checking this machine…' : claudeExecutableState === 'unavailable' ? 'Claude Code was not found on this machine.' : 'Run the desktop app to check Claude Code availability.'}</span>{claudeExecutablePath && <code>{claudeExecutablePath}</code>}{claudeExecutableState === 'unavailable' && <Button variant="link" size="sm" onClick={onClaudeRecheck}>Recheck</Button>}<small>Maitty uses the installed Claude Code CLI; sign in with Claude Code.</small></div>
          </>}
          {section === 'keyboard' && <>
            <div className="settings-section-title"><h3>Keyboard</h3><p>Shortcuts available in Maitty Desktop.</p></div>
            <div className="keyboard-shortcut-list settings-shortcut-list">{[
              ['Open command switcher', primaryModifier, 'K'], ['Move through switcher results', '↑ or ↓', ''], ['Open the focused result', 'Enter', ''], ['Open focused conversation in terminal', primaryModifier, 'Enter'], ['Close the command switcher', 'Esc', ''], ['Open a project folder', primaryModifier, 'O'], ['New conversation', primaryModifier, 'N'], ['Open settings', primaryModifier, ','], ['Next conversation needing attention', `${primaryModifier}+Shift`, 'A'], ['Show or hide Changes', `${primaryModifier}+Shift`, 'D'], ['Show or hide Context', primaryModifier, 'I'], ['Collapse or expand sidebar', primaryModifier, '\\'], ['Switch between chat and terminal', primaryModifier, '`'], ['Stop the current turn', primaryModifier, '.'], ['Previous conversation', 'Alt', '↑ or K'], ['Next conversation', 'Alt', '↓ or J'], ['Allow approval once', 'Y or A', ''], ['Decline approval', 'N', ''], ['Decline and stop turn', 'Shift', 'N'], ['Send message', 'Enter', ''], ['Add a new line', 'Shift', 'Enter'], ['Rename conversation', 'F2', ''], ['Show this shortcut list', '?', ''],
            ].concat([['Previous changed file', '[', ''], ['Next changed file', ']', '']]).map(([action, modifier, key]) => <div className="keyboard-shortcut-row" key={action}><span>{action}</span><span>{modifier && modifier.split('+').map((part) => <kbd key={part}>{part}</kbd>)}{key && <kbd>{key}</kbd>}</span></div>)}</div>
          </>}
        </section>
      </div>
      <DialogFooter><Button variant="outline" onClick={onClose}><Settings2 size={14} />Done</Button></DialogFooter>
    </DialogContent>
  </Dialog>
}

export { SettingsDialog }
