import { useState } from 'react'
import { TitleBar } from '@/components/TitleBar'
import { useShortcutText, useTipWithKeys } from '@/features/shortcuts/platform'
import { activeArrangement } from '../model/layout'
import { isShell } from '../model/sidebar'
import { useWorkspace } from '../workspaceContext'
import { Icon } from './glyphs'
import { MenuPopup, type MenuEntry } from './MenuPopup'

/** The gap between the logo button and the menu under it. */
const MENU_GAP_PX = 4

/** One of the title bar's view toggles: on in `--fg`, off in `--dim`, its state in `aria-pressed`. */
function Toggle({ label, tip, pressed, icon, onClick }: { label: string; tip: string; pressed: boolean; icon: 'panelLeft' | 'panelBottom' | 'panelRight'; onClick: () => void }) {
  return (
    <button type="button" className={`mx-tbar-toggle${pressed ? ' is-on' : ''}`} aria-label={label} title={tip} aria-pressed={pressed} onClick={onClick}>
      <Icon name={icon} />
    </button>
  )
}

/**
 * The window's title bar as the workspace fills it: the Maitty menu behind the logo (New chat, Open a folder, Search,
 * Settings and the rest that starts something), and the Sidebar, Terminal and Changes and files toggles for the
 * focused session.
 */
export function AppTitleBar() {
  const tip = useTipWithKeys()
  const shortcutText = useShortcutText()
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)
  const { root, canStart, newSession, newTerminal, focusedMachineNumber, setPaletteOpen, setSettingsSection, setRecentOpen, sidebarMode, drawerShown, toggleSidebar, focusMode, focusedSession, layout, preferences, agentTerminals, drawerFoldedOf, setDrawerFolded, addAgentTerminal, inspectOpen, inspectView, setInspectView, setInspectOpen, toggleInspect, togglePanel, chooseStartFolder, home, startFolderLabel, setNewChatSeed, setNewChatOpen } = useWorkspace()
  const items: MenuEntry[] = [
    { label: 'New chat', hint: shortcutText('new-agent-chat'), disabled: !canStart, run: () => { newSession(focusedMachineNumber) } },
    { label: 'New shell', hint: shortcutText('new-terminal'), disabled: !canStart, run: () => { newTerminal(focusedMachineNumber) } },
    {
      label: 'Open a folder…',
      disabled: !canStart,
      run: () => {
        void chooseStartFolder(home).then((folder) => {
          if (folder === null) return
          setNewChatSeed({ folder, message: '' })
          setNewChatOpen(true)
        })
      },
    },
    { label: 'Search', hint: shortcutText('palette'), run: () => { setPaletteOpen(true) } },
    'separator',
    { label: 'Recent sessions', run: () => { setRecentOpen(true) } },
    { label: 'Add a remote machine…', run: () => { setSettingsSection('machines') } },
    { label: 'Settings', hint: shortcutText('settings'), run: () => { setSettingsSection('appearance') } },
  ]
  const focusedPane = activeArrangement(layout)?.focus ?? null
  const session = focusedSession ?? null
  const terminals = session === null ? [] : agentTerminals.get(session.id) ?? []
  // A shell's further terminals live in the side panel; an agent's where Settings docks them.
  const underChat = session !== null && !isShell(session) && preferences.terminalDock === 'under'
  const terminalShown = session !== null && terminals.length > 0 && (underChat ? !drawerFoldedOf(session.id) : inspectOpen && inspectView === 'terminal')
  const toggleTerminal = () => {
    if (session === null) return
    if (terminals.length === 0) addAgentTerminal(session.id)
    else if (underChat) setDrawerFolded(session.id, terminalShown)
    else if (focusedPane !== null) togglePanel(focusedPane, session.id, 'terminal')
  }
  const changesShown = inspectOpen && inspectView !== 'terminal'
  const toggleChanges = () => {
    if (changesShown) {
      setInspectOpen(false)
      return
    }
    if (inspectView === 'terminal') setInspectView('changes')
    toggleInspect()
  }
  const start = (
    <button
      type="button"
      className="mx-tbar-logo"
      title="Maitty menu"
      aria-haspopup="menu"
      aria-expanded={menuAt !== null}
      onClick={(event) => {
        const box = event.currentTarget.getBoundingClientRect()
        setMenuAt({ x: box.left, y: box.bottom + MENU_GAP_PX })
      }}
    >
      <img src="/maitty-logo.png" alt="" />
      <span className="mx-tbar-word">m<span className="mx-tbar-ai">AI</span>tty</span>
    </button>
  )
  const end = (
    <span className="mx-tbar-toggles">
      {!focusMode && <Toggle label="Sidebar" tip={tip('Sidebar', 'toggle-sidebar')} pressed={sidebarMode === 'docked' || drawerShown} icon="panelLeft" onClick={toggleSidebar} />}
      <Toggle label="Terminal" tip={session === null ? 'Terminal: open a session first' : 'Terminal'} pressed={terminalShown} icon="panelBottom" onClick={toggleTerminal} />
      <Toggle label="Changes and files" tip={tip('Changes and files', 'inspect')} pressed={changesShown} icon="panelRight" onClick={toggleChanges} />
    </span>
  )
  return (
    <>
      <TitleBar start={start} end={end} />
      {menuAt !== null && <MenuPopup title={`Maitty menu, starting in ${startFolderLabel}`} untitled at={menuAt} items={items} portalContainer={root} onClose={() => { setMenuAt(null) }} />}
    </>
  )
}
