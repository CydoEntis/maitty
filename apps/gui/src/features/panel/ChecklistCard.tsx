import { Check, Circle } from 'lucide-react'
import { Spinner } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { Checklist } from './panelModel'

type Props = {
  checklist: Checklist
  /** The agent is working now, so the current item shows its spinner and a running time. */
  working: boolean
  /** When the current item became current; `null` when unknown. */
  currentSince: number | null
}

/**
 * The agent's own to-do list (v18 C.2): "Checklist · 2 of 5", a 3px progress bar, done items struck
 * through, the current item bold with a spinner and its time, pending items hollow. Maitty adds no items.
 */
export function ChecklistCard({ checklist, working, currentSince }: Props) {
  const total = checklist.items.length
  return (
    <section aria-label="Checklist" className="rounded-[10px] border border-line2 bg-raise px-3 py-2.5">
      <h3 className="m-0 mb-1.5 text-[11.5px] font-semibold text-fg2">Checklist · {checklist.done} of {total}</h3>
      <div className="mb-2 h-[3px] overflow-hidden rounded-full bg-line2" role="progressbar" aria-label="Checklist progress" aria-valuemin={0} aria-valuemax={total} aria-valuenow={checklist.done}>
        <div className="h-full origin-left bg-brand" style={{ transform: `scaleX(${String(total === 0 ? 0 : checklist.done / total)})` }} />
      </div>
      <ul className="m-0 list-none p-0">
        {checklist.items.map((item, index) => (
          <li key={index} className="flex items-start gap-2 py-0.5 text-[13px]">
            {item.state === 'done'
              ? <Check className="mt-0.5 size-3.5 shrink-0 text-[var(--ok)]" aria-label="Done" />
              : item.state === 'current'
                ? <span className="mt-0.5 flex size-3.5 shrink-0 items-center justify-center text-[var(--accent)]" role="img" aria-label="In progress">{working ? <Spinner className="mx-spin" /> : <Circle className="size-3.5 fill-current" aria-hidden="true" />}</span>
                : <Circle className="mt-0.5 size-3.5 shrink-0 text-dim" aria-label="To do" />}
            <span className={`min-w-0 flex-1 [overflow-wrap:anywhere] ${item.state === 'done' ? 'text-dim line-through' : item.state === 'current' ? 'font-semibold text-fg' : 'text-fg2'}`}>{item.text}</span>
            {item.state === 'current' && working && currentSince !== null && <span className="shrink-0 font-mono text-[11.5px] text-[var(--accent)]"><ElapsedSince since={currentSince} /></span>}
          </li>
        ))}
      </ul>
    </section>
  )
}
