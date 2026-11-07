import { Dialog as DialogPrimitive } from 'radix-ui'
import { HOTKEY_GROUPS, hotkeyRows, type Chord, type Platform } from '@/features/workspace/model/shortcuts'
import { Icon } from '@/features/workspace/components/glyphs'

/** Keys the chat answers itself, so they are not in the window's table (README: In chat). */
const CHAT_KEYS: ReadonlyArray<{ action: string; chord: Chord }> = [
  { action: 'Send', chord: ['Enter'] },
  { action: 'New line', chord: ['Shift', 'Enter'] },
  { action: 'Allow once', chord: ['Y'] },
  { action: 'Allow for session', chord: ['A'] },
  { action: 'Deny', chord: ['N'] },
  { action: 'Stop the agent', chord: ['Esc'] },
]

/** The dialog itself takes the focus as it opens, so no control shows a focus ring nobody asked for. */
const focusDialog = (event: Event) => {
  event.preventDefault()
  if (event.currentTarget instanceof HTMLElement) event.currentTarget.focus()
}

function Row({ action, chord }: { action: string; chord: Chord }) {
  return (
    <div className="mx-keys-row">
      <span>{action}</span>
      <span className="mx-keys-caps">{chord.map((key) => <kbd key={key}>{key}</kbd>)}</span>
    </div>
  )
}

/** Every shortcut the window handles, grouped, from the same table Settings > Hotkeys reads, and the chat's own keys. */
export function ShortcutOverlay({ platform, portalContainer, onClose }: { platform: Platform; portalContainer: HTMLElement | null; onClose: () => void }) {
  const rows = hotkeyRows(platform)
  return (
    <DialogPrimitive.Root open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogPrimitive.Portal container={portalContainer}>
        <DialogPrimitive.Overlay className="mx-dialog-scrim" />
        <DialogPrimitive.Content className="mx-keys-dialog" aria-describedby={undefined} tabIndex={-1} onOpenAutoFocus={focusDialog}>
          <header className="mx-dialog-head">
            <DialogPrimitive.Title className="mx-dialog-title">Keyboard shortcuts</DialogPrimitive.Title>
            <span className="mx-dialog-note">Change them in Settings › Hotkeys</span>
            <DialogPrimitive.Close className="mx-dialog-close" aria-label="Close"><Icon name="close" size={13} /></DialogPrimitive.Close>
          </header>
          <div className="mx-keys-grid">
            {HOTKEY_GROUPS.map((group) => (
              <section key={group} aria-label={group} className="mx-keys-group">
                <h3>{group}</h3>
                {rows.filter((row) => row.group === group).map((row) => <Row key={row.action} action={row.action} chord={row.chords[0] ?? []} />)}
              </section>
            ))}
            <section aria-label="In chat" className="mx-keys-group">
              <h3>In chat</h3>
              {CHAT_KEYS.map((row) => <Row key={row.action} action={row.action} chord={row.chord} />)}
            </section>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
