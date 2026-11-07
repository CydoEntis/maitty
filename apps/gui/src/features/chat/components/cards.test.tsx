import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { DiffLine } from '../chatEvents'
import { approvalEvent, commandEvent, editEvent, subagentEvent, todoEvent, toolEvent } from '../eventFixtures'
import { AgentsCard } from './AgentsCard'
import { ApprovalCard } from './ApprovalCard'
import { Markdown } from './Markdown'
import { CommandCard, EditCard, TodoCard, ToolChip } from './StepCards'

afterEach(cleanup)

/** The diff's lines as drawn, each its own row of syntax-coloured text. */
const diffLines = () => screen.queryAllByText((_, element) => element?.classList.contains('mx-diff-text') === true && /^line \d+$/.test(element.textContent))

describe('command card', () => {
  const finished = (over = {}) => commandEvent('c', { command: 'pnpm test', output: 'one\ntwo\nthree\nfour\nfive', ...over })

  it.each([
    ['exit 0 in green with the provider duration', finished({ exitCode: 0, durationMs: 1_400 }), '1s', 'exit 0'],
    ['a non-zero exit', finished({ exitCode: 2, durationMs: 2_000, status: 'failed' }), '2s', 'exit 2'],
    ['an honest unavailable exit code (Claude reports none)', { ...finished(), timing: { startedAtMs: 1_000, completedAtMs: 4_000 } }, '3s', 'exit code unavailable'],
    ['unavailable time when the history kept none', finished({ exitCode: 0 }), 'time unavailable', 'exit 0'],
  ])('shows %s', (_name, event, took, exit) => {
    render(<CommandCard event={event} />)

    expect(screen.getByText(took)).toBeInTheDocument()
    expect(screen.getByText(exit)).toBeInTheDocument()
  })

  it('shows Running with a live clock while it runs, and no exit code', () => {
    vi.useFakeTimers()
    vi.setSystemTime(10_000)
    render(<CommandCard event={commandEvent('c', { status: 'running', timing: { startedAtMs: 7_000, completedAtMs: null } })} />)

    expect(screen.getByText(/Running · 0:03/)).toBeInTheDocument()
    expect(screen.queryByText(/exit/)).toBeNull()
    vi.useRealTimers()
  })

  it('shows the last 3 lines and all of them on request', async () => {
    render(<CommandCard event={finished({ exitCode: 0 })} />)

    const group = screen.getByRole('group', { name: 'Command pnpm test' })
    expect(group).toHaveTextContent('three four five')
    expect(group).not.toHaveTextContent('one')
    await userEvent.click(screen.getByRole('button', { name: 'Show all 5 lines' }))
    expect(group).toHaveTextContent('one two three four five')
    await userEvent.click(screen.getByRole('button', { name: 'Show less' }))
    expect(group).not.toHaveTextContent('one')
  })

  it('has no toggle for output that fits', () => {
    render(<CommandCard event={finished({ output: 'a\nb' })} />)

    expect(screen.queryByRole('button')).toBeNull()
  })

  it('shows a masked token as it is and says the daemon hid it', () => {
    render(<CommandCard event={commandEvent('c', { command: 'echo $T', output: 'token is [redacted]', exitCode: 0 })} />)

    expect(screen.getByText('token is [redacted]')).toBeInTheDocument()
    expect(screen.getByText(/looked like a secret/)).toBeInTheDocument()
    expect(screen.getByText('exit 0')).toBeInTheDocument()
  })

  it('says a secret file was read instead of showing the marker as output', () => {
    render(<CommandCard event={commandEvent('c', { command: 'cat .env', output: '[content of a secret file omitted]', exitCode: 0 })} />)

    expect(screen.getByText(/shows a secret file/)).toBeInTheDocument()
    expect(screen.queryByText('[content of a secret file omitted]')).toBeNull()
  })
})

