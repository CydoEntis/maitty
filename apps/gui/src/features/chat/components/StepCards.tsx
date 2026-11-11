import { useState } from 'react'
import { ArrowUpRight, Lock } from 'lucide-react'
import { HighlightedLine } from '@/components/HighlightedLine'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { CommandEvent, EditEvent, ToolEvent } from '../chatEvents'
import { isOmitted, redactionNote } from '../redaction'
import { commandMeta, relativePath } from '../stepInfo'
import { toolFacts, type ToolFacts } from '../toolFacts'
import type { WorkStep } from '../workSummary'
import { documentOf } from '../writtenDocument'
import { ChangePill, FileBadge, Pill, StepRow, type MetaTone } from './cardParts'
import { diffRows, type DiffRow } from './diffRows'
import { DocumentCard } from './DocumentCard'

/**
 * The steps an agent takes, each a row that opens to its detail (handoff items 8 to 15). A fact the
 * daemon's history does not carry is said to be unavailable, never guessed. Colours are theme
 * tokens, and nothing here animates except the spinner of a step that is running.
 */

export const FOLD_LINES = 12
export const COMMAND_TAIL_LINES = 3

const STOPPED_WORD = 'stopped'
const FAILED_WORD = 'failed'

/** A muted line saying the daemon hid a secret in the text above it; nothing when it hid none. */
function RedactionNote({ texts }: { texts: readonly (string | null)[] }) {
  const note = redactionNote(...texts)
  return note === null ? null : <p className="mx-step-note">{note}</p>
}

/** Opens by itself when the step failed, until the person opens or closes it themselves. */
function useOpen(byItself: boolean): [boolean, () => void] {
  const [chosen, setChosen] = useState<boolean | null>(null)
  const open = chosen ?? byItself
  return [open, () => { setChosen(!open) }]
}

