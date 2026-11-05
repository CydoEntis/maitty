import { useState } from 'react'
import type { SessionUsage, UsageFigures } from '@/api/usage'
import { AppIcon, Icon } from '@/features/workspace/components/glyphs'
import {
  ariaSortOf, DEFAULT_SORT, formatCost, formatLastActive, formatTokenCount, formatWorking, nextSort, sessionLabel, sortSessions, THIS_COMPUTER,
  type SortKey, type SortState,
} from './model'

/** The pane a report row's `term` is showing now, when this window has it. */
export type OpenSession = { id: string; name: string }

type Column = { key: SortKey; label: string; numeric: boolean }

const COLUMNS: readonly Column[] = [
  { key: 'session', label: 'Session', numeric: false },
  { key: 'project', label: 'Project', numeric: false },
  { key: 'machine', label: 'Machine', numeric: false },
  { key: 'agent', label: 'Agent', numeric: false },
  { key: 'tokensIn', label: 'Tokens in', numeric: true },
  { key: 'tokensOut', label: 'Tokens out', numeric: true },
  { key: 'cost', label: 'Cost', numeric: true },
  { key: 'turns', label: 'Turns', numeric: true },
  { key: 'workingSecs', label: 'Time working', numeric: true },
  { key: 'lastDay', label: 'Last active', numeric: true },
]

type TableProps = {
  rows: readonly SessionUsage[]
  totals: UsageFigures
  /** The report's last day, which "Last active" counts back from. */
  today: string
  sessionForTerm: (term: number) => OpenSession | null
  onOpenSession: (id: string) => void
}

/** Every session in the window. Every header sorts; a row opens its pane only when that pane is still open. */
export function UsageTable({ rows, totals, today, sessionForTerm, onOpenSession }: TableProps) {
  const [sort, setSort] = useState<SortState>(DEFAULT_SORT)
  const openOf = (row: SessionUsage): OpenSession | null => (row.term === null ? null : sessionForTerm(row.term))
  const labelOf = (row: SessionUsage) => sessionLabel(row, openOf(row)?.name ?? null)
  const sorted = sortSessions(rows, sort, labelOf)

  return (
    <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-pane-line">
      <table className="w-full border-collapse text-[12.5px]">
        <caption className="sr-only">Sessions in this period. Press a column header to sort by it.</caption>
        <thead>
          <tr>
            {COLUMNS.map((column) => (
              <th
                key={column.key}
                scope="col"
                aria-sort={ariaSortOf(sort, column.key)}
                className={`sticky top-0 z-10 whitespace-nowrap border-b border-pane-line bg-raise px-3 py-2 font-medium text-dim ${column.numeric ? 'text-right' : 'text-left'}`}
              >
                <button
                  type="button"
                  className={`inline-flex items-center gap-1 rounded-sm text-inherit hover:text-fg ${column.numeric ? 'flex-row-reverse' : ''}`}
                  onClick={() => { setSort((current) => nextSort(current, column.key)) }}
                >
                  {column.label}
                  <span className="inline-flex w-3" aria-hidden="true">
                    {sort.key === column.key && <Icon name={sort.direction === 'ascending' ? 'arrowUp' : 'arrowDown'} size={11} />}
                  </span>
                </button>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => {
            const open = openOf(row)
            return (
              <tr
                key={row.session}
                className={`border-b border-pane-line last:border-b-0 ${open ? 'cursor-pointer hover:bg-hover' : ''}`}
                onClick={open ? () => { onOpenSession(open.id) } : undefined}
              >
                <td className="px-3 py-2">
                  <span className="flex min-w-0 items-center gap-2">
                    <AppIcon process={row.agent} size={22} />
                    {open
                      ? <button type="button" className="min-w-0 truncate rounded-sm text-left font-medium text-fg hover:underline" onClick={(event) => { event.stopPropagation(); onOpenSession(open.id) }}>{labelOf(row)}</button>
                      : <span className="min-w-0 truncate text-fg2" title="This session is closed">{labelOf(row)}</span>}
                  </span>
                </td>
                <td className="px-3 py-2 text-fg2">{row.project}</td>
                <td className="whitespace-nowrap px-3 py-2 text-fg2">{THIS_COMPUTER}</td>
                <td className="px-3 py-2 text-fg2">{row.agent}</td>
                <Figure>{formatTokenCount(row.tokensIn)}</Figure>
                <Figure>{formatTokenCount(row.tokensOut)}</Figure>
                <Figure>{formatCost(row.cost)}</Figure>
                <Figure>{String(row.turns)}</Figure>
                <Figure>{formatWorking(row.workingSecs)}</Figure>
                <Figure>{formatLastActive(row.lastDay, today)}</Figure>
              </tr>
            )
          })}
        </tbody>
        <tfoot>
          <tr className="border-t border-pane-line bg-raise font-medium text-fg">
            <th scope="row" colSpan={4} className="px-3 py-2 text-left">Total</th>
            <Figure>{formatTokenCount(totals.tokensIn)}</Figure>
            <Figure>{formatTokenCount(totals.tokensOut)}</Figure>
            <Figure>{formatCost(totals.cost)}</Figure>
            <Figure>{String(totals.turns)}</Figure>
            <Figure>{formatWorking(totals.workingSecs)}</Figure>
            <td />

          </tr>
        </tfoot>
      </table>
    </div>
  )
}

/** A right-aligned number in mono with tabular figures, so columns line up. */
function Figure({ children }: { children: string }) {
  return <td className="whitespace-nowrap px-3 py-2 text-right font-mono tabular-nums text-fg">{children}</td>
}