describe('edit block', () => {
  it('names a secret file and says its content is not kept, not that the diff failed or is empty', () => {
    render(<EditCard event={editEvent('e', { file: 'app/.env', change: 'add', diff: { status: 'omitted' } })} root={null} onOpen={null} />)

    expect(screen.getByRole('group', { name: 'Created app/.env' })).toBeInTheDocument()
    expect(screen.getByText(/keeps its name but not its content/)).toBeInTheDocument()
    expect(screen.queryByText(/unavailable/)).toBeNull()
    expect(screen.queryByText(/^[+−]\d+$/)).toBeNull()
  })

  const lines = (count: number): DiffLine[] => Array.from({ length: count }, (_, index) => ({ kind: 'add', text: `line ${String(index)}` }))
  const long = (count: number) => editEvent('e', { diff: { status: 'available', hunks: [{ header: '', lines: lines(count) }], added: count, removed: 0, truncated: false } })

  it('shows the file relative to the project, with +N and −N hidden at zero', () => {
    render(<EditCard event={editEvent('e', { file: '/repo/src/a.ts' })} root="/repo" onOpen={null} />)

    expect(screen.getByRole('group', { name: 'Edited src/a.ts' })).toBeInTheDocument()
    expect(screen.getByText('+1')).toBeInTheDocument()
    expect(screen.getByText('−1')).toBeInTheDocument()
    cleanup()
    render(<EditCard event={long(3)} root={null} onOpen={null} />)
    expect(screen.queryByText(/^−/)).toBeNull()
  })

  it.each([
    [12, null],
    [13, '1 more lines'],
    [35, '23 more lines'],
  ])('folds %i lines to 12 (%s)', async (count, footer) => {
    render(<EditCard event={long(count)} root={null} onOpen={null} />)

    expect(diffLines()).toHaveLength(Math.min(count, 12))
    if (footer === null) {
      expect(screen.queryByRole('button')).toBeNull()
      return
    }
    await userEvent.click(screen.getByRole('button', { name: footer }))
    expect(diffLines()).toHaveLength(count)
    expect(screen.getByRole('button', { name: 'Show less' })).toHaveAttribute('aria-expanded', 'true')
  })

  it('says the diff is unavailable instead of inventing one', () => {
    render(<EditCard event={editEvent('e', { file: null, diff: { status: 'unavailable' } })} root={null} onOpen={null} />)

    expect(screen.getByText(/diff is unavailable/)).toBeInTheDocument()
  })

  it('opens the file in the Changes panel from its header', async () => {
    const onOpen = vi.fn()
    render(<EditCard event={editEvent('e', { file: '/repo/src/a.ts' })} root="/repo" onOpen={onOpen} />)

    await userEvent.click(screen.getByRole('button', { name: 'Open the diff of src/a.ts in the Changes panel' }))
    expect(onOpen).toHaveBeenCalledWith('/repo/src/a.ts')
  })
})

describe('tool chip', () => {
  it('shows name and target, and opens the input and result', async () => {
    render(<ToolChip event={toolEvent('t', { name: 'Read', target: 'src/a.ts', input: '{\n  "file_path": "src/a.ts"\n}', detail: 'file text' })} />)

    const chip = screen.getByRole('button', { name: /Read.*src\/a\.ts/ })
    expect(chip).toHaveAttribute('aria-expanded', 'false')
    await userEvent.click(chip)
    expect(screen.getByText('Input')).toBeInTheDocument()
    expect(screen.getByText('file text')).toBeInTheDocument()
  })

  it('says so when the history kept nothing for the step', async () => {
    render(<ToolChip event={toolEvent('t', { name: 'Glob' })} />)

    await userEvent.click(screen.getByRole('button', { name: /Glob/ }))
    expect(screen.getByText('The conversation history kept no detail for this step.')).toBeInTheDocument()
  })
})

