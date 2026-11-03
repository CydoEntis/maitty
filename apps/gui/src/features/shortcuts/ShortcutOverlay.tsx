import { Dialog as DialogPrimitive } from 'radix-ui'
import { HOTKEY_GROUPS, hotkeyRows, isModifierSymbol, type Chord } from '@/features/workspace/model/shortcuts'

function Keycaps({ chord }: { chord: Chord }) {
  return <span className="mx-wel-chord">{chord.map((key) => <kbd key={key} className={`mx-keycap${isModifierSymbol(key) ? ' is-symbol' : ''}`}>{key}</kbd>)}</span>
}

/** Every shortcut the window handles, grouped, from the same table Settings > Hotkeys reads. */
export function ShortcutOverlay({ mac, portalContainer, onClose }: { mac: boolean; portalContainer: HTMLElement | null; onClose: () => void }) {
  const rows = hotkeyRows(mac)
  return (
    <DialogPrimitive.Root open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogPrimitive.Portal container={portalContainer}>
        <DialogPrimitive.Overlay className="mx-wel-scrim" />
        <DialogPrimitive.Content className="mx-shortcuts" aria-describedby={undefined}>
          <header className="mx-shortcuts-head">
            <DialogPrimitive.Title className="mx-shortcuts-title">Keyboard shortcuts</DialogPrimitive.Title>
            <DialogPrimitive.Close className="mx-settings-close" aria-label="Close shortcuts">Close<kbd>esc</kbd></DialogPrimitive.Close>
          </header>
          <div className="mx-shortcuts-grid">
            {HOTKEY_GROUPS.map((group) => (
              <section key={group} aria-label={group}>
                <h3 className="mx-menu-caps">{group}</h3>
                <ul className="mx-shortcuts-list">
                  {rows.filter((row) => row.group === group).map((row) => (
                    <li key={row.action}>
                      <span>{row.action}</span>
                      <span className="mx-shortcuts-chords">
                        {row.chords.map((chord) => <Keycaps key={chord.join('+')} chord={chord} />)}
                      </span>
                    </li>
                  ))}
                </ul>
              </section>
            ))}
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}
