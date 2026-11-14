import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import type { PaneShow } from '../model/chatView'
import { useTipWithKeys } from '@/features/shortcuts/platform'

/** What the Work option says beside its word: how many runs are going, and whether one needs the person. */
export type WorkCount = { going: number; needsYou: boolean }

type Props = {
  /** The views this pane can show; the switch keeps its own order. */
  views: readonly PaneShow[]
  view: PaneShow
  /** Present when the session reported workflow runs. */
  work: WorkCount | null
  onChange: (view: PaneShow) => void
}

const ORDER: readonly PaneShow[] = ['chat', 'terminal', 'work']
const WORD: Record<PaneShow, string> = { chat: 'Chat', terminal: 'Terminal', work: 'Work' }

const isShow = (value: string): value is PaneShow => (ORDER as readonly string[]).includes(value)

/**
 * The Chat · Terminal · Work tabs of an agent's pane header, the chosen one underlined in the accent. Choosing a view
 * changes what the pane shows and sends nothing to the daemon: every view reads the same session. Work shows only for
 * a session that reported workflow runs, with the count of those going and the needs square while one needs the person.
 */
export function PaneViewSwitch({ views, view, work, onChange }: Props) {
  const tip = useTipWithKeys()
  const label = (value: PaneShow) => {
    if (value === 'work') return `Work view${work === null ? '' : `, ${String(work.going)} going${work.needsYou ? ', one needs you' : ''}`}`
    return tip(`${WORD[value]} view`, 'chat-toggle')
  }
  return (
    <RadioGroup aria-label="Pane view" orientation="horizontal" value={view} onValueChange={(next) => { if (isShow(next)) onChange(next) }} className="mx-view-switch" onMouseDown={(event) => { event.stopPropagation() }}>
      {ORDER.filter((value) => views.includes(value)).map((value) => (
        <RadioGroupItem key={value} value={value} aria-label={label(value)} title={label(value)} className="mx-view-option">
          {WORD[value]}
          {value === 'work' && work !== null && work.going > 0 && <span className="mx-view-count" aria-hidden="true">{work.going}</span>}
          {value === 'work' && work?.needsYou === true && <span className="mx-view-needs" aria-hidden="true" />}
        </RadioGroupItem>
      ))}
    </RadioGroup>
  )
}
