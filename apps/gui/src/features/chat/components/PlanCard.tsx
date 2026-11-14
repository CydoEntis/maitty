import { Card, CardHead } from '@/components/ui/card'
import { Counter, Progress } from '@/components/ui/progress'
import { Icon } from '@/features/workspace/components/glyphs'
import type { TodoEvent, TodoItemState } from '../chatEvents'
import { todoProgress } from '../stepInfo'

const CHECK_SIZE = 11
const CHECK_STROKE = 2.8

/**
 * The mark of one item. The current one is a lit ring, not the spinner: the history does not say
 * whether an old plan's current item is still being worked on, and a past plan must not spin for good.
 */
function ItemMark({ state }: { state: TodoItemState }) {
  if (state === 'done') return <span className="mx-plan-mark is-done" role="img" aria-label="Done"><Icon name="check" size={CHECK_SIZE} strokeWidth={CHECK_STROKE} /></span>
  return <span className={`mx-plan-mark is-${state}`} role="img" aria-label={state === 'current' ? 'In progress' : 'To do'}>{state === 'current' && <span className="mx-plan-ring" />}</span>
}

/** The agent's own plan as the history carried it: how much is done, and each item. Maitty adds no items. */
export function PlanCard({ event }: { event: TodoEvent }) {
  if (event.items === null) return <p className="mx-step-absent">The agent kept a plan here, but the history didn&apos;t save it.</p>
  const progress = todoProgress(event.items)
  return (
    <Card role="group" aria-label="Plan">
      <CardHead>
        <span>Plan</span>
        <Counter>{progress.done} of {progress.total} done</Counter>
        <span className="mx-spacer" />
        {progress.total > 0 && <Progress value={progress.done} max={progress.total} label="Plan progress" className="mx-plan-bar" />}
      </CardHead>
      <ul className="mx-plan-items">
        {event.items.map((item, index) => (
          // The list is the agent's own, drawn whole each time, so an item's place is its identity.
          <li key={index} className={`mx-plan-item is-${item.state}`}>
            <ItemMark state={item.state} />
            <span className="mx-plan-text">{item.text}</span>
            {item.state === 'current' && <span className="mx-plan-now">in progress</span>}
          </li>
        ))}
      </ul>
    </Card>
  )
}
