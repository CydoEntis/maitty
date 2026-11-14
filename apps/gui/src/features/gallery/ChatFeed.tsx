import '@/features/chat/chat.css'
import type { DiffLine } from '@/features/chat/chatEvents'
import { AgentsCard } from '@/features/chat/components/AgentsCard'
import { Markdown } from '@/features/chat/components/Markdown'
import { PlanCard } from '@/features/chat/components/PlanCard'
import { ApprovalTray, PromptTray } from '@/features/chat/components/QuestionTray'
import { CommandStep, EditStep } from '@/features/chat/components/StepCards'
import { ThoughtRow } from '@/features/chat/components/ThoughtRow'
import { WorkCard } from '@/features/chat/components/WorkCard'
import { approvalEvent, commandEvent, editEvent, subagentEvent, todoEvent, toolEvent } from '@/features/chat/eventFixtures'

/*
 * The chat's cards on made-up steps, so the feed can be checked against the design without an agent running. The
 * events come from the same builders the tests use.
 */

const TEST_OUTPUT = ['> refunds@1.4.0 test', '> vitest run', '', ' ✓ src/refunds/partial.test.ts (6 tests)', ' ✓ src/refunds/limits.test.ts (4 tests)', ' ✓ src/api/refunds.test.ts (9 tests)', '', ' Test Files  3 passed (3)', '      Tests  19 passed (19)'].join('\n')
const FAILED_OUTPUT = ['src/db/migrate.ts:41', '  error: relation "refund_lines" already exists', '  at Migrator.run (node_modules/kysely/migrator.js:212)', 'Migration 0042_partial_refunds failed'].join('\n')

const lines = (kind: DiffLine['kind'], ...texts: string[]): DiffLine[] => texts.map((text) => ({ kind, text }))
const DIFF: DiffLine[] = [
  ...lines('context', 'export function refundable(order: Order): number {'),
  ...lines('remove', '  return order.total'),
  ...lines('add', '  const refunded = order.refunds.reduce((sum, r) => sum + r.amount, 0)', '  return Math.max(0, order.total - refunded)'),
  ...lines('context', '}'),
]

const REPLY = [
  'The partial refunds endpoint is in and its tests pass.',
  '',
  '### What changed',
  '- `refundable()` now subtracts earlier refunds, so a second refund can never exceed what is left.',
  '- `POST /orders/:id/refunds` takes an `amount` and checks it against the limit.',
  '',
  '| Order | Total | Refunded | Left |',
  '| --- | ---: | ---: | ---: |',
  '| 1042 | 120.00 | 40.00 | 80.00 |',
  '| 1043 | 64.50 | 0.00 | 64.50 |',
  '',
  '```ts',
  'const left = refundable(order)',
  'if (amount > left) throw new RefundTooLarge(amount, left)',
  '```',
].join('\n')

const TOOK_MS = 4_200
/** When the gallery opened, so a running helper's clock starts near zero. */
const OPENED_MS = Date.now()
const started = (ms: number) => ({ startedAtMs: ms, completedAtMs: ms + TOOK_MS })

const IDLE = () => Promise.resolve()
const ACTIONS = { answer: () => undefined, answerInput: () => undefined, remember: () => undefined, busy: false }
const PLAN = { text: '#### Partial refunds\n1. Subtract earlier refunds in `refundable()`.\n2. Add `POST /orders/:id/refunds`.\n3. Test the limits.', cut: false }

