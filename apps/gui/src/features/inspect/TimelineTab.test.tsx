import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TimelineTab } from './TimelineTab'
import { turnLengths } from './turnLengths'

const events = [
  { at: 100, kind: 'Working' as const, detail: 'hook: UserPromptSubmit' },
  { at: 165, kind: 'Done' as const, detail: 'hook: Stop' },
  { at: 170, kind: 'CommandFinished' as const, detail: '2' },
]

describe('Timeline', () => {
  it('lists the newest event first and says how long a turn ran', () => {
    render(<TimelineTab events={events} />)
    const rows = screen.getAllByRole('listitem').map((row) => row.textContent)
    expect(rows[0]).toContain('Command finished (exit 2)')
    expect(rows[2]).toContain('Started working')
    expect(rows[2]).toContain('ran 1m 5s')
  })

  it('explains an empty timeline instead of showing nothing', () => {
    render(<TimelineTab events={[]} />)
    expect(screen.getByText(/Nothing yet/)).toBeTruthy()
  })

  it('measures a turn from working to the next event that is not working', () => {
    expect([...turnLengths(events)]).toEqual([[0, 65]])
  })
})
