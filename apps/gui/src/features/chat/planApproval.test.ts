import { describe, expect, it } from 'vitest'
import { inputText } from './claudeTools'
import { toolEvent } from './eventFixtures'
import { askedPlan, planOfInput } from './planApproval'

describe('the plan an approval is about', () => {
  it('reads the plan from the ExitPlanMode input', () => {
    expect(planOfInput(inputText({ plan: '## Refunds\n1. Add the endpoint' }))).toEqual({ text: '## Refunds\n1. Add the endpoint', cut: false })
  })

  it('reads as much of a long plan as the history kept, and says it was cut', () => {
    const plan = `# Plan\n${'Step "one" \\ done.\n'.repeat(400)}`
    const read = planOfInput(inputText({ plan }))
    expect(read?.cut).toBe(true)
    expect(plan.startsWith(read?.text ?? 'missing')).toBe(true)
  })

  it('is nothing where the history kept no plan', () => {
    expect([planOfInput(null), planOfInput('{}'), planOfInput('{"plan": "  "}'), planOfInput('not json')]).toEqual([null, null, null, null])
  })

  it('takes the latest ExitPlanMode call, not an earlier one', () => {
    const events = [
      toolEvent('1', { name: 'ExitPlanMode', input: inputText({ plan: 'first' }) }),
      toolEvent('2', { name: 'Read', input: inputText({ file_path: 'a.ts' }) }),
      toolEvent('3', { name: 'ExitPlanMode', input: inputText({ plan: 'second' }) }),
    ]
    expect(askedPlan(events)?.text).toBe('second')
    expect(askedPlan([toolEvent('1')])).toBeNull()
  })
})
