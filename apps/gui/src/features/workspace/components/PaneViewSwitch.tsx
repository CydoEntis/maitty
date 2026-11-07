import type { PaneView } from '../model/chatView'
import { useTipWithKeys } from '@/features/shortcuts/platform'
import { Icon } from './glyphs'

type Props = {
  view: PaneView
  onChange: (view: PaneView) => void
}

/**
 * The terminal | chat switch of an agent's pane. Choosing a view changes what the pane shows and
 * sends nothing to the daemon: both views read the same session. The thumb moves with a transform
 * only, and Reduce motion turns the move off at the window root.
 */
export function PaneViewSwitch({ view, onChange }: Props) {
  const tip = useTipWithKeys()
  const choices: ReadonlyArray<{ value: PaneView; label: string }> = [
    { value: 'terminal', label: tip('Terminal view', 'chat-toggle') },
    { value: 'chat', label: tip('Chat view', 'chat-toggle') },
  ]
  return (
    <div role="radiogroup" aria-label="Pane view" className={`mx-view-switch${view === 'chat' ? ' is-chat' : ''}`} onMouseDown={(event) => { event.stopPropagation() }}>
      <span aria-hidden="true" className="mx-view-thumb" />
      {choices.map(({ value, label }) => (
        <button key={value} type="button" role="radio" aria-checked={view === value} aria-label={label} title={label} className="mx-view-option" onClick={() => { if (view !== value) onChange(value) }}>
          <Icon name={value === 'terminal' ? 'prompt' : 'chat'} size={14} strokeWidth={2} />
        </button>
      ))}
    </div>
  )
}
