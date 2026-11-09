import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { SettingsPreferences } from '../preferences'
import { machineHeads, type RemoteMachine } from '@/features/workspace/model/machines'
import { DesktopSettings, type DesktopSettingsSection, type MachinesInfo } from './DesktopSettings'

const PREFS: SettingsPreferences = { accent: 'teal', spinner: 'braille', restoreLayout: true, reduceMotion: false, animateSidebarActivity: true, compactSidebar: false, toastPosition: 'bottom-right', notifyNeedsYou: true, notifyFinished: true, notifyError: true, notificationSound: false, terminalFontSize: 13, terminalFontFamily: '', terminalFontWeight: 400, terminalFontWeightBold: 700, startFolder: '', themeMode: 'dark', darkTheme: 'Maitty Night', lightTheme: 'Maitty Day', cursorStyle: 'block', scrollback: 10000, scrollSpeed: 1, density: 'comfortable', sidebarTextSize: 13.5, tabTextSize: 13.5, sidebarWidth: 264, sidebarSide: 'left', sidebarOrder: 'creation', terminalScreenReader: false, autoDownloadUpdates: true, restartWhenIdle: false, monthlyBudget: null, iconSize: 'medium', iconColor: 'color', statusBar: true, chatViewDefault: { claude: 'terminal', codex: 'terminal', gemini: 'terminal' }, defaultAgent: 'claude', newSessionView: 'chat', chatFullAccessDefault: false, perfOverlay: false, terminalPlainDrawing: false, terminalSeeThrough: false, backgroundGlow: false, cursorBlink: false, style: 'glass' }

function info(overrides: Partial<MachinesInfo> = {}): MachinesInfo {
  return { known: true, list: [], onRetry: vi.fn(), onRemove: vi.fn(), onGrant: vi.fn(), onUpdate: vi.fn(() => Promise.resolve()), onCheckActivity: vi.fn(() => Promise.resolve(null)), onCopyCommand: vi.fn(), add: { windows: false, loadHosts: () => Promise.resolve([]), onAdd: vi.fn(), onCancel: vi.fn() }, ...overrides }
}

function renderSettings(section: DesktopSettingsSection, overrides: Partial<Parameters<typeof DesktopSettings>[0]> = {}) {
  const onChange = vi.fn()
  const onSection = vi.fn()
  const onPreviewTheme = vi.fn()
  const onShowWelcome = vi.fn()
  render(<QueryClientProvider client={new QueryClient()}><DesktopSettings usage={{ budget: null, onBudget: vi.fn(), sessionForTerm: () => null, onOpenSession: vi.fn() }} section={section} onSection={onSection} preferences={PREFS} onChange={onChange} systemReducedMotion={false} systemDark onPreviewTheme={onPreviewTheme} onShowWelcome={onShowWelcome} platform="linux" daemon={{ online: true, sessionCount: 3 }} machines={info()} appVersion="0.64.0" validateFolder={() => Promise.resolve(true)} onClose={vi.fn()} {...overrides} /></QueryClientProvider>)
  return { onChange, onSection, onPreviewTheme, onShowWelcome }
}

