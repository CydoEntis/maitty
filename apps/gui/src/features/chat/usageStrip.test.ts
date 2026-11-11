import { describe, expect, it } from 'vitest'
import { usageItems, waitWords } from './usageStrip'

const NOW = 1_000_000

describe('the usage strip under the message box', () => {
  it('shows the context first, then each limit with how long until it starts over', () => {
    const items = usageItems({ context: 8.6, limits: [{ name: '5h', used: 12.2, resetsAt: NOW + 3 * 3600 + 600 }, { name: 'week', used: 43, resetsAt: NOW + 2 * 86_400 + 4 * 3600 }] }, NOW)
    expect(items.map((item) => [item.label, item.percent, item.resetsIn])).toEqual([
      ['Context', 9, null],
      ['5-hour limit', 12, '3h 10m'],
      ['Weekly limit', 43, '2d 4h'],
    ])
  })

  it('has no meter for a figure the agent did not report, never a zero', () => {
    expect(usageItems({ context: null, limits: [] }, NOW)).toEqual([])
    expect(usageItems({ context: null, limits: [{ name: 'week', used: 5, resetsAt: 0 }] }, NOW).map((item) => [item.label, item.resetsIn])).toEqual([['Weekly limit', null]])
  })

  it('turns a meter amber from three quarters and red from nine tenths', () => {
    const tones = [10, 74, 75, 89, 90, 140].map((used) => usageItems({ context: used, limits: [] }, NOW)[0]?.tone)
    expect(tones).toEqual(['plain', 'plain', 'warn', 'warn', 'danger', 'danger'])
    expect(usageItems({ context: 140, limits: [] }, NOW)[0]?.percent).toBe(100)
  })

  it('names a limit it does not know as the daemon did', () => {
    expect(usageItems({ context: null, limits: [{ name: 'opus', used: 1, resetsAt: 0 }] }, NOW)[0]?.label).toBe('opus')
  })

  it('says a wait in its two largest parts', () => {
    expect([30, 12 * 60, 3600, 3 * 3600 + 600, 86_400, 2 * 86_400 + 4 * 3600 + 59].map(waitWords)).toEqual(['now', '12m', '1h', '3h 10m', '1d', '2d 4h'])
  })
})
