import { useState, type SyntheticEvent, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { loadUsageReport, type UsagePeriod, type UsageReport } from '@/api/usage'
import { Skeleton } from '@/components/ui/skeleton'
import { useReducedMotion } from '@/features/workspace/components/motion'
import {
  budgetRing, dayBars, DASH, PERCENT, formatCost, formatTokenCount, formatWorking, parseBudget, PERIOD_CHOICES, shortDay, totalTokens, windowLabel,
  type DayBar, type Ring, type RingTone,
} from './model'
import { UsageTable, type OpenSession } from './UsageTable'

/** A modest refresh while the page is open; the daemon is also asked on open and when the period changes. */
export const USAGE_REFRESH_MS = 60_000

const RING_SIZE = 56
const RING_CENTER = 28
const RING_STROKE = 5
/** The stroke sits inside the box, so the circle's diameter is the box less one stroke. */
const RING_DIAMETER = RING_SIZE - RING_STROKE
/** Half the diameter. */
const RING_RADIUS = 25.5
const RING_LENGTH = Math.PI * RING_DIAMETER
/** A bar of a day with tokens never shrinks to nothing, so a small day is still visible. */
const BAR_MIN_PERCENT = 3

const TONE_COLOR: Record<RingTone, string> = { accent: 'var(--accent)', warn: 'var(--warn)', danger: 'var(--danger)' }

type PageProps = {
  /** The monthly budget in dollars, or null for none. */
  budget: number | null
  onBudget: (budget: number | null) => void
  sessionForTerm: (term: number) => OpenSession | null
  onOpenSession: (id: string) => void
}

/** Settings › Usage & cost (v18 D.7): what agents reported using, by session, from the daemon's usage store (ADR 0009). */
export function UsagePage({ budget, onBudget, sessionForTerm, onOpenSession }: PageProps) {
  const [period, setPeriod] = useState<UsagePeriod>('week')

  const query = useQuery({
    queryKey: ['usage', period],
    queryFn: () => loadUsageReport(period),
    refetchInterval: USAGE_REFRESH_MS,
    refetchOnMount: 'always',
    // The page offers Retry itself; hiding failures behind silent retries would leave a blank page for seconds.
    retry: false,
  })
  const report = query.data

  return (
    <section aria-label="Usage and cost" className="flex min-h-0 flex-col gap-4 text-fg">
      <header className="flex flex-wrap items-center gap-3">
        <PeriodSwitch period={period} onChange={setPeriod} />
      </header>
      {report === undefined
        ? query.isError
          ? <LoadFailed onRetry={() => { void query.refetch() }} retrying={query.isFetching} />
          : <Loading />
        : (
          <>
            {query.isError && <RefreshFailed onRetry={() => { void query.refetch() }} retrying={query.isFetching} />}
            <Summary report={report} period={period} budget={budget} onBudget={onBudget} />
            {report.sessionCount === 0
              ? <Empty retentionDays={report.retentionDays} />
              : (
                <>
                  {report.sessionsWithoutCost > 0 && <NoCostNote count={report.sessionsWithoutCost} />}
                  <DayChart report={report} />
                  <UsageTable rows={report.sessions} totals={report.totals} today={report.to} sessionForTerm={sessionForTerm} onOpenSession={onOpenSession} />
                </>
              )}
          </>
        )}
    </section>
  )
}

function PeriodSwitch({ period, onChange }: { period: UsagePeriod; onChange: (period: UsagePeriod) => void }) {
  return (
    <div role="group" aria-label="Period" className="inline-flex rounded-lg bg-raise p-0.5">
      {PERIOD_CHOICES.map((choice) => (
        <button
          key={choice.period}
          type="button"
          aria-pressed={choice.period === period}
          title={choice.window}
          className={`rounded-md px-3 py-1 text-[12.5px] font-medium ${choice.period === period ? 'bg-[var(--accent-soft)] text-fg' : 'text-dim hover:text-fg'}`}
          onClick={() => { onChange(choice.period) }}
        >
          {choice.label}
        </button>
      ))}
    </div>
  )
}

function Summary({ report, period, budget, onBudget }: { report: UsageReport; period: UsagePeriod; budget: number | null; onBudget: (budget: number | null) => void }) {
  const { totals } = report
  const tokens = totalTokens(totals)
  const ring = budgetRing(period, totals.cost, budget)
  return (
    <div className="flex flex-wrap items-center gap-x-8 gap-y-3 rounded-lg border border-pane-line bg-raise px-4 py-3">
      <div className="min-w-0">
        <p className="text-[10.5px] font-semibold uppercase tracking-[.08em] text-dim">{windowLabel(period)} · {shortDay(report.from)} to {shortDay(report.to)}</p>
        <p className="text-[12px] text-dim">Reported by the agents, never estimated</p>
      </div>
      <Stat label="Tokens" value={formatTokenCount(tokens)} detail={tokens === null ? 'none reported' : `${formatTokenCount(totals.tokensIn)} in · ${formatTokenCount(totals.tokensOut)} out`} big />
      <Stat label="Cost" value={formatCost(totals.cost)} detail={totals.cost === null ? 'none reported' : `reported cost · ${String(report.sessionsWithoutCost)} ${report.sessionsWithoutCost === 1 ? 'session' : 'sessions'} without cost`} />
      <Stat label="Turns" value={String(totals.turns)} detail={`${String(report.sessionCount)} ${report.sessionCount === 1 ? 'session' : 'sessions'}`} />
      <Stat label="Time working" value={formatWorking(totals.workingSecs)} detail="" />
      <span className="flex-1" />
      <Budget period={period} budget={budget} ring={ring} onBudget={onBudget} />
    </div>
  )
}

function Stat({ label, value, detail, big = false }: { label: string; value: string; detail: string; big?: boolean }) {
  return (
    <div className="min-w-0">
      <p className="text-[11px] text-dim">{label}</p>
      <p className={`font-mono tabular-nums font-semibold ${big ? 'text-[22px]' : 'text-[17px] text-fg2'}`}>{value}</p>
      {detail !== '' && <p className="text-[11.5px] text-dim">{detail}</p>}
    </div>
  )
}

/** The ring when a budget is set and measurable; otherwise the way to set one. */
function Budget({ period, budget, ring, onBudget }: { period: UsagePeriod; budget: number | null; ring: Ring | null; onBudget: (budget: number | null) => void }) {
  const [editing, setEditing] = useState(false)
  if (editing) return <BudgetForm initial={budget} onSave={(value) => { onBudget(value); setEditing(false) }} onCancel={() => { setEditing(false) }} />
  if (budget === null) {
    return <button type="button" className="text-[12.5px] text-[var(--accent-text)] underline-offset-2 hover:underline" onClick={() => { setEditing(true) }}>Set a monthly budget</button>
  }
  return (
    <div className="flex items-center gap-3">
      {ring && <BudgetRing ring={ring} budget={budget} />}
      <div className="text-[12px] text-dim">
        <p>{formatCost(budget)} monthly budget</p>
        {!ring && <p>{period === 'month' ? 'No cost reported to compare' : 'Choose Month to compare'}</p>}
        <button type="button" className="text-[var(--accent-text)] underline-offset-2 hover:underline" onClick={() => { setEditing(true) }}>Change</button>
      </div>
    </div>
  )
}

function BudgetRing({ ring, budget }: { ring: Ring; budget: number }) {
  return (
    <svg width={RING_SIZE} height={RING_SIZE} viewBox={`0 0 ${String(RING_SIZE)} ${String(RING_SIZE)}`} role="img" aria-label={`${String(ring.percent)}% of the ${formatCost(budget)} monthly budget used`}>
      <circle cx={RING_CENTER} cy={RING_CENTER} r={RING_RADIUS} fill="none" stroke="var(--line2)" strokeWidth={RING_STROKE} />
      <circle
        cx={RING_CENTER}
        cy={RING_CENTER}
        r={RING_RADIUS}
        fill="none"
        stroke={TONE_COLOR[ring.tone]}
        strokeWidth={RING_STROKE}
        strokeLinecap="round"
        strokeDasharray={`${String(RING_LENGTH * ring.fraction)} ${String(RING_LENGTH)}`}
        transform={`rotate(-90 ${String(RING_CENTER)} ${String(RING_CENTER)})`}
      />
      <text x="50%" y="50%" textAnchor="middle" dominantBaseline="central" fill="currentColor" fontSize="12" fontWeight="600">{ring.percent}%</text>
    </svg>
  )
}

function BudgetForm({ initial, onSave, onCancel }: { initial: number | null; onSave: (budget: number | null) => void; onCancel: () => void }) {
  const [text, setText] = useState(initial === null ? '' : String(initial))
  const [invalid, setInvalid] = useState(false)
  const submit = (event: SyntheticEvent) => {
    event.preventDefault()
    const parsed = parseBudget(text)
    if (parsed.ok) onSave(parsed.value)
    else setInvalid(true)
  }
  return (
    <form className="flex flex-wrap items-center gap-2 text-[12.5px]" onSubmit={submit}>
      <label className="flex items-center gap-1 text-dim" htmlFor="usage-budget">Monthly budget $</label>
      <input
        id="usage-budget"
        autoFocus
        inputMode="decimal"
        value={text}
        aria-invalid={invalid}
        aria-describedby={invalid ? 'usage-budget-error' : undefined}
        className="w-24 rounded-md border border-pane-line bg-term px-2 py-1 font-mono text-fg"
        onChange={(event) => { setText(event.target.value); setInvalid(false) }}
      />
      <button type="submit" className="rounded-md bg-[var(--accent)] px-2.5 py-1 font-medium text-[var(--on-accent)]">Save</button>
      <button type="button" className="rounded-md px-2.5 py-1 text-dim hover:text-fg" onClick={() => { onSave(null) }}>No budget</button>
      <button type="button" className="rounded-md px-2.5 py-1 text-dim hover:text-fg" onClick={onCancel}>Cancel</button>
      {invalid && <span id="usage-budget-error" role="alert" className="basis-full text-[var(--danger)]">Enter an amount in dollars, such as 50 or 12.50.</span>}
    </form>
  )
}

/** Shown once: subscription sessions report no cost, and their rows show a dash instead of repeating this. */
function NoCostNote({ count }: { count: number }) {
  return (
    <p role="note" className="rounded-lg border border-pane-line px-3 py-2 text-[12.5px] text-fg2">
      {count === 1 ? '1 session reported no cost' : `${String(count)} sessions reported no cost`}: subscription plans and some agents do not report one, so their cost shows {DASH}. Cost is never estimated.
    </p>
  )
}

function DayChart({ report }: { report: UsageReport }) {
  const bars = dayBars(report.from, report.to, report.days)
  const first = bars[0]
  const last = bars[bars.length - 1]
  if (first === undefined || last === undefined) return null
  return (
    <figure className="flex flex-col gap-1.5" aria-label="Tokens per day">
      <div className="flex h-16 items-end gap-0.5" aria-hidden="true">
        {bars.map((bar) => <Bar key={bar.day} bar={bar} />)}
      </div>
      <figcaption className="flex justify-between text-[11px] text-dim">
        <span>{shortDay(first.day)}</span>
        <span>Tokens per day (input plus output)</span>
        <span>{shortDay(last.day)}</span>
      </figcaption>
      {/* The bars say nothing to a screen reader, so the same figures are here as a table. */}
      <table className="sr-only">
        <caption>Tokens and cost per day</caption>
        <thead><tr><th scope="col">Day</th><th scope="col">Tokens</th><th scope="col">Cost</th></tr></thead>
        <tbody>{bars.map((bar) => <tr key={bar.day}><th scope="row">{bar.day}</th><td>{formatTokenCount(bar.tokens)}</td><td>{formatCost(bar.cost)}</td></tr>)}</tbody>
      </table>
    </figure>
  )
}

function Bar({ bar }: { bar: DayBar }): ReactNode {
  const percent = bar.tokens === null ? 0 : Math.max(BAR_MIN_PERCENT, bar.fraction * PERCENT)
  return (
    <div className="flex h-full min-w-0 flex-1 items-end" title={`${shortDay(bar.day)}: ${formatTokenCount(bar.tokens)} tokens`}>
      <div className="w-full rounded-t-sm bg-[var(--accent)] opacity-80" style={{ height: `${String(percent)}%` }} />
    </div>
  )
}

function Empty({ retentionDays }: { retentionDays: number }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-2 py-12 text-center">
      <h2 className="text-[15px] font-semibold">Nothing recorded yet</h2>
      <p className="max-w-sm text-[12.5px] text-dim">Usage appears here as agent sessions run. Maitty keeps what the agents report for {String(retentionDays)} days and never estimates a figure they did not.</p>
    </div>
  )
}