describe('desktop settings', () => {
  it('groups the sections the way the design does', () => {
    renderSettings('general')
    const nav = screen.getByRole('navigation', { name: 'Settings sections' })
    expect(nav).toHaveTextContent('ESSENTIALSGeneralGTERMINALText & cursorTAppearanceAWORKSPACEUsage & costUNotificationsNMachinesMHotkeys?ADVANCEDDaemonDPerformanceP')
    expect(within(nav).getByRole('button', { name: /General/ })).toHaveAttribute('aria-current', 'page')
  })

  it('closes with a plain icon button, not a labelled one', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    renderSettings('general', { onClose })
    const close = screen.getByRole('button', { name: 'Close settings' })

    expect(close).toHaveAttribute('title', 'Close (Esc)')
    expect(close).toHaveTextContent('')
    await user.click(close)

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('picks the start folder with the system picker and saves it like a typed one', async () => {
    const user = userEvent.setup()
    const onChooseFolder = vi.fn().mockResolvedValue('/home/casey/dev')
    const { onChange } = renderSettings('general', { onChooseFolder })

    await user.click(screen.getByRole('button', { name: 'Browse…' }))

    expect(onChooseFolder).toHaveBeenCalledTimes(1)
    expect(screen.getByRole('textbox', { name: 'Start folder' })).toHaveValue('/home/casey/dev')
    await waitFor(() => { expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ startFolder: '/home/casey/dev' })) })
  })

  it('leaves the start folder alone when the picker is cancelled', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('general', { onChooseFolder: vi.fn().mockResolvedValue(null) })
    const before = screen.getByRole<HTMLInputElement>('textbox', { name: 'Start folder' }).value

    await user.click(screen.getByRole('button', { name: 'Browse…' }))

    expect(screen.getByRole('textbox', { name: 'Start folder' })).toHaveValue(before)
    expect(onChange).not.toHaveBeenCalled()
  })

  it('sets which view a Claude, Codex or Gemini session opens in, one agent at a time', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('general')

    const claude = screen.getByRole('radiogroup', { name: 'Claude sessions open in' })
    expect(within(claude).getByRole('radio', { name: 'Terminal' })).toBeChecked()
    await user.click(within(claude).getByRole('radio', { name: 'Chat' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ chatViewDefault: { claude: 'chat', codex: 'terminal', gemini: 'terminal' } }))
  })

  it('chooses the default agent and what a new session opens as', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('general')

    const agent = screen.getByRole('radiogroup', { name: 'Default agent' })
    expect(within(agent).getByRole('radio', { name: 'Claude' })).toBeChecked()
    await user.click(within(agent).getByRole('radio', { name: 'Codex' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ defaultAgent: 'codex' }))

    const view = screen.getByRole('radiogroup', { name: 'New sessions open as' })
    expect(within(view).getByRole('radio', { name: 'Chat' })).toBeChecked()
    await user.click(within(view).getByRole('radio', { name: 'Terminal' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ newSessionView: 'terminal' }))
  })

  it('chooses whether new chats get full access', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('general')

    await user.click(screen.getByRole('switch', { name: 'Full access in new chats' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ chatFullAccessDefault: true }))
  })

  it('picks where notifications appear and shows a test card', async () => {
    const user = userEvent.setup()
    const onTestNotice = vi.fn()
    const { onChange } = renderSettings('notifications', { onTestNotice })

    expect(screen.getByRole('button', { name: 'Bottom right' })).toHaveAttribute('aria-pressed', 'true')
    await user.click(screen.getByRole('button', { name: 'Bottom left' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ toastPosition: 'bottom-left' }))
    await user.click(screen.getByRole('button', { name: 'Show a test' }))
    expect(onTestNotice).toHaveBeenCalledTimes(1)
  })

  it('shows the update state with one fitting button and two switches', async () => {
    const user = userEvent.setup()
    const updates = { summary: 'Maitty 0.61.0 is available.', status: 'available' as const, onCheck: vi.fn(), onDownload: vi.fn(), onRestart: vi.fn() }
    const { onChange } = renderSettings('general', { updates })

    expect(screen.getByText('Update available')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Download' }))
    expect(updates.onDownload).toHaveBeenCalledTimes(1)
    expect(screen.getByRole('switch', { name: 'Download updates automatically' })).toBeChecked()
    expect(screen.getByRole('switch', { name: 'Install updates when idle' })).not.toBeChecked()
    await user.click(screen.getByRole('switch', { name: 'Install updates when idle' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ restartWhenIdle: true }))
  })

  it('offers Restart now once the update is downloaded', async () => {
    const user = userEvent.setup()
    const updates = { summary: 'ready', status: 'ready' as const, onCheck: vi.fn(), onDownload: vi.fn(), onRestart: vi.fn() }
    renderSettings('general', { updates })

    await user.click(screen.getByRole('button', { name: 'Restart now' }))

    expect(updates.onRestart).toHaveBeenCalledTimes(1)
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

  it('picks a working spinner from a dropdown of TUI and web spinners', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')
    const button = screen.getByRole('button', { name: /^Working spinner: / })
    expect(button).toHaveTextContent('Braille')
    expect(button).toHaveTextContent('From the Maitty TUI')

    await user.click(button)
    const list = await screen.findByRole('menu', { name: /^Working spinner/ })
    const choices = within(list).getAllByRole('menuitemradio')
    // The preview glyph is Braille text inside the item; the name is what is left.
    expect(choices.map((choice) => choice.textContent.replace(/[\u2800-\u28FF]/g, '').replace(/(From the Maitty TUI|Web icon)$/, ''))).toEqual(['Braille', 'Snake', 'Bars', 'Circle', 'Dots', 'Equalizer', 'Spokes', 'Ping'])
    expect(list).toHaveTextContent('Maitty TUI')
    expect(list).toHaveTextContent('Web')
    expect(within(list).getByRole('menuitemradio', { name: /Braille/ })).toHaveAttribute('aria-checked', 'true')
    expect(list.querySelectorAll('.mx-wspin')).toHaveLength(5)

    await user.click(within(list).getByRole('menuitemradio', { name: /Circle/ }))
    expect(onChange).toHaveBeenLastCalledWith({ ...PREFS, spinner: 'circle' })
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('names a web spinner as a web icon, and closes the list with Esc without closing Settings', async () => {
    const user = userEvent.setup()
    const onClose = vi.fn()
    renderSettings('appearance', { preferences: { ...PREFS, spinner: 'ping' }, onClose })
    const button = screen.getByRole('button', { name: /^Working spinner: / })
    expect(button).toHaveTextContent('Ping')
    expect(button).toHaveTextContent('Web icon')

    await user.click(button)
    await screen.findByRole('menu', { name: /^Working spinner/ })
    await user.keyboard('{Escape}')

    await waitFor(() => { expect(screen.queryByRole('menu')).toBeNull() })
    expect(onClose).not.toHaveBeenCalled()
  })

  it('measures running animations on the Performance page and offers no animation level to choose', () => {
    renderSettings('performance')

    expect(screen.getByText('Animations running')).toBeInTheDocument()
    expect(screen.queryByText('Animations')).not.toBeInTheDocument()
  })

  it('picks a dark and a light theme separately, from searchable dropdowns that list the one in use first', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, darkTheme: 'Gruvbox Dark' } })
    expect(screen.getByText('16 themes installed')).toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Dark theme: Gruvbox Dark' }))
    const dark = (await screen.findByRole('radiogroup', { name: 'Dark themes' }))
    expect(within(dark).getAllByRole('radio')[0]).toHaveTextContent('Gruvbox Dark')
    expect(within(dark).queryByRole('radio', { name: /Maitty Day/ })).not.toBeInTheDocument()
    await user.type(screen.getByRole('textbox', { name: 'Search dark themes' }), 'tokyo')
    expect(within(dark).getAllByRole('radio')).toHaveLength(1)
    await user.click(within(dark).getByRole('radio', { name: /Tokyo Night/ }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ darkTheme: 'Tokyo Night' }))
    expect(screen.queryByRole('radiogroup', { name: 'Dark themes' })).not.toBeInTheDocument()

    await user.click(screen.getByRole('button', { name: 'Light theme: Maitty Day' }))
    expect(within((await screen.findByRole('radiogroup', { name: 'Light themes' }))).getByRole('radio', { name: /GitHub Light/ })).toBeInTheDocument()
  })

  it('switches between System, Dark and Light, and says which one the system picks', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, themeMode: 'system' } })
    expect(screen.getByText('Follows your OS. Right now: dark.')).toBeInTheDocument()

    await user.click(within(screen.getByRole('radiogroup', { name: 'Appearance' })).getByRole('radio', { name: 'Light' }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ themeMode: 'light' }))
  })

  it('opens the theme list outside its card, so the card cannot clip it, and closes it on Escape', async () => {
    const user = userEvent.setup()
    renderSettings('appearance')

    await user.click(screen.getByRole('button', { name: 'Dark theme: Maitty Night' }))
    const list = (await screen.findByRole('radiogroup', { name: 'Dark themes' }))
    expect(list.closest('.mx-scard')).toBeNull()
    expect(list.closest('.mx-picker-menu')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Search dark themes' })).toHaveFocus()

    await user.keyboard('{Escape}')
    expect(screen.queryByRole('radiogroup', { name: 'Dark themes' })).not.toBeInTheDocument()
  })

  it('previews a hovered theme only for the slot that is in use, and ends the preview on leaving', async () => {
    const user = userEvent.setup()
    const { onPreviewTheme } = renderSettings('appearance', { preferences: { ...PREFS, themeMode: 'dark' } })

    await user.click(screen.getByRole('button', { name: 'Dark theme: Maitty Night' }))
    await user.hover(within((await screen.findByRole('radiogroup', { name: 'Dark themes' }))).getByRole('radio', { name: /Gruvbox Dark/ }))
    expect(onPreviewTheme).toHaveBeenLastCalledWith('Gruvbox Dark')
    await user.hover(screen.getByRole('textbox', { name: 'Search dark themes' }))
    expect(onPreviewTheme).toHaveBeenLastCalledWith(null)

    await user.keyboard('{Escape}')
    await user.click(screen.getByRole('button', { name: 'Light theme: Maitty Day' }))
    onPreviewTheme.mockClear()
    await user.hover(within((await screen.findByRole('radiogroup', { name: 'Light themes' }))).getByRole('radio', { name: /GitHub Light/ }))
    expect(onPreviewTheme).not.toHaveBeenCalledWith('GitHub Light')
  })

  it('has no status effect, animation level or working-effect controls', () => {
    renderSettings('appearance')
    expect(screen.queryByRole('radiogroup', { name: 'Animations' })).toBeNull()
    expect(screen.queryByRole('radiogroup', { name: 'Working effect' })).toBeNull()
    expect(screen.queryByRole('switch', { name: 'Needs you' })).toBeNull()
  })

  it('shows only what maittyd reports about itself', () => {
    renderSettings('daemon', { daemon: { online: false, sessionCount: 3 } })
    expect(screen.getByRole('status')).toHaveTextContent('Disconnected')
    expect(screen.getByText('Sessions').parentElement?.parentElement).toHaveTextContent('3')
    expect(screen.queryByText('Version')).not.toBeInTheDocument()
    expect(screen.queryByText('Socket')).not.toBeInTheDocument()
  })

  it('picks the terminal font from a list that starts with the bundled one, and keeps a saved family that is not installed', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('terminal', { preferences: { ...PREFS, terminalFontFamily: 'Fira Code' } })
    const button = screen.getByRole('button', { name: 'Terminal font family: Fira Code' })
    expect(button).toHaveTextContent('Saved choice')

    await user.click(button)
    const list = await screen.findByRole('menu', { name: /^Terminal font family/ })
    expect(within(list).getByRole('menuitemradio', { name: /Fira Code/ })).toHaveAttribute('aria-checked', 'true')
    await user.click(within(list).getByRole('menuitemradio', { name: /Geist Mono/ }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ terminalFontFamily: '' }))
  })

  it('sets the normal and the bold weight of the terminal text', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('terminal')

    await user.click(within(screen.getByRole('radiogroup', { name: 'Terminal text weight' })).getByRole('radio', { name: 'Medium' }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ terminalFontWeight: 500 }))
    await user.click(within(screen.getByRole('radiogroup', { name: 'Terminal bold weight' })).getByRole('radio', { name: 'Extra bold' }))
    expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ terminalFontWeightBold: 800 }))
  })

  it('sets how big the sidebar icons are', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')

    await user.click(within(screen.getByRole('radiogroup', { name: 'Sidebar icon size' })).getByRole('radio', { name: 'Small' }))

    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ iconSize: 'small' }))
  })

  it('offers the four window styles in one dropdown, with the current one checked', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance', { preferences: { ...PREFS, style: 'glass' } })
    const button = screen.getByRole('button', { name: 'Window style: Glass' })

    await user.click(button)
    const list = await screen.findByRole('menu', { name: /^Window style/ })
    expect(within(list).getAllByRole('menuitemradio').map((item) => item.textContent)).toEqual([
      expect.stringContaining('Glass'),
      expect.stringContaining('Mono Type'),
      expect.stringContaining('Console'),
      expect.stringContaining('Retro Pixel'),
    ])
    expect(within(list).getByRole('menuitemradio', { name: /Glass/ })).toHaveAttribute('aria-checked', 'true')

    await user.click(within(list).getByRole('menuitemradio', { name: /Retro Pixel/ }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ style: 'pixel' }))
  })

  it('replays the welcome from General', async () => {
    const user = userEvent.setup()
    const { onShowWelcome } = renderSettings('general')

    await user.click(screen.getByRole('button', { name: 'Show the welcome again' }))

    expect(onShowWelcome).toHaveBeenCalledTimes(1)
  })

  it('turns the status bar on and off from General', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('general')

    await user.click(screen.getByRole('switch', { name: 'Status bar' }))

    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ statusBar: false }))
  })
})

