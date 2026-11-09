import { Users } from 'lucide-react'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Icon } from '@/features/workspace/components/glyphs'
import type { CrewApi } from '../useCrew'
import { needsCount } from '../model/crew'

/** The pill under the filter that swaps the sidebar between fleets and the Captain's crew. */
export function CrewModeSwitch({ crew }: { crew: CrewApi }) {
  const { mode } = crew.state
  const waiting = needsCount(crew.state)
  return (
    <RadioGroup className="mx-crew-modes" aria-label="Sidebar mode" orientation="horizontal" value={mode} onValueChange={(next) => { crew.setMode(next === 'crew' ? 'crew' : 'fleets') }}>
      <RadioGroupItem value="fleets" className={`mx-crew-mode${mode === 'fleets' ? ' is-on' : ''}`}>
        <Icon name="fleets" size={13} />Fleets
      </RadioGroupItem>
      <RadioGroupItem value="crew" className={`mx-crew-mode${mode === 'crew' ? ' is-on' : ''}`}>
        <Users size={13} strokeWidth={2} aria-hidden="true" />Crewmates
        {waiting > 0 && mode !== 'crew' && <span className="mx-crew-mode-count" aria-label={`${String(waiting)} need you`}>{waiting}</span>}
      </RadioGroupItem>
    </RadioGroup>
  )
}
