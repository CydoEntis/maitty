import { describe, expect, it } from 'vitest'
import { ageOf, launchFlags, splitRecent } from './newChatHost'

const project = (name: string) => ({ folder: `/dev/${name}`, name, color: 'red' })

describe('launchFlags', () => {
  it('puts a picked Claude model and effort on the launch line', () => {
    expect(launchFlags('claude', 'sonnet', 'high')).toBe(' --model sonnet --effort high')
    expect(launchFlags('claude', 'default', 'max')).toBe(' --effort max')
  })

  it('puts a picked Codex model and effort in its own flags', () => {
    expect(launchFlags('codex', 'gpt-5-codex', 'medium')).toBe(' -m gpt-5-codex -c model_reasoning_effort=medium')
  })

  it('sends nothing when nothing was picked', () => {
    expect(launchFlags('claude', 'default', 'default')).toBe('')
    expect(launchFlags('codex', 'default', 'default')).toBe('')
  })

  it('leaves out an effort Codex has no level for', () => {
    expect(launchFlags('codex', 'default', 'max')).toBe('')
  })

  it('drops a model name that is not plain model-id characters', () => {
    expect(launchFlags('claude', 'x; rm -rf /', 'default')).toBe('')
    expect(launchFlags('codex', '-evil', 'default')).toBe('')
  })
})

describe('ageOf', () => {
  const now = 1_700_000_000
  it('says how long ago a time was in its largest short unit', () => {
    expect(ageOf(now - 30, now)).toBe('1m')
    expect(ageOf(now - 4 * 60, now)).toBe('4m')
    expect(ageOf(now - 3 * 3600, now)).toBe('3h')
    expect(ageOf(now - 86_400, now)).toBe('1d')
    expect(ageOf(now - 21 * 86_400, now)).toBe('3w')
  })

  it('is empty when the time is unknown', () => {
    expect(ageOf(null, now)).toBe('')
  })
})

describe('splitRecent', () => {
  it('takes the first three as recent and the rest as the others', () => {
    const { recent, rest } = splitRecent(['a', 'b', 'c', 'd', 'e'].map(project))
    expect(recent.map((item) => item.name)).toEqual(['a', 'b', 'c'])
    expect(rest.map((item) => item.name)).toEqual(['d', 'e'])
  })

  it('has no others when there are three or fewer', () => {
    expect(splitRecent(['a', 'b'].map(project)).rest).toEqual([])
  })
})
