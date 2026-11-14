import type { ChatEvent } from './chatEvents'

/** The tool Claude asks to leave plan mode with; its input holds the plan it wants approved. */
export const PLAN_TOOL = 'ExitPlanMode'

const CUT_MARK = '\n[cut]'
const PLAN_START = '"plan": "'

export type AskedPlan = {
  readonly text: string
  /** The history kept only the start of a long plan. */
  readonly cut: boolean
}

/** A JSON string's body read as far as it goes: the history cuts a long input mid-string. */
function readCutString(body: string): string | null {
  // Drop a half escape at the cut, then any quote and brace after the string's end.
  const end = body.search(/(?<!\\)"/)
  const inside = (end === -1 ? body : body.slice(0, end)).replace(/\\u[0-9a-fA-F]{0,3}$|\\$/, '')
  try {
    return JSON.parse(`"${inside}"`) as string
  } catch {
    return null
  }
}

/** The plan in an ExitPlanMode input as the history kept it, or null where it kept none. */
export function planOfInput(input: string | null): AskedPlan | null {
  if (input === null) return null
  const cut = input.endsWith(CUT_MARK)
  if (!cut) {
    try {
      const parsed: unknown = JSON.parse(input)
      const plan = typeof parsed === 'object' && parsed !== null && 'plan' in parsed ? parsed.plan : null
      return typeof plan === 'string' && plan.trim() !== '' ? { text: plan, cut: false } : null
    } catch {
      return null
    }
  }
  const start = input.indexOf(PLAN_START)
  if (start === -1) return null
  const text = readCutString(input.slice(start + PLAN_START.length, -CUT_MARK.length))
  return text === null || text.trim() === '' ? null : { text, cut: true }
}

/** The plan of the latest ExitPlanMode call in the history: the one a plan-approval prompt is about. */
export function askedPlan(events: readonly ChatEvent[]): AskedPlan | null {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index]
    if (event?.kind === 'tool' && event.name === PLAN_TOOL) return planOfInput(event.input)
  }
  return null
}
