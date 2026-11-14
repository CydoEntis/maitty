import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it } from 'vitest'
import type { DiffLine } from '../chatEvents'
import { commandEvent, editEvent, toolEvent } from '../eventFixtures'
import { documentOf } from '../writtenDocument'
import { EditStep } from './StepCards'
import { WorkCard } from './WorkCard'

afterEach(cleanup)

const card = (steps: Parameters<typeof WorkCard>[0]['steps'], waitingStep: string | null = null) => <WorkCard steps={steps} root={null} openDiff={null} waitingStep={waitingStep} />

describe('a run of steps', () => {
  it('shows its latest 3 steps under its summary until opened, then every step', async () => {
    render(card([commandEvent('1', { command: 'pnpm lint' }), toolEvent('2', { target: 'a.ts' }), toolEvent('3'), commandEvent('4', { command: 'pnpm test', exitCode: 0 })]))

    expect(screen.getByRole('group', { name: 'Command pnpm test' })).toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Command pnpm lint' })).toBeNull()
    await userEvent.click(screen.getByTitle('Show every step'))
    expect(screen.getByRole('group', { name: 'Command pnpm lint' })).toBeInTheDocument()
    expect(screen.queryByText('+1 more')).toBeNull()
  })

  it('opens by itself when a request waits on a step the fold hides', () => {
    render(card([commandEvent('1', { command: 'rm -rf build', status: 'running' }), toolEvent('2'), toolEvent('3'), toolEvent('4')], '1'))

    expect(screen.getByText('waiting for you ↓')).toBeInTheDocument()
  })

  it('opens by itself when a step failed', () => {
    render(card([commandEvent('1', { command: 'pnpm test', exitCode: 1, status: 'failed', output: 'boom' })]))

    expect(screen.getByRole('group', { name: 'Command pnpm test' })).toHaveTextContent('boom')
  })

  it('says how many steps the fold leaves out', () => {
    render(card([commandEvent('1'), toolEvent('2'), editEvent('3'), commandEvent('4'), editEvent('5')]))

    expect(screen.getByText('+2 more')).toBeInTheDocument()
  })

  it('marks the step a request is waiting on', () => {
    render(card([commandEvent('1', { command: 'rm -rf build', status: 'running' })], '1'))

    expect(screen.getByText('waiting for you ↓')).toBeInTheDocument()
  })
})

describe('a Markdown document the agent wrote', () => {
  const added = (...texts: string[]): DiffLine[] => texts.map((text) => ({ kind: 'add', text }))
  const written = (file: string, lines: DiffLine[]) => editEvent('e', { tool: 'Write', file, change: 'add', diff: { status: 'available', hunks: [{ header: '', lines }], added: lines.length, removed: 0, truncated: false } })

  it('is read as a title and its first lines', () => {
    expect(documentOf(written('docs/plan.md', added('# Refunds plan', '', 'First we add the endpoint.', '## Steps', 'Then the tests.')))).toEqual({
      title: 'Refunds plan',
      excerpt: 'First we add the endpoint.\nThen the tests.',
      lineCount: 5,
      text: '# Refunds plan\n\nFirst we add the endpoint.\n## Steps\nThen the tests.',
    })
  })

  it.each([
    ['a file that is not Markdown', written('src/a.ts', added('# not a title'))],
    ['an edit of a Markdown file', { ...written('a.md', added('# T')), tool: 'Edit' }],
    ['a write still running', { ...written('a.md', added('# T')), status: 'running' as const }],
    ['a write whose text the history did not keep', editEvent('e', { tool: 'Write', file: 'a.md', diff: { status: 'unavailable' } })],
  ])('is not made of %s', (_name, event) => {
    expect(documentOf(event)).toBeNull()
  })

  it('shows as a card with its path and line count, not a diff', () => {
    render(<EditStep event={written('docs/plan.md', added('# Refunds plan', 'First we add the endpoint.'))} root={null} onOpen={null} />)

    const group = screen.getByRole('group', { name: 'Write docs/plan.md' })
    expect(within(group).getByTitle('docs/plan.md')).toBeInTheDocument()
    expect(group).toHaveTextContent('wrote 2 lines')
    expect(group).toHaveTextContent('Refunds plan')
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument()
  })
})
