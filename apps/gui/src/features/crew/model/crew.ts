/**
 * Crewmates and crews (design v31, README section 4.5 and 6.3). A Captain is your lead agent, a
 * crewmate is a named agent with its own chat, and a crew is a group chat the Captain, crewmates
 * and other people join. Pure: the hook keeps the state, this decides what a change means.
 *
 * shortcut: nothing here talks to the daemon. The design says agents chat only on this computer
 * and that crews with other people need a sync service nobody has designed, so the window runs on
 * sample data and keeps it in memory.
 */

export type CrewAgent = 'claude' | 'codex'
export type CrewStatus = 'idle' | 'working' | 'needs' | 'done'

/** Anyone who can speak in a chat: an agent (a crewmate or someone else's) or a person. */
export type Participant = {
  id: string
  name: string
  agent?: CrewAgent
  /** A person, drawn as a round letter avatar in this colour. */
  color?: string
  status?: CrewStatus
}

export type Mate = Participant & { agent: CrewAgent; role: string; status: CrewStatus; unread: number }

export type Crew = { id: string; name: string; members: readonly string[]; unread: number }

export type ChatLine =
  | { kind: 'system'; text: string }
  | { kind: 'message'; from: string; text: string; time: string }

export type Typing = { chat: string; who: string }

export type CrewState = {
  mode: 'fleets' | 'crew'
  selected: string
  mates: readonly Mate[]
  others: Readonly<Record<string, Participant>>
  crews: readonly Crew[]
  lines: Readonly<Record<string, readonly ChatLine[]>>
  typing: Typing | null
}

export const CAPTAIN_ID = 'cap'
const YOU = 'you'

const REPLIES: Readonly<Record<string, string>> = {
  cap: 'Got it. I’ll take that, and pull in Reviewer if it touches code. I’ll report back here.',
  m1: 'Looking into it now. I’ll post a short summary here.',
  m2: 'On it. I’ll leave comments inline and post the verdict here.',
  m3: 'Running it now. Results in a minute.',
  samd: 'I can mock that up. Give me a few minutes.',
}
const GENERIC_REPLY = 'On it. I’ll report back here.'

const MINUTE_DIGITS = 2

export function clockTime(at: Date): string {
  return `${String(at.getHours())}:${String(at.getMinutes()).padStart(MINUTE_DIGITS, '0')}`
}

const message = (from: string, text: string, time: string): ChatLine => ({ kind: 'message', from, text, time })

export function initialCrew(): CrewState {
  return {
    mode: 'fleets',
    selected: CAPTAIN_ID,
    mates: [
      { id: CAPTAIN_ID, name: 'Captain', agent: 'claude', role: 'Your lead · plans and hands out work', status: 'idle', unread: 0 },
      { id: 'm1', name: 'Researcher', agent: 'claude', role: 'Reads docs and code, reports back', status: 'done', unread: 1 },
      { id: 'm2', name: 'Reviewer', agent: 'codex', role: 'Reviews diffs before they merge', status: 'working', unread: 0 },
      { id: 'm3', name: 'Tester', agent: 'claude', role: 'Writes and runs tests', status: 'needs', unread: 1 },
    ],
    others: {
      sam: { id: 'sam', name: 'Sam', color: 'var(--fleet-violet)' },
      samd: { id: 'samd', name: 'Sam’s Designer', agent: 'claude', status: 'idle' },
    },
    crews: [
      { id: 'c1', name: 'Launch crew', members: [CAPTAIN_ID, 'm2', 'sam', 'samd'], unread: 3 },
      { id: 'c2', name: 'Docs crew', members: [CAPTAIN_ID, 'm1'], unread: 0 },
    ],
    lines: {
      cap: [message('cap', 'Morning. Reviewer is reading the pricing diff and Tester is waiting on you to approve a test run. Want me to take anything else?', '9:41')],
      m1: [
        message(YOU, 'How does Stripe handle proration on plan changes?', '9:12'),
        message('m1', 'It credits unused time on the old plan and charges the new plan from today. I wrote a short summary with the three settings that matter. Want it here or in the docs?', '9:20'),
      ],
      m2: [
        message(YOU, 'Review the pricing copy PR', '9:30'),
        message('m2', 'Reading 6 files. Two copy changes look risky; I’ll flag them inline and post a verdict here.', '9:31'),
      ],
      m3: [message('m3', 'I need to run npm run test:e2e against staging. Okay to go?', '9:38')],
      c1: [
        { kind: 'system', text: 'Sam joined with Sam’s Designer' },
        message('sam', 'Morning all. Designer has the new pricing cards ready.', '9:02'),
        message('samd', 'Three variants are up. I’d go with B: clearer annual savings.', '9:04'),
        message(CAPTAIN_ID, 'Agreed on B. Reviewer, can you check the copy matches what pricing.ts actually charges?', '9:06'),
        message('m2', 'Checking now. One mismatch: the card says 20% off yearly, code gives 16.7%.', '9:09'),
      ],
      c2: [
        { kind: 'system', text: 'You started Docs crew' },
        message('m1', 'Docs nav is outlined. Waiting on Captain for the order of sections.', '8:50'),
      ],
    },
    typing: null,
  }
}

export function whoIs(state: CrewState, id: string): Participant {
  return state.mates.find((mate) => mate.id === id) ?? state.others[id] ?? { id, name: id, agent: 'claude' }
}

