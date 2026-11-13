import { afterEach, describe, expect, it, vi } from 'vitest'
import { snoozeChoices, tomorrowNine } from './snooze'

const SECOND = 1000
const HOUR = 3600 * SECOND

describe('tomorrow at 9:00', () => {
  it('is the next day at 9:00 whether it is before or after 9:00 now', () => {
    for (const now of [new Date(2026, 0, 5, 7, 30), new Date(2026, 0, 5, 9, 0), new Date(2026, 0, 5, 15, 45), new Date(2026, 0, 5, 23, 59)]) {
      expect(tomorrowNine(now.getTime())).toBe(new Date(2026, 0, 6, 9, 0).getTime())
    }
  })

  it('rolls over the end of a month and a year', () => {
    expect(tomorrowNine(new Date(2026, 0, 31, 22, 0).getTime())).toBe(new Date(2026, 1, 1, 9, 0).getTime())
    expect(tomorrowNine(new Date(2026, 11, 31, 22, 0).getTime())).toBe(new Date(2027, 0, 1, 9, 0).getTime())
  })

  describe('across a clock change', () => {
    afterEach(() => { vi.unstubAllEnvs() })

    it('stays at 9:00 on the wall clock when the day is 23 or 25 hours long', () => {
      vi.stubEnv('TZ', 'America/New_York')
      const springEve = new Date(2026, 2, 7, 20, 0).getTime()
      const springNine = tomorrowNine(springEve)
      expect(new Date(springNine).getHours()).toBe(9)
      expect(new Date(springNine).getDate()).toBe(8)
      expect(springNine - springEve).toBe(12 * HOUR)
      const fallEve = new Date(2026, 9, 31, 20, 0).getTime()
      const fallNine = tomorrowNine(fallEve)
      expect(new Date(fallNine).getHours()).toBe(9)
      expect(new Date(fallNine).getDate()).toBe(1)
      expect(fallNine - fallEve).toBe(14 * HOUR)
    })
  })
})

describe('the snooze choices', () => {
  const NOW = new Date(2026, 0, 5, 9, 30).getTime()

  it('offer an hour and tomorrow at 9:00, as unix seconds', () => {
    const choices = snoozeChoices(NOW, null)
    expect(choices.map((choice) => [choice.label, choice.until])).toEqual([
      ['Snooze 1 hour', (NOW + HOUR) / SECOND],
      ['Snooze until tomorrow 9:00', new Date(2026, 0, 6, 9, 0).getTime() / SECOND],
    ])
  })

  it('add until the limit resets only when the daemon says when it does', () => {
    const resumeAt = NOW + 4 * HOUR
    const choices = snoozeChoices(NOW, resumeAt)
    expect(choices.map((choice) => choice.label)).toEqual(['Snooze 1 hour', 'Snooze until tomorrow 9:00', 'Snooze until the limit resets'])
    expect(choices[2]?.until).toBe(resumeAt / SECOND)
  })

  it('leave out a reset already past, or further off than the daemon lets a snooze last', () => {
    expect(snoozeChoices(NOW, NOW - SECOND)).toHaveLength(2)
    expect(snoozeChoices(NOW, NOW + 8 * 24 * HOUR)).toHaveLength(2)
  })
})
