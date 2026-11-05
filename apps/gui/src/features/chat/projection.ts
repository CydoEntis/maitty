import type {
  ApprovalDecision,
  ApprovalEvent,
  ChatEvent,
  ChatProvider,
  ItemFields,
  ItemStatus,
} from './chatEvents'
import { changeKindOf, diffForChange } from './codexFileChange'
import { parseAnsweredApproval, parseApprovalRequest, parseHistoryEntry, type ApprovalRequest } from './historyInput'
import { projectItem, statusOf } from './projectItem'

/**
 * The chat projection: a reducer over the daemon's conversation history.
 *
 * Feed it history entries as they are read (`applyHistoryEntries`) and the pending approvals
 * (`applyApprovals`). Each call returns a new state and re-projects only what it was given:
 * an entry the daemon updates in place (a tool going from Running to Completed) replaces that
 * item's events at its original position, so order never changes and re-reading a page twice
 * is harmless. `selectChatEvents` flattens the state into the list the view renders.
 *
 * Pure: the clock comes in as `nowMs`, and nothing here reads the DOM, Tauri or the network.
 */

type ItemRecord = {
  readonly events: readonly ChatEvent[]
  readonly firstSeenAtMs: number
  readonly settledAtMs: number | null
}

type TurnRecord = {
  readonly turnId: string
  /** `null` until the daemon sends the turn's own status entry. */
  readonly status: ItemStatus | null
  readonly startedAtMs: number
  readonly endedAtMs: number | null
  /** The daemon sent the turn's own start and end times, so its duration is real and not what this app saw. */
  readonly timed: boolean
  readonly itemOrder: readonly string[]
  readonly items: ReadonlyMap<string, ItemRecord>
}

type WorkingTurn = {
  turnId: string
  status: ItemStatus | null
  startedAtMs: number
  endedAtMs: number | null
  timed: boolean
  itemOrder: string[]
  items: Map<string, ItemRecord>
}

export type ChatProjection = {
  readonly provider: ChatProvider
  readonly turnOrder: readonly string[]
  readonly turns: ReadonlyMap<string, TurnRecord>
  readonly approvals: readonly ApprovalEvent[]
  /** How many history entries or approvals failed validation and were left out. */
  readonly rejectedInputs: number
}

export function createChatProjection(provider: ChatProvider): ChatProjection {
  return { provider, turnOrder: [], turns: new Map(), approvals: [], rejectedInputs: 0 }
}

function approvalId(requestId: number | string): string {
  return `approval:${typeof requestId}:${String(requestId)}`
}

export function applyHistoryEntries(state: ChatProjection, entries: readonly unknown[], nowMs: number): ChatProjection {
  const turnOrder = [...state.turnOrder]
  const working = new Map<string, WorkingTurn>()
  let rejectedInputs = state.rejectedInputs

  const turnFor = (turnId: string): WorkingTurn => {
    const existing = working.get(turnId)
    if (existing) return existing
    const previous = state.turns.get(turnId)
    const copy: WorkingTurn = previous
      ? { ...previous, itemOrder: [...previous.itemOrder], items: new Map(previous.items) }
      : { turnId, status: null, startedAtMs: nowMs, endedAtMs: null, timed: false, itemOrder: [], items: new Map() }
    if (!previous) turnOrder.push(turnId)
    working.set(turnId, copy)
    return copy
  }

  for (const raw of entries) {
    const entry = parseHistoryEntry(raw)
    if (entry === null) {
      rejectedInputs += 1
      continue
    }
    const turn = turnFor(entry.source.turn)
    const status = statusOf(entry.state)

    if (entry.content === 'TurnStatus') {
      turn.status = status
      if (status !== 'running') turn.endedAtMs ??= nowMs
      // The daemon's own times replace the ones this app saw, so a turn read from history still has its duration.
      if (entry.started_at_ms !== undefined) turn.startedAtMs = entry.started_at_ms
      if (entry.completed_at_ms !== undefined) turn.endedAtMs = entry.completed_at_ms
      turn.timed = entry.started_at_ms !== undefined && entry.completed_at_ms !== undefined
      continue
    }

    const id = `${entry.source.turn}:${entry.source.item}`
    const previous = turn.items.get(id)
    const firstSeenAtMs = previous?.firstSeenAtMs ?? nowMs
    const settledAtMs = previous?.settledAtMs ?? (status === 'running' ? null : nowMs)
    const timing = { startedAtMs: entry.started_at_ms ?? null, completedAtMs: entry.completed_at_ms ?? null }
    const base: ItemFields = { id, turnId: entry.source.turn, status, observed: { firstSeenAtMs, settledAtMs }, timing }
    if (!previous) turn.itemOrder.push(id)
    turn.items.set(id, { events: projectItem(state.provider, entry.content, base), firstSeenAtMs, settledAtMs })
  }

  if (working.size === 0 && rejectedInputs === state.rejectedInputs) return state
  return { ...state, turnOrder, turns: new Map([...state.turns, ...working]), rejectedInputs }
}

