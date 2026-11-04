import { isModifierSymbol, type Chord } from '@/features/workspace/model/shortcuts'

/**
 * One key combination as a row of keycaps; modifier symbols (⌘ ⇧ ⌃ ⌥) are set in the interface font.
 * `labelled` names the group for a screen reader; leave it off inside a control that already has a name.
 */
export function Keycaps({ chord, className, labelled = false }: { chord: Chord; className?: string; labelled?: boolean }) {
  return (
    <span className={className} {...(labelled ? { role: 'group', 'aria-label': chord.join(' ') } : {})}>
      {chord.map((key) => <kbd key={key} className={`mx-keycap${isModifierSymbol(key) ? ' is-symbol' : ''}`}>{key}</kbd>)}
    </span>
  )
}
