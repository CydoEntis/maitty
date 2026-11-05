import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { UserInputQuestion } from '../chatEvents'
import { approvalEvent } from '../eventFixtures'
import { ApprovalCard } from './ApprovalCard'
import { UserInputForm } from './UserInputForm'

afterEach(cleanup)

const pick: UserInputQuestion = {
  id: 'q1', header: 'Branch', question: 'Which branch?', isOther: false, isSecret: false,
  options: [{ label: 'main', description: 'the default' }, { label: 'dev', description: '' }],
}
const free: UserInputQuestion = { id: 'q2', header: '', question: 'What is the ticket?', isOther: false, isSecret: false, options: [] }
const secret: UserInputQuestion = { id: 'q3', header: '', question: 'API key?', isOther: false, isSecret: true, options: [] }
const other: UserInputQuestion = { ...pick, id: 'q4', isOther: true }

describe('the questions of a user-input request', () => {
  it('sends one answer per question, an offered option as it is and free text as written', async () => {
    const onSubmit = vi.fn()
    render(<UserInputForm questions={[pick, free]} busy={false} onSubmit={onSubmit} />)
    const user = userEvent.setup()
    const send = screen.getByRole('button', { name: 'Send answer' })
    expect(send).toBeDisabled()

    await user.click(screen.getByRole('radio', { name: /^main/ }))
    expect(send).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: 'What is the ticket?' }), ' MAITTY-9 ')
    await user.click(send)

    expect(onSubmit).toHaveBeenCalledWith({ q1: ['main'], q2: ['MAITTY-9'] })
  })

  it('sends the person\'s own words as a note when the question allows it, replacing a picked option', async () => {
    const onSubmit = vi.fn()
    render(<UserInputForm questions={[other]} busy={false} onSubmit={onSubmit} />)
    const user = userEvent.setup()
    await user.click(screen.getByRole('radio', { name: /^main/ }))
    await user.type(screen.getByRole('textbox', { name: /your own answer/ }), 'feature/x')
    await user.click(screen.getByRole('button', { name: 'Send answer' }))

    expect(onSubmit).toHaveBeenCalledWith({ q4: ['user_note: feature/x'] })
  })

  it('types a secret hidden', () => {
    render(<UserInputForm questions={[secret]} busy={false} onSubmit={vi.fn()} />)
    expect(screen.getByLabelText('API key?')).toHaveAttribute('type', 'password')
  })

  it('takes nothing while an answer is on its way', () => {
    render(<UserInputForm questions={[free]} busy onSubmit={vi.fn()} />)
    expect(screen.getByRole('textbox', { name: 'What is the ticket?' })).toBeDisabled()
    expect(screen.getByRole('button', { name: 'Send answer' })).toBeDisabled()
  })
})

describe('the approval card for a user-input request', () => {
  const event = approvalEvent(5, { approvalKind: 'UserInput', command: null, choices: [], questions: [free] })

  it('shows the form, and answers by the request id', async () => {
    const answerInput = vi.fn()
    render(<ApprovalCard event={event} actions={{ answer: vi.fn(), answerInput, busy: false }} step={null} onGoToStep={null} />)
    const user = userEvent.setup()
    await user.type(screen.getByRole('textbox', { name: 'What is the ticket?' }), 'MAITTY-9')
    await user.click(screen.getByRole('button', { name: 'Send answer' }))
    expect(answerInput).toHaveBeenCalledWith(5, { q2: ['MAITTY-9'] })
  })

  it('points to the terminal only where there is nothing to answer with', () => {
    render(<ApprovalCard event={event} actions={null} step={null} onGoToStep={null} />)
    expect(screen.getByText(/Answer it in the terminal/)).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Send answer' })).not.toBeInTheDocument()
  })
})
