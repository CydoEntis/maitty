import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import type { PaneView } from '../model/chatView'
import { useTipWithKeys } from '@/features/shortcuts/platform'

type Props = {
  view: PaneView
  onChange: (view: PaneView) => void
}

/**
 * The Chat · Terminal tabs of an agent's pane header, the chosen one underlined in the accent. Choosing a view changes
 * what the pane shows and sends nothing to the daemon: both views read the same session.
 */
export function PaneViewSwitch({ view, onChange }: Props) {
  const tip = useTipWithKeys()
  const choices: ReadonlyArray<{ value: PaneView; word: string; label: string }> = [
    { value: 'chat', word: 'Chat', label: tip('Chat view', 'chat-toggle') },
    { value: 'terminal', word: 'Terminal', label: tip('Terminal view', 'chat-toggle') },
  ]
  return (
    <RadioGroup aria-label="Pane view" orientation="horizontal" value={view} onValueChange={(next) => { if (next === 'terminal' || next === 'chat') onChange(next) }} className="mx-view-switch" onMouseDown={(event) => { event.stopPropagation() }}>
      {choices.map(({ value, word, label }) => (
        <RadioGroupItem key={value} value={value} aria-label={label} title={label} className="mx-view-option">{word}</RadioGroupItem>
      ))}
    </RadioGroup>
  )
}