export const isCrew = (state: CrewState, id: string): boolean => state.crews.some((crew) => crew.id === id)

export const mateAgentWord = (agent: CrewAgent): string => (agent === 'codex' ? 'Codex' : 'Claude')

/** How many crewmates are waiting on you: the amber count on the Crewmates switch. */
export const needsCount = (state: CrewState): number => state.mates.filter((mate) => mate.status === 'needs').length

/** The line under a chat's name: a crewmate's job and agent, or who is aboard a crew. */
export function chatSubtitle(state: CrewState, id: string): string {
  const crew = state.crews.find((candidate) => candidate.id === id)
  if (!crew) {
    const mate = state.mates.find((candidate) => candidate.id === id)
    return mate ? `${mate.role} · ${mateAgentWord(mate.agent)}` : ''
  }
  const people = crew.members.filter((member) => whoIs(state, member).agent === undefined).length
  return `${String(crew.members.length)} aboard · ${String(people)} ${people === 1 ? 'person' : 'people'}`
}

export function select(state: CrewState, id: string): CrewState {
  return {
    ...state,
    selected: id,
    mates: state.mates.map((mate) => (mate.id === id ? { ...mate, unread: 0 } : mate)),
    crews: state.crews.map((crew) => (crew.id === id ? { ...crew, unread: 0 } : crew)),
  }
}

/** Who answers a message: in a crew the agent you @name, else each agent in turn; in a chat, its crewmate. */
export function replier(state: CrewState, chat: string, text: string): string {
  const crew = state.crews.find((candidate) => candidate.id === chat)
  if (!crew) return chat
  const agents = crew.members.filter((member) => whoIs(state, member).agent !== undefined)
  const named = /@(\w+)/.exec(text)?.[1]?.toLowerCase()
  const hit = named === undefined ? undefined : agents.find((member) => whoIs(state, member).name.toLowerCase().startsWith(named))
  return hit ?? agents[(state.lines[chat]?.length ?? 0) % Math.max(agents.length, 1)] ?? CAPTAIN_ID
}

export function replyText(who: string): string {
  return REPLIES[who] ?? GENERIC_REPLY
}

const appended = (state: CrewState, chat: string, line: ChatLine): CrewState['lines'] => ({ ...state.lines, [chat]: [...(state.lines[chat] ?? []), line] })

/** Your message goes in and the replier starts typing. Blank text changes nothing. */
export function send(state: CrewState, chat: string, text: string, at: Date): CrewState {
  const body = text.trim()
  if (body === '') return state
  const lines = appended(state, chat, message(YOU, body, clockTime(at)))
  return { ...state, lines, typing: { chat, who: replier(state, chat, body) } }
}

/** The replier's answer arrives and the typing mark goes. */
export function receive(state: CrewState, at: Date): CrewState {
  if (!state.typing) return state
  const { chat, who } = state.typing
  return { ...state, lines: appended(state, chat, message(who, replyText(who), clockTime(at))), typing: null }
}

const nextId = (prefix: string, taken: readonly string[]): string => {
  let n = taken.length + 1
  while (taken.includes(`${prefix}${String(n)}`)) n += 1
  return `${prefix}${String(n)}`
}

export const MAX_NAME_LENGTH = 40

/** A new crewmate under the Captain, selected so its chat opens. A blank name adds nothing. */
export function addMate(state: CrewState, input: { name: string; role: string; agent: CrewAgent }): CrewState {
  const name = input.name.trim().slice(0, MAX_NAME_LENGTH)
  if (name === '') return state
  const id = nextId('m', state.mates.map((mate) => mate.id))
  const role = input.role.trim() === '' ? 'Works with your Captain' : input.role.trim()
  const mate: Mate = { id, name, agent: input.agent, role, status: 'idle', unread: 0 }
  return { ...state, mates: [...state.mates, mate], selected: id, lines: { ...state.lines, [id]: [] } }
}

/** A new crew with your Captain aboard, selected so its chat opens. */
export function addCrew(state: CrewState, nameInput: string): CrewState {
  const name = nameInput.trim().slice(0, MAX_NAME_LENGTH)
  if (name === '') return state
  const id = nextId('c', state.crews.map((crew) => crew.id))
  const crew: Crew = { id, name, members: [CAPTAIN_ID], unread: 0 }
  return { ...state, crews: [...state.crews, crew], selected: id, lines: { ...state.lines, [id]: [{ kind: 'system', text: `You started ${name}` }] } }
}

/** The crewmates that are not yet in a crew: the Add crewmate menu. */
export function notAboard(state: CrewState, crewId: string): readonly Mate[] {
  const crew = state.crews.find((candidate) => candidate.id === crewId)
  if (!crew) return []
  return state.mates.filter((mate) => !crew.members.includes(mate.id))
}

export function addToCrew(state: CrewState, crewId: string, mateId: string): CrewState {
  const mate = state.mates.find((candidate) => candidate.id === mateId)
  const crew = state.crews.find((candidate) => candidate.id === crewId)
  if (!mate || !crew || crew.members.includes(mateId)) return state
  return {
    ...state,
    crews: state.crews.map((candidate) => (candidate.id === crewId ? { ...candidate, members: [...candidate.members, mateId] } : candidate)),
    lines: appended(state, crewId, { kind: 'system', text: `${mate.name} joined` }),
  }
}
