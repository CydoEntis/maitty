import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PaneViewSwitch } from './PaneViewSwitch'

describe('PaneViewSwitch', () => {
  it('offers Chat, Terminal and Work in that order, Work saying how many runs go and that one needs you', async () => {
    const user = userEvent.setup()
    const onChange = vi.fn()
    render(<PaneViewSwitch views={['work', 'terminal', 'chat']} view="chat" work={{ going: 3, needsYou: true }} onChange={onChange} />)
    expect(screen.getAllByRole('radio').map((radio) => radio.textContent)).toEqual(['Chat', 'Terminal', 'Work3'])
    await user.click(screen.getByRole('radio', { name: 'Work view, 3 going, one needs you' }))
    expect(onChange).toHaveBeenCalledWith('work')
  })

  it('leaves Work out for a session with no workflow runs', () => {
    render(<PaneViewSwitch views={['chat', 'terminal']} view="terminal" work={null} onChange={vi.fn()} />)
    expect(screen.queryByRole('radio', { name: /^Work/ })).toBeNull()
    expect(screen.getByRole('radio', { name: /^Terminal view/ })).toBeChecked()
  })
})
