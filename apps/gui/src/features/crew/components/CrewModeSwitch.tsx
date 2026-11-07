import { Users } from 'lucide-react'
import { Icon } from '@/features/workspace/components/glyphs'
import type { CrewApi } from '../useCrew'
import { needsCount } from '../model/crew'

/** The pill under the filter that swaps the sidebar between fleets and the Captain's crew. */
export function CrewModeSwitch({ crew }: { crew: CrewApi }) {
  const { mode } = crew.state
  const waiting = needsCount(crew.state)
  return (
    <div className="mx-crew-modes" role="radiogroup" aria-label="Sidebar mode">
      <button type="button" role="radio" aria-checked={mode === 'fleets'} className={`mx-crew-mode${mode === 'fleets' ? ' is-on' : ''}`} onClick={() => { crew.setMode('fleets') }}>
        <Icon name="fleets" size={13} />Fleets
      </button>
      <button type="button" role="radio" aria-checked={mode === 'crew'} className={`mx-crew-mode${mode === 'crew' ? ' is-on' : ''}`} onClick={() => { crew.setMode('crew') }}>
        <Users size={13} strokeWidth={2} aria-hidden="true" />Crewmates
        {waiting > 0 && mode !== 'crew' && <span className="mx-crew-mode-count" aria-label={`${String(waiting)} need you`}>{waiting}</span>}
      </button>
    </div>
  )
}
