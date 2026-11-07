import { act, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useCrew } from '../useCrew'
import { CrewChat } from './CrewChat'
import { CrewModeSwitch } from './CrewModeSwitch'
import { CrewmatesPanel } from './CrewmatesPanel'

// The Crewmates sidebar and the chat it opens, driven the way the window drives them.

function Window() {
  const crew = useCrew()
  return (
    <>
      <CrewModeSwitch crew={crew} />
      {crew.state.mode === 'crew' && <><CrewmatesPanel crew={crew} /><CrewChat crew={crew} /></>}
    </>
  )
}

async function openCrewmates(user: ReturnType<typeof userEvent.setup>) {
  render(<Window />)
  await user.click(screen.getByRole('radio', { name: /Crewmates/ }))
}

describe('the Crewmates sidebar', () => {
  it('shows the amber count of crewmates that need you on the switch until you open the mode', async () => {
    const user = userEvent.setup()
    render(<Window />)
    expect(within(screen.getByRole('radio', { name: /Crewmates/ })).getByLabelText('1 need you')).toBeTruthy()
    await user.click(screen.getByRole('radio', { name: /Crewmates/ }))
    expect(screen.queryByLabelText('1 need you')).toBeNull()
  })

  it('lists the Captain first with its tag, then the crewmates, then the crews', async () => {
    await openCrewmates(userEvent.setup())
    const captain = screen.getByRole('button', { name: /^Captain,/ })
    expect(within(captain).getByText('CAPTAIN')).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Reviewer,/ })).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Launch crew, 4 aboard · 1 person/ })).toBeTruthy()
  })

  it('opens a crewmate’s chat from its row and marks the row current', async () => {
    const user = userEvent.setup()
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: /^Researcher,/ }))
    expect(screen.getByRole('region', { name: 'Researcher' })).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Researcher,/ }).getAttribute('aria-current')).toBe('true')
    expect(screen.getByText(/credits unused time on the old plan/)).toBeTruthy()
  })

  it('adds a crewmate from the inline form and opens its chat', async () => {
    const user = userEvent.setup()
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: 'Add a crewmate' }))
    await user.type(screen.getByLabelText('Crewmate name'), 'Scribe')
    await user.type(screen.getByLabelText('What the crewmate does'), 'Writes the docs{Enter}')
    expect(screen.getByRole('region', { name: 'Scribe' })).toBeTruthy()
    expect(screen.getByRole('button', { name: /^Scribe, Writes the docs/ })).toBeTruthy()
  })

  it('cancels the form with Escape and adds nothing for a blank name', async () => {
    const user = userEvent.setup()
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: 'Add a crewmate' }))
    await user.type(screen.getByLabelText('Crewmate name'), '{Enter}')
    expect(screen.getByRole('group', { name: 'New crewmate' })).toBeTruthy()
    await user.keyboard('{Escape}')
    expect(screen.queryByRole('group', { name: 'New crewmate' })).toBeNull()
  })

  it('starts a crew with the Captain aboard', async () => {
    const user = userEvent.setup()
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: 'Start a crew' }))
    await user.type(screen.getByLabelText('Crew name'), 'Ops crew{Enter}')
    expect(screen.getByRole('region', { name: 'Ops crew' })).toBeTruthy()
    expect(screen.getByText('1 aboard · 0 people', { selector: '.mx-crew-head-sub' })).toBeTruthy()
  })
})

describe('a chat', () => {
  beforeEach(() => { vi.useFakeTimers({ shouldAdvanceTime: true }) })
  afterEach(() => { vi.useRealTimers() })

  it('sends with Enter, shows the replier typing, then shows its answer', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: /^Tester,/ }))
    await user.type(screen.getByLabelText('Message'), 'Yes, go ahead{Enter}')
    expect(screen.getByText('Yes, go ahead')).toBeTruthy()
    expect(screen.getByRole('status').textContent).toContain('Tester is typing')
    await act(async () => { await vi.advanceTimersByTimeAsync(1400) })
    expect(screen.queryByText(/is typing/)).toBeNull()
    expect(screen.getByText('Running it now. Results in a minute.')).toBeTruthy()
  })

  it('keeps Shift+Enter for a new line and does not send an empty message', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    await openCrewmates(user)
    const box = screen.getByLabelText('Message')
    await user.type(box, '{Enter}')
    expect(screen.queryByRole('status')).toBeNull()
    await user.type(box, 'one{Shift>}{Enter}{/Shift}two')
    expect((box as HTMLTextAreaElement).value).toBe('one\ntwo')
  })

  it('lets you @name one crewmate in a crew, and adds a crewmate from the menu', async () => {
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime })
    await openCrewmates(user)
    await user.click(screen.getByRole('button', { name: /^Docs crew,/ }))
    await user.type(screen.getByLabelText('Message'), '@tester can you run it{Enter}')
    expect(screen.getByRole('status').textContent).toContain('is typing')
    await user.click(screen.getByRole('button', { name: /Add crewmate/ }))
    await user.click(within(screen.getByRole('menu', { name: 'Your crewmates' })).getByRole('menuitem', { name: /Tester/ }))
    expect(screen.getByText('Tester joined')).toBeTruthy()
  })
})