function approvalEventOf(request: ApprovalRequest): ApprovalEvent {
  return {
    kind: 'approval',
    id: approvalId(request.request_id),
    requestId: request.request_id,
    turnId: request.turn_id ?? null,
    itemId: request.item_id ?? null,
    stepId: request.turn_id && request.item_id ? `${request.turn_id}:${request.item_id}` : null,
    approvalKind: request.kind,
    command: request.command ?? null,
    reason: request.reason ?? null,
    changes: request.changes.map((change) => ({
      path: change.path,
      kind: change.kind,
      diff: diffForChange(changeKindOf(change.kind), change.diff, false),
    })),
    questions: request.questions.map((question) => ({ id: question.id, header: question.header, question: question.question, isOther: question.is_other, isSecret: question.is_secret, options: question.options })),
    choices: request.choices,
    resolution: { status: 'pending' },
  }
}

/**
 * Applies the daemon's list of approvals still waiting. The daemon forgets a request once it is
 * answered, so a known request that is no longer listed is marked answered. Its choice is
 * `unknown` unless `recordApprovalAnswer` saw it. An answered request never becomes pending again.
 */
export function applyApprovals(state: ChatProjection, pending: readonly unknown[]): ChatProjection {
  const requests: ApprovalRequest[] = []
  let rejectedInputs = state.rejectedInputs
  for (const raw of pending) {
    const request = parseApprovalRequest(raw)
    if (request) requests.push(request)
    else rejectedInputs += 1
  }
  const listed = new Set(requests.map((request) => approvalId(request.request_id)))
  const known = new Set(state.approvals.map((approval) => approval.id))

  const approvals: ApprovalEvent[] = state.approvals.map((approval) =>
    approval.resolution.status === 'pending' && !listed.has(approval.id)
      ? { ...approval, resolution: { status: 'answered', answer: 'unknown' } }
      : approval,
  )
  for (const request of requests) {
    if (!known.has(approvalId(request.request_id))) approvals.push(approvalEventOf(request))
  }
  return { ...state, approvals, rejectedInputs }
}

/**
 * Applies the daemon's record of approvals a person answered, wherever they answered them. A known
 * request is marked answered with the real choice (replacing `unknown`); one never seen pending is
 * added as answered, so a chat opened after the answer still shows what happened. A choice this app
 * recorded itself is kept.
 */
export function applyAnsweredApprovals(state: ChatProjection, answered: readonly unknown[]): ChatProjection {
  if (answered.length === 0) return state
  let rejectedInputs = state.rejectedInputs
  const approvals = [...state.approvals]
  const indexOf = new Map(approvals.map((approval, index) => [approval.id, index]))
  let changed = false
  for (const raw of answered) {
    const record = parseAnsweredApproval(raw)
    if (record === null) {
      rejectedInputs += 1
      changed = true
      continue
    }
    const resolution = { status: 'answered', answer: record.decision, answeredAtMs: record.answered_at_ms ?? null } as const
    const id = approvalId(record.request.request_id)
    const at = indexOf.get(id)
    if (at === undefined) {
      indexOf.set(id, approvals.length)
      approvals.push({ ...approvalEventOf(record.request), resolution })
      changed = true
      continue
    }
    const known = approvals[at]
    if (known === undefined) continue
    // A choice this app recorded itself is the person's own click; only its time is filled in.
    const own = known.resolution.status === 'answered' && known.resolution.answer !== 'unknown' ? known.resolution : null
    if (own !== null && own.answeredAtMs !== undefined) continue
    approvals[at] = { ...known, resolution: own === null ? resolution : { ...own, answeredAtMs: resolution.answeredAtMs } }
    changed = true
  }
  return changed ? { ...state, approvals, rejectedInputs } : state
}

/** Marks a pending request answered with the user's choice, before the daemon confirms it. */
export function recordApprovalAnswer(state: ChatProjection, requestId: number | string, decision: ApprovalDecision): ChatProjection {
  const id = approvalId(requestId)
  return {
    ...state,
    approvals: state.approvals.map((approval) =>
      approval.id === id && approval.resolution.status === 'pending'
        ? { ...approval, resolution: { status: 'answered', answer: decision } }
        : approval,
    ),
  }
}

/**
 * The events in reading order: each turn's items as first seen, then that turn's approvals, then
 * its divider once the turn has ended. Approvals that name no known turn come last.
 */
export function selectChatEvents(state: ChatProjection): ChatEvent[] {
  const events: ChatEvent[] = []
  for (const turnId of state.turnOrder) {
    const turn = state.turns.get(turnId)
    if (!turn) continue
    for (const itemId of turn.itemOrder) {
      const item = turn.items.get(itemId)
      if (item) events.push(...item.events)
    }
    events.push(...state.approvals.filter((approval) => approval.turnId === turnId))
    if (turn.status !== null && turn.status !== 'running' && turn.endedAtMs !== null) {
      events.push({ kind: 'turn', id: `turn:${turnId}`, turnId, outcome: turn.status, startedAtMs: turn.startedAtMs, endedAtMs: turn.endedAtMs, timed: turn.timed })
    }
  }
  events.push(...state.approvals.filter((approval) => approval.turnId === null || !state.turns.has(approval.turnId)))
  return events
}

/** Projects a whole read in one call: the same result as applying it in any number of chunks. */
export function projectConversation(
  provider: ChatProvider,
  entries: readonly unknown[],
  approvals: readonly unknown[],
  nowMs: number,
  answered: readonly unknown[] = [],
): ChatEvent[] {
  const projected = applyApprovals(applyHistoryEntries(createChatProjection(provider), entries, nowMs), approvals)
  return selectChatEvents(applyAnsweredApprovals(projected, answered))
}