describe('performance section', () => {
  it('shows what it measures and turns the overlay on', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('performance')
    expect(screen.getByRole('region', { name: 'Web view' })).toHaveTextContent('Frames')
    expect(screen.getByRole('region', { name: 'Processes' })).toHaveTextContent('Daemons')
    expect(screen.getByRole('button', { name: 'Copy report' })).toBeInTheDocument()
    await user.click(screen.getByRole('switch', { name: 'Show the performance overlay' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ perfOverlay: true }))
  })

  it('turns terminal drawing without the GPU on, for a web view that shows typed text late', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('performance')
    await user.click(screen.getByRole('switch', { name: 'Draw terminals without the GPU' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ terminalPlainDrawing: true }))
  })
})

describe('appearance section', () => {
  it('turns the see-through terminal on', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')
    await user.click(screen.getByRole('switch', { name: 'See-through terminal' }))
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ terminalSeeThrough: true }))
  })

  it('turns the background glow on, starting from off', async () => {
    const user = userEvent.setup()
    const { onChange } = renderSettings('appearance')
    const glow = screen.getByRole('switch', { name: 'Background glow' })
    expect(glow).toHaveAttribute('aria-checked', 'false')
    await user.click(glow)
    expect(onChange).toHaveBeenCalledWith(expect.objectContaining({ backgroundGlow: true }))
  })
})