/** The waiting cards over a stand-in for the message box, one of each kind. */
export function WaitingCards() {
  const box = <div className="border border-(--line2) bg-(--card) px-3.5 pt-3 pb-8 text-sm text-(--dim)">Message claude</div>
  return (
    <div className="mx-chat" style={{ zoom: 1 }}>
      <div className="mx-chat-column">
        <div><ApprovalTray event={approvalEvent('r1', { command: 'pnpm db:migrate --env staging', canRemember: true })} actions={ACTIONS} agent="Claude" />{box}</div>
        <div><ApprovalTray event={approvalEvent('r2', { approvalKind: 'FileChange', command: null, changes: ['src/refunds/partial.ts', 'src/api/refunds.ts', 'docs/refunds.md'].map((path) => ({ path, kind: 'update', diff: { status: 'unavailable' as const } })) })} actions={ACTIONS} agent="Codex" />{box}</div>
        <div><PromptTray agent="claude" online ask={{ tool: 'ExitPlanMode', detail: '', answerable: true }} answer={IDLE} plan={PLAN} />{box}</div>
        <div><PromptTray agent="claude" online ask={{ tool: 'AskUserQuestion', detail: 'Which database should refunds use?', answerable: false, choices: ['Postgres', 'SQLite', 'Keep the current one'] }} answer={IDLE} pick={IDLE} />{box}</div>
        <div><PromptTray agent="claude" online ask={{ tool: 'AskUserQuestion', detail: '', answerable: false, choices: [], questions: [{ question: 'Which database?', choices: ['Postgres', 'SQLite'] }, { question: 'Which language?', choices: ['Rust', 'Go'] }] }} answer={IDLE} pickAll={IDLE} />{box}</div>
      </div>
    </div>
  )
}

export function ChatFeed() {
  const run = [
    toolEvent('r1', { target: 'src/refunds/partial.ts' }),
    toolEvent('r2', { name: 'Grep', target: 'refundable' }),
    editEvent('r3', { file: 'src/refunds/partial.ts', diff: { status: 'available', hunks: [{ header: '@@ -12,2 +12,3 @@', lines: DIFF }], added: 2, removed: 1, truncated: false } }),
    commandEvent('r4', { command: 'pnpm test', output: TEST_OUTPUT, exitCode: 0, durationMs: TOOK_MS }),
    toolEvent('r5', { name: 'Read', target: 'src/api/refunds.ts' }),
  ]
  return (
    <div className="mx-chat" style={{ zoom: 1 }}>
      <div className="mx-chat-column">
        <WorkCard steps={run} root={null} openDiff={null} waitingStep={null} />
        <CommandStep event={commandEvent('c1', { command: 'pnpm test --filter refunds', output: TEST_OUTPUT, exitCode: 0, timing: started(0) })} alone />
        <CommandStep event={commandEvent('c2', { command: 'pnpm db:migrate --env staging', output: FAILED_OUTPUT, exitCode: 1, status: 'failed', timing: started(0) })} alone />
        <EditStep event={editEvent('e1', { file: 'src/refunds/partial.ts', diff: { status: 'available', hunks: [{ header: '@@ -12,2 +12,3 @@', lines: DIFF }], added: 2, removed: 1, truncated: false } })} root={null} onOpen={() => undefined} alone />
        <EditStep event={editEvent('e2', { file: '.env.local', diff: { status: 'omitted' } })} root={null} onOpen={null} alone />
        <EditStep event={editEvent('e3', { tool: 'Write', file: 'docs/refunds.md', change: 'add', diff: { status: 'available', hunks: [{ header: '', lines: lines('add', '# Partial refunds', '', 'A refund can be any amount up to what is left on the order.', '## Limits', '- Never more than the order total.') }], added: 5, removed: 0, truncated: false } })} root={null} onOpen={null} />
        <PlanCard event={todoEvent('p1', [{ text: 'Read the refunds module', state: 'done' }, { text: 'Subtract earlier refunds', state: 'done' }, { text: 'Add the endpoint', state: 'current' }, { text: 'Write the tests', state: 'pending' }])} />
        <AgentsCard process="claude" agents={[subagentEvent('a1', { title: 'Find call sites', status: 'running', prompt: 'Find every caller of refundable()', timing: { startedAtMs: OPENED_MS, completedAtMs: null } }), subagentEvent('a2', { title: 'Check the limits', report: 'The limit is checked in two places.' }), subagentEvent('a3', { title: 'Review the migration', agentType: 'Plan', report: 'It is safe to rerun.' })]} />
        <ThoughtRow texts={['The second refund must not exceed what is left, so subtract the earlier ones first.']} />
        <div className="mx-msg-agent"><Markdown text={REPLY} /></div>
      </div>
    </div>
  )
}
