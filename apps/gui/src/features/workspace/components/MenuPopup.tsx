import { useRef } from 'react'
import { EDGE_PX } from './menuPoint'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'

/** One row of a menu, or a separator line between groups of rows. */
export type MenuEntry =
  | 'separator'
  | {
      label: string
      /** A key or a word shown dim at the right. */
      hint?: string
      run?: () => void
      /** Red. */
      danger?: boolean
      disabled?: boolean
      /** Why it is disabled, as its tooltip. */
      why?: string
    }

type Props = {
  /** The menu's dim heading: what it is for (a session's name, a machine). */
  title: string
  at: { x: number; y: number }
  items: readonly MenuEntry[]
  /** Where the menu mounts, so it reads the theme's colours. */
  portalContainer: HTMLElement | null
  onClose: () => void
}

/**
 * A right-click or ⋯ menu, opened at the pointer. It is the shared dropdown menu anchored to an empty point:
 * Radix keeps it inside the window, puts focus on the first item, moves with the arrows, closes on Esc or a
 * click outside, and focus returns to where it was.
 */
/** The first item takes focus, as a menu opened from the keyboard does, whichever way this one was opened. */
function focusFirstItem(menu: HTMLDivElement | null): void {
  if (menu === null) return
  queueMicrotask(() => { menu.querySelector<HTMLElement>('[data-radix-collection-item]:not([data-disabled])')?.focus() })
}

export function MenuPopup({ title, at, items, portalContainer, onClose }: Props) {
  const returnTo = useRef(document.activeElement instanceof HTMLElement ? document.activeElement : null)
  const choose = (entry: Exclude<MenuEntry, 'separator'>) => {
    onClose()
    entry.run?.()
  }
  return (
    <DropdownMenu open modal={false} onOpenChange={(open) => { if (!open) onClose() }}>
      <DropdownMenuTrigger asChild>
        <span aria-hidden="true" style={{ position: 'fixed', left: at.x, top: at.y, width: 0, height: 0, pointerEvents: 'none' }} />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        container={portalContainer}
        aria-label={title}
        aria-labelledby={undefined}
        loop
        side="bottom"
        align="start"
        sideOffset={0}
        collisionPadding={EDGE_PX}
        className="mx-ctx"
        style={{ position: 'static' }}
        ref={focusFirstItem}
        onCloseAutoFocus={(event) => {
          event.preventDefault()
          if (returnTo.current?.isConnected === true) returnTo.current.focus()
        }}
        onContextMenu={(event) => { event.preventDefault() }}
      >
        <span className="mx-ctx-title">{title}</span>
        {items.map((entry, index) => entry === 'separator'
          ? <DropdownMenuSeparator key={`sep-${String(index)}`} className="mx-ctx-sep" />
          : (
            <DropdownMenuItem
              key={entry.label}
              className={`mx-ctx-item${entry.danger === true ? ' is-danger' : ''}`}
              disabled={entry.disabled === true}
              title={entry.disabled === true ? entry.why : undefined}
              onSelect={() => { choose(entry) }}
            >
              <span className="mx-ctx-label">{entry.label}</span>
              {entry.hint !== undefined && <span className="mx-ctx-hint" aria-hidden="true">{entry.hint}</span>}
            </DropdownMenuItem>
          ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