const looksLikeFile = (target: string) => /^[^\s"'`]+\.[A-Za-z0-9]+$/.test(target)

function ToolTarget({ facts }: { facts: ToolFacts }) {
  switch (facts.kind) {
    case 'file':
    case 'folder':
      return <FileBadge path={facts.kind === 'folder' && !facts.path.endsWith('/') ? `${facts.path}/` : facts.path} />
    case 'pattern':
      return <><code className="mx-code-chip">{facts.pattern}</code>{facts.where !== null && <span className="mx-step-where">in {facts.where}</span>}</>
    case 'address':
      return <span className="mx-step-mono">{facts.address}</span>
    case 'query':
      return <span className="mx-step-quoted">“{facts.query}”</span>
    case 'extension':
    case 'other':
      if (facts.target === null) return null
      return looksLikeFile(facts.target) ? <FileBadge path={facts.target} /> : <span className="mx-step-plain">{facts.target}</span>
  }
}

type ToolStepProps = {
  event: ToolEvent
  /** The session's folder, so paths are shown relative to it. */
  root?: string | null
  waiting?: boolean
}

/** Any tool with no card of its own: a read, a search, a fetch, an extension's tool. Opens to its input and result. */
export function ToolStep({ event, root = null, waiting = false }: ToolStepProps) {
  const [open, toggle] = useOpen(false)
  const facts = toolFacts(event, root)
  const meta = event.status === 'failed' ? FAILED_WORD : event.status === 'interrupted' ? STOPPED_WORD : facts.kind === 'file' ? facts.lines : null
  const result = event.detail.trim()
  return (
    <div className="mx-step">
      <StepRow
        status={event.status}
        verb={facts.kind === 'extension' ? <><span className="mx-step-server">{facts.server}</span>{facts.tool}</> : facts.verb}
        target={<ToolTarget facts={facts} />}
        meta={meta}
        metaTone={event.status === 'failed' ? 'danger' : 'dim'}
        open={open}
        onToggle={toggle}
        waiting={waiting}
      />
      {open && (
        <div className="mx-step-body">
          {event.input === null && result === '' ? <p className="mx-step-absent">The conversation history kept no detail for this step.</p> : (
            <dl className="mx-step-io">
              {event.input !== null && <><dt>input</dt><dd>{event.input}</dd></>}
              <dt>result</dt>
              <dd className={result === '' ? 'is-absent' : undefined}>{result === '' ? 'It returned nothing, or the history did not keep it.' : event.detail}</dd>
            </dl>
          )}
          <RedactionNote texts={[event.input, event.detail]} />
        </div>
      )}
    </div>
  )
}

type CommandStepProps = { event: CommandEvent; waiting?: boolean }

/**
 * A command the agent ran. Closed it is one line, the command's first line and how it ended, as the
 * agent's own terminal shows it; opened it is the whole command and the last three lines of its
 * output (all of it on request). One that failed starts open: that is the one worth reading.
 */
export function CommandStep({ event, waiting = false }: CommandStepProps) {
  const [open, toggle] = useOpen(event.status === 'failed')
  const [all, setAll] = useState(false)
  const omitted = isOmitted(event.output)
  const lines = event.output === '' || omitted ? [] : event.output.replace(/\n+$/, '').split('\n')
  const shown = all ? lines : lines.slice(-COMMAND_TAIL_LINES)
  const typed = event.command.split('\n')
  const first = typed[0] ?? ''
  const running = event.status === 'running'
  return (
    <div role="group" aria-label={`Command ${first}`} className="mx-step">
      <StepRow
        status={event.status}
        verb="Run"
        target={<code className="mx-code-chip">{event.command === '' ? 'A command' : typed.length > 1 ? `${first} …` : first}</code>}
        meta={running ? <ElapsedSince since={event.timing.startedAtMs ?? event.observed.firstSeenAtMs} /> : commandMeta(event)}
        metaTone={running ? 'working' : event.status === 'failed' ? 'danger' : 'dim'}
        open={open}
        onToggle={toggle}
        waiting={waiting}
      />
      {open && (
        <div className="mx-step-body">
          <div className="mx-step-card">
            <div className="mx-step-head is-command">
              <span className="mx-step-prompt" aria-hidden="true">$</span>
              <span className="mx-step-command">{event.command}</span>
              {event.status === 'failed' && event.exitCode !== null && <Pill tone="danger">exit {event.exitCode}</Pill>}
            </div>
            {omitted && <p className="mx-step-note">The output shows a secret file, so the history does not keep it.</p>}
            {shown.length > 0 && <pre className="mx-step-output">{shown.join('\n')}</pre>}
            {shown.length > 0 && <RedactionNote texts={[event.output]} />}
            {lines.length > COMMAND_TAIL_LINES && (
              <button type="button" aria-expanded={all} className="mx-step-more" onClick={() => { setAll((current) => !current) }}>
                {all ? 'Show less' : `Show all ${String(lines.length)} lines`}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  )
}

const LINE_TONE: Record<DiffRow['tone'], string> = { add: ' is-add', remove: ' is-del', context: '', header: '' }

/** One line of a diff: a hunk header in blue, or the line with its sign and the file's syntax colours. */
function DiffLine({ row, path }: { row: DiffRow; path: string }) {
  if (row.tone === 'header') return <div className="mx-diff-hunk">{row.text}</div>
  return (
    <div className={`mx-diff-line${LINE_TONE[row.tone]}`}>
      <span className="mx-diff-sign" aria-hidden="true">{row.sign}</span>
      <span className="mx-diff-text">{row.text === '' ? ' ' : <HighlightedLine line={row.text} path={path} />}</span>
    </div>
  )
}

function EditMeta({ event }: { event: EditEvent }) {
  if (event.status === 'running') return 'writing…'
  if (event.status === 'failed') return FAILED_WORD
  if (event.status === 'interrupted') return STOPPED_WORD
  if (event.diff.status === 'unavailable') return 'no diff'
  if (event.diff.status === 'omitted') return 'contents hidden'
  const { added, removed } = event.diff
  return (
    <>
      {added > 0 && <span className="mx-step-add">+{added}</span>}
      {added > 0 && removed > 0 && ' '}
      {removed > 0 && <span className="mx-step-del">−{removed}</span>}
    </>
  )
}

const EDIT_TONE: Partial<Record<EditEvent['status'], MetaTone>> = { running: 'working', failed: 'danger' }

type EditStepProps = {
  event: EditEvent
  /** The session's folder, so the file is shown relative to it. */
  root: string | null
  /** Opens this file's diff in the Changes panel; absent where there is no repository. */
  onOpen: ((file: string) => void) | null
  /** Its diff shows from the start, as a step outside a folded run does. */
  startOpen?: boolean
  waiting?: boolean
}

/** A file change: the file, what happened to it and its counts, opening to the first lines of its diff. */
export function EditStep({ event, root, onOpen, startOpen = false, waiting = false }: EditStepProps) {
  const [open, toggle] = useOpen(startOpen || event.status === 'failed')
  const [all, setAll] = useState(false)
  const file = event.file === null ? null : relativePath(event.file, root)
  const name = file ?? 'a file'
  const verb = event.tool === 'Write' ? 'Write' : 'Edit'
  const openFile = onOpen !== null && event.file !== null ? () => { if (event.file !== null) onOpen(event.file) } : null
  const document = documentOf(event)
  if (document !== null && file !== null) return <DocumentCard path={file} document={document} onOpen={openFile} />

  const tooLarge = event.diff.status === 'available' && event.diff.truncated
  const rows = event.diff.status === 'available' && !tooLarge ? diffRows(event.diff.hunks) : []
  const shown = all ? rows : rows.slice(0, FOLD_LINES)
  const sidePanel = openFile !== null && <button type="button" className="mx-ghost-btn" onClick={openFile}>Open in side panel<ArrowUpRight className="size-3" aria-hidden="true" /></button>
  return (
    <div role="group" aria-label={`${verb} ${name}`} className="mx-step">
      <StepRow
        status={event.status}
        verb={verb}
        target={<>{file === null ? <span className="mx-step-plain">a file</span> : <FileBadge path={file} />}<ChangePill change={event.change} /></>}
        meta={<EditMeta event={event} />}
        metaTone={EDIT_TONE[event.status] ?? 'dim'}
        open={open}
        onToggle={toggle}
        waiting={waiting}
      />
      {open && (
        <div className="mx-step-body">
          {event.diff.status === 'unavailable' && <p className="mx-step-absent">The history didn&apos;t keep this diff.</p>}
          {event.diff.status === 'omitted' && <p className="mx-step-secret"><Lock className="size-3 shrink-0" aria-hidden="true" />Secret file: Maitty doesn&apos;t show its contents.</p>}
          {tooLarge && <div className="mx-step-toolarge">Diff too large to show here{sidePanel}</div>}
          {rows.length > 0 && (
            <div className="mx-step-card">
              <div className="mx-step-head">
                <span className="mx-step-path" title={name}>{name}</span>
                {sidePanel}
              </div>
              <div className="mx-diff">{shown.map((row) => <DiffLine key={row.key} row={row} path={file ?? ''} />)}</div>
              {rows.length > FOLD_LINES && (
                <button type="button" aria-expanded={all} className="mx-step-more" onClick={() => { setAll((current) => !current) }}>
                  {all ? 'Show less' : `Show all ${String(rows.length)} lines`}
                </button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  )
}

type StepViewProps = {
  event: WorkStep
  root: string | null
  openDiff: ((file: string) => void) | null
  /** A step on its own in the chat, not one line of an opened run: an edit shows its diff from the start. */
  alone?: boolean
  waiting?: boolean
}

/** The row for one step, whatever kind it is. */
export function StepView({ event, root, openDiff, alone = false, waiting = false }: StepViewProps) {
  if (event.kind === 'edit') return <EditStep event={event} root={root} onOpen={openDiff} startOpen={alone} waiting={waiting} />
  if (event.kind === 'command') return <CommandStep event={event} waiting={waiting} />
  return <ToolStep event={event} root={root} waiting={waiting} />
}
