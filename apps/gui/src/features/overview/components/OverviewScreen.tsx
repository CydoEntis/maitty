import { useEffect, useRef, type ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { useTipWithKeys } from '@/features/shortcuts/platform'
import { AppIcon, Icon } from '@/features/workspace/components/glyphs'
import { StatusGlyph } from '@/features/workspace/components/PaneStatus'
import { ElapsedSince } from '@/features/workspace/components/times'
import { useReducedMotion } from '@/features/workspace/components/motion'
import { displayName, STATUS_WORD, type SessionStatus, type SessionView } from '@/features/workspace/model/session'
import { STATUS_LABEL } from '@/features/workspace/model/statusText'
import type { SessionTrack } from '@/features/workspace/model/tracking'
import { machineCount, overviewSections, SECTION_LABEL, type OverviewSection } from '../model/overview'

/** Which card the last jump landed on; `key` changes on every jump so the one-shot pulse plays again on the same card. */
export type JumpMark = { id: string; key: number }

type Props = {
  sessions: readonly SessionView[]
  tracks: ReadonlyMap<string, SessionTrack>
  /** The project's name for each session, as the sidebar groups them. */
  projectOf: ReadonlyMap<string, { name: string }>
  jumped: JumpMark | null
  onOpen: (sessionId: string) => void
  onJump: () => void
  onClose: () => void
}

/** The status colour of a card's word and shape. Never the only signal: the shape and the word say it too. */
const TONE: Record<SessionStatus, string> = {
  working: 'text-[var(--accent)]',
  needs: 'text-[var(--warn)]',
  done: 'text-[var(--ok)]',
  error: 'text-[var(--danger)]',
  idle: 'text-dim',
  unknown: 'text-dim',
}

const plural = (count: number, word: string) => `${String(count)} ${word}${count === 1 ? '' : 's'}`

/** The first line of what a stopped agent is asking, else where the pane is: process and folder. */
function contextLine(session: SessionView): string {
  const asking = session.status === 'needs' ? (session.asking.split('\n').find((line) => line.trim() !== '') ?? '').trim() : ''
  return asking !== '' ? asking : `${session.process} · ${session.cwdLabel}`
}

/** Every session on every machine, by what it needs from you. It sits over the panes, which stay mounted underneath. */
export function OverviewScreen({ sessions, tracks, projectOf, jumped, onOpen, onJump, onClose }: Props) {
  const tip = useTipWithKeys()
  const groups = overviewSections(sessions)
  const machines = machineCount(sessions)
  return (
    <section aria-label="Overview" className="absolute inset-0 z-10 overflow-y-auto bg-card p-4 text-foreground">
      <div className="mx-auto flex max-w-[1100px] flex-col gap-5 rounded-xl border border-line2 bg-raise p-5">
        <header className="flex items-center gap-3">
          <h2 className="text-[15px] font-semibold">Overview</h2>
          <span className="text-[12.5px] text-dim">{plural(sessions.length, 'session')} on {plural(machines, 'machine')}</span>
          <span className="flex-1" />
          <Button variant="outline" size="sm" aria-label="Jump to next session that needs you" title={tip('Jump to next', 'jump-needs')} onClick={onJump}>
            Jump to next
          </Button>
          <Button variant="ghost" size="icon-sm" aria-label="Close Overview" title={tip('Close Overview', 'overview')} onClick={onClose}>
            <Icon name="close" size={12} />
          </Button>
        </header>
        {groups.map(({ section, sessions: members }) => (members.length > 0 || section === 'needs') && (
          <Section key={section} section={section} count={members.length}>
            {members.length === 0
              ? <p className="rounded-xl border border-dashed border-line2 px-4 py-3 text-[12.5px] text-dim">Nothing needs you. Agents are working or done.</p>
              : (
                <ul className="m-0 grid list-none grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5 p-0">
                  {members.map((session) => (
                    <li key={session.id} className="contents">
                      <Card key={`${session.id}:${String(jumped?.id === session.id ? jumped.key : 0)}`} session={session} track={tracks.get(session.id)} project={projectOf.get(session.id)?.name ?? null} jump={jumped?.id === session.id ? jumped : null} onOpen={onOpen} />
                    </li>
                  ))}
                </ul>
              )}
          </Section>
        ))}
      </div>
    </section>
  )
}

function Section({ section, count, children }: { section: OverviewSection; count: number; children: ReactNode }) {
  const label = SECTION_LABEL[section]
  return (
    <section aria-label={label} className="flex flex-col gap-2.5">
      <div className="flex items-center gap-2.5">
        <h3 className={cn('text-[11px] font-semibold tracking-[.08em] uppercase', section === 'needs' ? 'text-[var(--warn)]' : 'text-dim')}>{label}</h3>
        <span className="text-[11px] text-dim tabular-nums">{count}</span>
        <span className="h-px flex-1 bg-pane-line" aria-hidden="true" />
      </div>
      {children}
    </section>
  )
}

function Card({ session, track, project, jump, onOpen }: { session: SessionView; track: SessionTrack | undefined; project: string | null; jump: JumpMark | null; onOpen: (id: string) => void }) {
  const reduced = useReducedMotion()
  const ref = useRef<HTMLButtonElement>(null)
  const jumpKey = jump?.key
  useEffect(() => {
    if (jumpKey === undefined) return
    const card = ref.current
    if (!card) return
    card.focus({ preventScroll: true })
    // Keeps the card in view without a jump when motion is reduced. jsdom has no scrollIntoView.
    if ('scrollIntoView' in card) card.scrollIntoView({ block: 'nearest', behavior: reduced ? 'auto' : 'smooth' })
  }, [jumpKey, reduced])
  const word = STATUS_LABEL[session.status]
  const name = displayName({ name: session.name, title: session.title })
  const since = session.status === 'unknown' ? null : track?.changedAt ?? null
  return (
    <button
      ref={ref}
      type="button"
      aria-label={`${name}, ${STATUS_WORD[session.status]}${project ? `, ${project}` : ''}${session.machineName ? `, on ${session.machineName}` : ''}`}
      data-status={session.status}
      className={cn(
        'flex min-w-0 flex-col gap-1.5 rounded-xl border bg-[color-mix(in_srgb,var(--fg)_3%,transparent)] p-3 text-left transition-colors hover:bg-hover',
        session.status === 'needs' ? 'border-[var(--warn-line)]' : 'border-pane-line',
        jump && 'ring-2 ring-[var(--accent)] shadow-[0_0_0_6px_var(--accent-soft)] mx-ov-pulse',
      )}
      onClick={() => { onOpen(session.id) }}
    >
      <span className="flex min-w-0 items-center gap-2">
        <span aria-hidden="true" className={cn('mx-pane-status-icon shrink-0', TONE[session.status])}><StatusGlyph status={session.status} frozen /></span>
        <span className="min-w-0 flex-1 truncate text-[13.5px] font-semibold">{name}</span>
        <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
      </span>
      <span className="flex min-w-0 items-center gap-1.5 text-[12px] text-dim">
        {project && <span className="truncate">{project}</span>}
        {session.branch && <span className="flex min-w-0 items-center gap-1 font-mono text-[11.5px]"><Icon name="branch" size={11} /><span className="truncate">{session.branch}</span></span>}
        {session.machineName && <span className="shrink-0 rounded-md border border-line2 px-1.5 text-[11px] leading-[16px]">{session.machineName}</span>}
      </span>
      <span className="line-clamp-2 min-h-[2lh] break-words text-[12px] text-fg2">{contextLine(session)}</span>
      <span className={cn('text-[12px] font-medium', TONE[session.status])}>
        {word}
        {session.status === 'unknown' && session.lastKnownStatus && ` · last seen ${STATUS_WORD[session.lastKnownStatus]}`}
        {since !== null && <ElapsedSince since={since} prefix=" · " />}
      </span>
    </button>
  )
}
