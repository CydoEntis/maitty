import type { ReactNode } from 'react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { BYPASS_MODE, DANGER_MODES, MODE_FACTS, modeLabel, PICKABLE_MODES } from '../chatMeta'

type Props = {
  /** The button that opens the menu. */
  children: ReactNode
  /** The agent's name, said in the menu's heading. */
  agent: string
  /** What the menu is for, read out: "Permission mode" in a chat, "Starting permission mode" before one. */
  label: string
  mode: string
  /** The modes to offer; Claude Code's when not given. */
  modes?: readonly string[]
  onPick: (mode: string) => void
  /** Why Bypass permissions cannot be picked: a few words for its row, and the whole reason for its tooltip. Absent when it can. */
  bypassOff?: { note: string; why: string } | undefined
}

/** The permission modes of a chat: a row each, with what the mode does, opening up from the tray. */
export function PermissionMenu({ children, agent, label, mode, modes = PICKABLE_MODES, onPick, bypassOff }: Props) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>{children}</DropdownMenuTrigger>
      {/* Mounted in the window root, as the model menu is, so it reads the theme's colours. */}
      <DropdownMenuContent side="top" align="start" container={document.querySelector<HTMLElement>('.mx-app')} aria-label={label} className="mx-pm text-fg">
        <p className="mx-pm-head m-0"><strong>Permissions</strong><span>{agent}</span></p>
        <DropdownMenuRadioGroup value={mode} onValueChange={onPick}>
          {modes.map((key) => {
            const off = key === BYPASS_MODE && bypassOff !== undefined
            const tag = off ? bypassOff.note : MODE_FACTS[key]?.tag
            return (
              <DropdownMenuRadioItem key={key} value={key} disabled={off} title={off ? bypassOff.why : undefined} className={`mx-pm-mode focus:bg-transparent focus:text-fg${DANGER_MODES.includes(key) ? ' is-danger' : ''}`}>
                <span className="mx-pm-text">
                  <span className="mx-pm-name">{modeLabel(key)}{tag !== undefined && <small>{tag}</small>}</span>
                  <span className="mx-pm-what">{MODE_FACTS[key]?.what}</span>
                </span>
              </DropdownMenuRadioItem>
            )
          })}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