describe('checklist card', () => {
  it('counts what is done and marks each item', () => {
    render(<TodoCard event={todoEvent('d', [{ text: 'Plan', state: 'done' }, { text: 'Build', state: 'current' }, { text: 'Ship', state: 'pending' }])} />)

    expect(screen.getByText('Checklist · 1 of 3')).toBeInTheDocument()
    expect(screen.getByLabelText('Done')).toBeInTheDocument()
    expect(screen.getByLabelText('In progress')).toBeInTheDocument()
    expect(screen.getByLabelText('To do')).toBeInTheDocument()
  })

  it('does not invent items the history lacks', () => {
    render(<TodoCard event={todoEvent('d', null)} />)

    expect(screen.getByText(/did not keep this checklist/)).toBeInTheDocument()
  })
})

describe('helper agents card', () => {
  it('summarises, shows done time, and opens the prompt and report', async () => {
    const done = { ...subagentEvent('a', { title: 'Audit', prompt: 'Read parser.rs', report: 'Found 3 issues.' }), timing: { startedAtMs: 0, completedAtMs: 330_000 } }
    render(<AgentsCard agents={[done, subagentEvent('b', { title: 'Search', status: 'running' })]} />)

    expect(screen.getByRole('group', { name: '2 agents, 1 working' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Ran a subagent: Audit · 5m 30s/ })).toBeInTheDocument()
    await userEvent.click(screen.getByRole('button', { name: /Audit/ }))
    expect(screen.getByText('Read parser.rs')).toBeInTheDocument()
    expect(screen.getByText('Found 3 issues.')).toBeInTheDocument()
  })

  it('has nothing to open for a helper with no prompt or report', () => {
    render(<AgentsCard agents={[subagentEvent('a', { title: 'Quiet' })]} />)

    expect(screen.getByRole('button', { name: /Quiet/ })).toBeDisabled()
  })
})

describe('approval card', () => {
  const actions = (busy = false) => ({ answer: vi.fn(), answerInput: vi.fn(), busy })

  it.each([
    ['y', 'Accept'],
    ['a', 'AcceptForSession'],
    ['n', 'Decline'],
    ['Y', 'Accept'],
  ])('answers %s with %s by request id while the card is focused', async (key, decision) => {
    const handlers = actions()
    render(<ApprovalCard event={approvalEvent('req-1')} actions={handlers} step={null} onGoToStep={null} />)

    screen.getByRole('group', { name: /Approval/ }).focus()
    await userEvent.keyboard(key)

    expect(handlers.answer).toHaveBeenCalledExactlyOnceWith('req-1', decision)
  })

  it.each([
    ['a choice the request does not offer', approvalEvent(1, { choices: ['Accept'] }), 'n'],
    ['an answered request', approvalEvent(1, { resolution: { status: 'answered', answer: 'Accept' } }), 'y'],
    ['another key', approvalEvent(1), 'x'],
    ['a user-input request', approvalEvent(1, { approvalKind: 'UserInput' }), 'y'],
  ])('ignores the key for %s', async (_name, event, key) => {
    const handlers = actions()
    render(<ApprovalCard event={event} actions={handlers} step={null} onGoToStep={null} />)

    screen.getByRole('group', { name: /Approval/ }).focus()
    await userEvent.keyboard(key)

    expect(handlers.answer).not.toHaveBeenCalled()
  })

  it('does not take a key with a modifier, or while an answer is on its way', async () => {
    const handlers = actions(true)
    render(<ApprovalCard event={approvalEvent(1)} actions={handlers} step={null} onGoToStep={null} />)
    screen.getByRole('group', { name: /Approval/ }).focus()

    await userEvent.keyboard('y')
    await userEvent.keyboard('{Control>}n{/Control}')

    expect(handlers.answer).not.toHaveBeenCalled()
  })

  it('answers by id from a button too', async () => {
    const handlers = actions()
    render(<ApprovalCard event={approvalEvent(7)} actions={handlers} step={null} onGoToStep={null} />)

    await userEvent.click(screen.getByRole('button', { name: 'This session (a)' }))

    expect(handlers.answer).toHaveBeenCalledExactlyOnceWith(7, 'AcceptForSession')
  })

  it('reads "You answered" with no buttons when it was answered elsewhere', () => {
    render(<ApprovalCard event={approvalEvent(1, { resolution: { status: 'answered', answer: 'AcceptForSession', answeredAtMs: 1_000 } })} actions={actions()} step={null} onGoToStep={null} />)

    expect(screen.getByText(/You answered: This session/)).toBeInTheDocument()
    expect(screen.queryByRole('button')).toBeNull()
  })

  it('says only "Answered." when the choice was never recorded', () => {
    render(<ApprovalCard event={approvalEvent(1, { resolution: { status: 'answered', answer: 'unknown' } })} actions={actions()} step={null} onGoToStep={null} />)

    expect(screen.getByText('Answered.')).toBeInTheDocument()
  })

  it('links to its tool step', async () => {
    const onGoToStep = vi.fn()
    render(<ApprovalCard event={approvalEvent(1)} actions={actions()} step={{ key: 't1:c', label: 'pnpm test' }} onGoToStep={onGoToStep} />)

    await userEvent.click(screen.getByRole('button', { name: 'Go to the step this is about: pnpm test' }))
    expect(onGoToStep).toHaveBeenCalledWith('t1:c')
  })

  it('tells the person to use the terminal when the chat cannot answer', () => {
    render(<ApprovalCard event={approvalEvent(1)} actions={null} step={null} onGoToStep={null} />)

    expect(screen.getByText(/Answer it in the terminal/)).toBeInTheDocument()
  })

  it('is not a tab stop once answered', () => {
    render(<ApprovalCard event={approvalEvent(1, { resolution: { status: 'answered', answer: 'Decline' } })} actions={actions()} step={null} onGoToStep={null} />)

    expect(screen.getByRole('group', { name: /Approval/ })).not.toHaveAttribute('tabindex')
  })
})