function Loading() {
  const reduced = useReducedMotion()
  const pulse = reduced ? 'animate-none' : ''
  return (
    <div role="status" aria-label="Loading usage" className="flex flex-col gap-3">
      <Skeleton className={`h-20 w-full ${pulse}`} />
      <Skeleton className={`h-16 w-full ${pulse}`} />
      <Skeleton className={`h-40 w-full ${pulse}`} />
    </div>
  )
}

function LoadFailed({ onRetry, retrying }: { onRetry: () => void; retrying: boolean }) {
  return (
    <div role="alert" className="flex flex-1 flex-col items-center justify-center gap-3 py-12 text-center">
      <h2 className="text-[15px] font-semibold">Could not load usage</h2>
      <p className="max-w-sm text-[12.5px] text-dim">Maitty could not read the usage figures from maittyd. Your sessions are not affected.</p>
      <button type="button" disabled={retrying} className="rounded-md bg-raise px-3 py-1.5 text-[12.5px] font-medium hover:bg-hover" onClick={onRetry}>Retry</button>
    </div>
  )
}

/** A refresh failed while older figures are still on the page: keep them, and say they may be out of date. */
function RefreshFailed({ onRetry, retrying }: { onRetry: () => void; retrying: boolean }) {
  return (
    <p role="alert" className="flex items-center gap-3 rounded-lg border border-[var(--warn-line)] px-3 py-2 text-[12.5px] text-fg2">
      Could not refresh usage; these figures may be out of date.
      <button type="button" disabled={retrying} className="font-medium text-[var(--accent-text)] hover:underline" onClick={onRetry}>Retry</button>
    </p>
  )
}