function box(kind: RemoteMachine['state']['state'], why: string | null = null): RemoteMachine {
  return { state: { machine: 1, name: 'box', state: kind, why, attempt: null, since: Date.now() - 180_000 }, conversations: [], heard: true }
}

describe('the Machines page', () => {
  const others = (kind: RemoteMachine['state']['state'], why: string | null = null) => machineHeads(new Map([[1, box(kind, why)]])).filter((head) => head.name !== null)

  it('says it is loading until the host has answered', () => {
    renderSettings('machines', { machines: info({ known: false }) })
    expect(screen.getByRole('status')).toHaveTextContent('Loading machines')
  })

  it('says where machines come from when there are none', () => {
    renderSettings('machines')
    expect(screen.getByText(/No other machines/)).toBeInTheDocument()
  })

  it('shows a machine that is down with its word, how long, and its fix in full', () => {
    renderSettings('machines', { machines: info({ list: others('needs-login', 'run ssh box once to log in') }) })
    expect(screen.getByText(/needs login/)).toBeInTheDocument()
    expect(screen.getByText('disconnected since 3m ago')).toBeInTheDocument()
    expect(screen.getByText('run ssh box once to log in')).toBeInTheDocument()
  })

  it('shows a refused machine with the reason, the fix, and Remove to act on it', () => {
    const why = 'a different machine answers at box (id bbbbbbbb instead of aaaaaaaa). If you replaced that machine, open Machines, remove it and add it again; otherwise check the host name and DNS.'
    renderSettings('machines', { machines: info({ list: others('refused', why) }) })
    expect(screen.getByText(/refused/)).toBeInTheDocument()
    expect(screen.getByText(why)).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Remove box' })).toBeEnabled()
  })

  it('offers to copy the update command, and never runs it', async () => {
    const user = userEvent.setup()
    const onCopyCommand = vi.fn()
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box --install` (or set `remote_cmd` to its path), then retry'), onCopyCommand }) })
    expect(screen.getByText('maitty machine update box --install')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Copy the update command for box' }))
    expect(onCopyCommand).toHaveBeenCalledWith('maitty machine update box --install')
  })

  it('asks before updating a machine, says what happens to its sessions, then updates it', async () => {
    const user = userEvent.setup()
    const onUpdate = vi.fn(() => Promise.resolve())
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box`, then retry'), onUpdate }) })
    expect(screen.getByText('box runs another version of Maitty.')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Update Maitty on box' }))
    expect(onUpdate).not.toHaveBeenCalled()
    expect(await screen.findByRole('alert')).toHaveTextContent(/come back with their agents resumed/)
    await user.click(await screen.findByRole('button', { name: 'Update box now' }))
    expect(onUpdate).toHaveBeenCalledWith('box', 'update', false)
  })

  it('installs on a machine with none in one confirmed click, without asking what is open there', async () => {
    const user = userEvent.setup()
    const onUpdate = vi.fn(() => Promise.resolve())
    const onCheckActivity = vi.fn(() => Promise.resolve(null))
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box --install` (or set `remote_cmd` to its path), then retry'), onUpdate, onCheckActivity }) })
    expect(screen.getByText('Maitty is not installed on box.')).toBeInTheDocument()
    expect(screen.queryByText(/set `remote_cmd`/)).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Install Maitty on box' }))
    expect(screen.getByRole('alert')).toHaveTextContent('Puts this version of Maitty on box over ssh')
    await user.click(screen.getByRole('button', { name: 'Install on box now' }))
    expect(onUpdate).toHaveBeenCalledWith('box', 'install', false)
    expect(onCheckActivity).not.toHaveBeenCalled()
  })

  it('does not update when the question is cancelled', async () => {
    const user = userEvent.setup()
    const onUpdate = vi.fn(() => Promise.resolve())
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box`, then retry'), onUpdate }) })
    await user.click(screen.getByRole('button', { name: 'Update Maitty on box' }))
    await user.click(await screen.findByRole('button', { name: 'Cancel' }))
    expect(onUpdate).not.toHaveBeenCalled()
  })

  it('can wait for the machine to be idle before it updates', async () => {
    const user = userEvent.setup()
    const onUpdate = vi.fn(() => Promise.resolve())
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box`, then retry'), onUpdate }) })
    await user.click(screen.getByRole('button', { name: 'Update Maitty on box' }))
    await user.click(await screen.findByRole('button', { name: 'Update box when idle' }))
    expect(onUpdate).toHaveBeenCalledWith('box', 'update', true)
  })

  it('says how many sessions are open and how many are busy before it updates', async () => {
    const user = userEvent.setup()
    const onCheckActivity = vi.fn(() => Promise.resolve({ sessions: 3, busy: 1 }))
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box`, then retry'), onCheckActivity }) })
    await user.click(screen.getByRole('button', { name: 'Update Maitty on box' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('box has 3 sessions open, 1 busy.')
    expect(onCheckActivity).toHaveBeenCalledWith('box')
  })

  it('still asks, and says it could not see what is open, when the machine cannot say', async () => {
    const user = userEvent.setup()
    renderSettings('machines', { machines: info({ list: others('needs-update', 'run `maitty machine update box`, then retry'), onCheckActivity: vi.fn(() => Promise.reject(new Error('no route'))) }) })
    await user.click(screen.getByRole('button', { name: 'Update Maitty on box' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('box could not say what is open on it.')
  })

  it('has no command to copy for a machine that needs nothing installed', () => {
    renderSettings('machines', { machines: info({ list: others('unreachable', 'no answer') }) })
    expect(screen.queryByRole('button', { name: /Copy the update command/ })).toBeNull()
  })

  it('retries a machine that is down, and has nothing to retry on one that is up', async () => {
    const user = userEvent.setup()
    const onRetry = vi.fn()
    const down = renderSettings('machines', { machines: info({ list: others('unreachable', 'no answer'), onRetry }) })
    await user.click(screen.getByRole('button', { name: 'Retry box' }))
    expect(onRetry).toHaveBeenCalledWith('box')
    expect(down.onChange).not.toHaveBeenCalled()
  })

  it('does not offer a retry for a machine that is connected', () => {
    renderSettings('machines', { machines: info({ list: others('connected') }) })
    expect(screen.getByRole('button', { name: 'Retry box' })).toBeDisabled()
  })

  it('asks before removing, and says what stays', async () => {
    const user = userEvent.setup()
    const onRemove = vi.fn()
    renderSettings('machines', { machines: info({ list: others('connected'), onRemove }) })
    await user.click(screen.getByRole('button', { name: 'Remove box' }))
    expect(onRemove).not.toHaveBeenCalled()
    expect(screen.getByRole('alert')).toHaveTextContent('Removes box from this list. Its sessions keep running on box.')
    await user.click(screen.getByRole('button', { name: 'Keep' }))
    expect(onRemove).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: 'Remove box' }))
    await user.click(screen.getByRole('button', { name: 'Remove box' }))
    expect(onRemove).toHaveBeenCalledWith('box')
  })
})

describe('what a machine may do to this computer', () => {
  it('is off until allowed, and each switch asks for just its own power', async () => {
    const user = userEvent.setup()
    const onGrant = vi.fn()
    const list = machineHeads(new Map([[1, box('connected')]])).filter((head) => head.name !== null)
    renderSettings('machines', { machines: info({ list, onGrant }) })
    const clipboard = screen.getByRole('switch', { name: 'Allow box to copy to your clipboard' })
    const notices = screen.getByRole('switch', { name: 'Allow box to notify you' })
    expect(clipboard).toHaveAttribute('aria-checked', 'false')
    expect(notices).toHaveAttribute('aria-checked', 'false')
    await user.click(notices)
    expect(onGrant).toHaveBeenCalledWith('box', 'notices', true)
    await user.click(clipboard)
    expect(onGrant).toHaveBeenLastCalledWith('box', 'clipboard', true)
  })

  it('shows a power the machine has, and turns it off', async () => {
    const user = userEvent.setup()
    const onGrant = vi.fn()
    const on = box('connected')
    const list = machineHeads(new Map([[1, { ...on, state: { ...on.state, notices: true } }]])).filter((head) => head.name !== null)
    renderSettings('machines', { machines: info({ list, onGrant }) })
    const notices = screen.getByRole('switch', { name: 'Allow box to notify you' })
    expect(notices).toHaveAttribute('aria-checked', 'true')
    await user.click(notices)
    expect(onGrant).toHaveBeenCalledWith('box', 'notices', false)
  })
})

describe('adding a machine', () => {
  const hosts = [{ alias: 'build', detail: 'me@build.example' }, { alias: 'lab', detail: 'lab.example' }]
  const added = (name: string) => ({ outcome: 'added' as const, name, version: 'maitty 1.2.3' })

  it('says it is loading the hosts, then lists them', async () => {
    renderSettings('machines', { machines: info({ add: { ...info().add, loadHosts: () => Promise.resolve(hosts) } }) })
    expect(screen.getByText(/Loading hosts/)).toBeInTheDocument()
    expect(await screen.findByRole('button', { name: /build/ })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /lab/ })).toBeInTheDocument()
  })

  it('says so when there are no hosts, and when they cannot be read', async () => {
    renderSettings('machines')
    expect(await screen.findByText(/No hosts in ~\/\.ssh\/config/)).toBeInTheDocument()
  })

  it('shows why the hosts could not be read', async () => {
    renderSettings('machines', { machines: info({ add: { ...info().add, loadHosts: () => Promise.reject(new Error('config unreadable')) } }) })
    expect(await screen.findByRole('alert')).toHaveTextContent('config unreadable')
  })

  it('adds a picked host and says what answered', async () => {
    const user = userEvent.setup()
    const onAdd = vi.fn().mockResolvedValue(added('build'))
    renderSettings('machines', { machines: info({ add: { ...info().add, loadHosts: () => Promise.resolve(hosts), onAdd } }) })
    await user.click(await screen.findByRole('button', { name: /build/ }))
    expect(onAdd).toHaveBeenCalledWith('build', undefined)
    expect(await screen.findByText('Added build. maitty 1.2.3 answered there. Connecting.')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /me@build\.example/ })).toBeNull()
  })

  it('adds the text as typed when Enter is pressed', async () => {
    const user = userEvent.setup()
    const onAdd = vi.fn().mockResolvedValue(added('box'))
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd } }) })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'me@box.example{Enter}')
    expect(onAdd).toHaveBeenCalledWith('me@box.example', undefined)
  })

  it('shows the words of a machine that cannot be added, and adds nothing', async () => {
    const user = userEvent.setup()
    const onAdd = vi.fn().mockRejectedValue(new Error('nope.example: could not resolve'))
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd } }) })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'nope.example{Enter}')
    expect(await screen.findByRole('alert')).toHaveTextContent('nope.example: could not resolve')
  })

  it('asks for a name when none can be made, then adds with it', async () => {
    const user = userEvent.setup()
    const onAdd = vi.fn().mockResolvedValueOnce({ outcome: 'needs-name' }).mockResolvedValueOnce(added('lab'))
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd } }) })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'me@{Enter}')
    await user.type(await screen.findByRole('textbox', { name: 'Machine name' }), 'lab{Enter}')
    expect(onAdd).toHaveBeenLastCalledWith('me@', 'lab')
  })

  it('shows checking with a way to cancel while ssh is looked at', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn()
    const onAdd = vi.fn().mockReturnValue(new Promise(() => undefined))
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd, onCancel } }) })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'slow.example{Enter}')
    expect(screen.getByRole('status')).toHaveTextContent('Checking slow.example')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onCancel).toHaveBeenCalled()
  })

  it('ends the look with Esc, and keeps Settings open', async () => {
    const user = userEvent.setup()
    const onCancel = vi.fn()
    const onClose = vi.fn()
    const onAdd = vi.fn().mockReturnValue(new Promise(() => undefined))
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd, onCancel } }), onClose })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'slow.example{Enter}')
    screen.getByRole('button', { name: 'Cancel' }).focus()
    await user.keyboard('{Escape}')
    expect(onCancel).toHaveBeenCalled()
    expect(onClose).not.toHaveBeenCalled()
  })

  it('points at the session to log in with when ssh needs a person', async () => {
    const user = userEvent.setup()
    const onAdd = vi.fn().mockResolvedValue({ outcome: 'login', name: 'box' })
    renderSettings('machines', { machines: info({ add: { ...info().add, onAdd } }) })
    await user.type(screen.getByRole('textbox', { name: 'Host or user@host' }), 'box{Enter}')
    expect(await screen.findByText(/ssh needs you to log in: use the session named “ssh box”/)).toBeInTheDocument()
  })

  it('says that Windows wants keys or an agent, on Windows only', () => {
    renderSettings('machines', { machines: info({ add: { ...info().add, windows: true } }) })
    expect(screen.getByText(/Windows: use ssh keys or an agent/)).toBeInTheDocument()
  })
})