describe('markdown view of untrusted agent text', () => {
  it('renders structure as elements', () => {
    render(<Markdown text={'# Title\n\nSome **bold** and `code`.\n\n- one\n- two\n\n```sh\nls\n```'} />)

    expect(screen.getByRole('heading', { name: 'Title', level: 1 })).toBeInTheDocument()
    expect(screen.getByText('bold').tagName).toBe('STRONG')
    expect(screen.getAllByRole('listitem')).toHaveLength(2)
    expect(screen.getByLabelText('Code, sh')).toHaveTextContent('ls')
  })

  it('turns no markup into elements, whatever the agent writes', () => {
    const hostile = [
      '<script>window.__pwned = 1</script>',
      '<img src=x onerror="window.__pwned = 2">',
      '<a href="javascript:window.__pwned = 3">raw anchor</a>',
      '[js](javascript:window.__pwned=4)',
      '[data](data:text/html,<script>1</script>)',
      '![tracker](https://tracker.example/p.png)',
      '<iframe src="https://evil.example"></iframe>',
    ].join('\n\n')
    const { container } = render(<Markdown text={hostile} />)

    expect(container.querySelector('script, img, iframe, a')).toBeNull()
    expect(container.querySelector('[onerror], [href], [src]')).toBeNull()
    expect(container).toHaveTextContent('<script>window.__pwned = 1</script>')
    expect(container).toHaveTextContent('tracker')
    expect((window as unknown as { __pwned?: number }).__pwned).toBeUndefined()
  })

  it('keeps an http link as a button that never navigates', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
    const { container } = render(<Markdown text="see [the docs](https://example.com/docs)" />)

    expect(container.querySelector('a')).toBeNull()
    await userEvent.click(within(container).getByRole('button', { name: 'the docs' }))
    expect(writeText).toHaveBeenCalledWith('https://example.com/docs')
  })
})
