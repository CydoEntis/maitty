import { Icon } from '@/features/workspace/components/glyphs'
import type { TodoEvent, TodoItemState } from '../chatEvents'
import { todoProgress } from '../stepInfo'

const CHECK_SIZE = 13
const CHECK_STROKE = 2.8

/**
 * The mark of one item. The current one is a lit ring, not the spinner: the history does not say
 * whether an old plan's current item is still being worked on, and a past plan must not spin for good.
 */
function ItemMark({ state }: { state: TodoItemState }) {
  if (state === 'done') return <span className="mx-plan-mark" role="img" aria-label="Done"><Icon name="check" size={CHECK_SIZE} strokeWidth={CHECK_STROKE} /></span>
  return <span className="mx-plan-mark" role="img" aria-label={state === 'current' ? 'In progress' : 'To do'}><span className={`mx-plan-ring${state === 'current' ? ' is-current' : ''}`} /></span>
}

/** The agent's own plan as the history carried it: how much is done, and each item. Maitty adds no items. */
export function PlanCard({ event }: { event: TodoEvent }) {
  if (event.items === null) return <p className="mx-step-absent">The agent kept a plan here, but the history didn&apos;t save it.</p>
  const progress = todoProgress(event.items)
  return (
    <div role="group" aria-label="Plan" className="mx-step-card">
      <div className="mx-plan-head">
        <span className="mx-plan-title">Plan</span>
        <span className="mx-plan-count">{progress.done} of {progress.total} done</span>
        <span className="mx-spacer" />
        {progress.total > 0 && (
          <span className="mx-plan-bar" aria-hidden="true">
            <span style={{ transform: `scaleX(${String(progress.done / progress.total)})` }} />
          </span>
        )}
      </div>
      <ul className="mx-plan-items">
        {event.items.map((item, index) => (
          // The list is the agent's own, drawn whole each time, so an item's place is its identity.
          <li key={index} className={`mx-plan-item is-${item.state}`}>
            <ItemMark state={item.state} />
            <span>{item.text}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}
