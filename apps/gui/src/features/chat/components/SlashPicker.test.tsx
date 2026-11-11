import { act, cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { forgetSlashCommands } from '../slashCommands'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  loadAgentCommands: vi.fn<(agent: string, folder: string) => Promise<{ name: string; description: string; hint: string; aliases: string[] }[]>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: fake.submitMessage,
  loadAgentCommands: fake.loadAgentCommands,
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

const COMMANDS = [
  { name: 'clear', description: 'Start a new conversation', hint: '', aliases: [] },
  { name: 'compact', description: 'Keep a summary of the conversation', hint: '', aliases: [] },
  { name: 'model', description: 'Pick the model', hint: '[model]', aliases: [] },
  { name: 'developer:pr', description: 'Open a pull request', hint: '', aliases: ['pr'] },
]
const ROOT = '/home/me/app'

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  forgetSlashCommands()
  fake.submitMessage.mockReset().mockResolvedValue({})
  fake.loadAgentCommands.mockReset().mockResolvedValue(COMMANDS)
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function open(props: Partial<Parameters<typeof ChatSession>[0]> = {}) {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
  fake.current = daemon
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1 }} agent="claude" online root={ROOT} {...props} />)
  await settle()
  return { user, box: screen.getByRole('textbox', { name: `Message ${props.agent ?? 'claude'}` }) }
}

const offered = (agent = 'claude') => within(screen.getByRole('listbox', { name: `${agent} commands` })).getAllByRole('option').map((option) => option.textContent)

describe('the agent\'s commands in the message box', () => {
  it('asks the agent for its commands in the session\'s folder as the chat opens, and lists them when a slash is typed', async () => {
    const { user, box } = await open()
    expect(fake.loadAgentCommands).toHaveBeenCalledExactlyOnceWith('claude', ROOT)
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()

    await user.type(box, '/')

    expect(offered()).toEqual(['/clearStart a new conversation', '/compactKeep a summary of the conversation', '/developer:prOpen a pull request', '/model [model]Pick the model'])
  })

  it('narrows the list as a name is typed, and finds a plugin\'s command by its own name', async () => {
    const { user, box } = await open()

    await user.type(box, '/c')
    expect(offered()).toEqual(['/clearStart a new conversation', '/compactKeep a summary of the conversation'])

    await user.clear(box)
    await user.type(box, '/pr')
    expect(offered()).toEqual(['/developer:prOpen a pull request'])
  })

  it('moves through the list with the arrows and completes the one they are on with Tab, sending nothing', async () => {
    const { user, box } = await open()

    await user.type(box, '/c{ArrowDown}')
    expect(screen.getByRole('option', { selected: true })).toHaveTextContent('/compact')
    await user.keyboard('{Tab}')

    expect(box).toHaveValue('/compact ')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('runs a command that takes nothing on Enter, and waits for what a command takes', async () => {
    const { user, box } = await open()

    await user.type(box, '/cl{Enter}')
    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith({ id: String(FAKE_CONVERSATION), generation: 1 }, 1, '/clear')
    expect(box).toHaveValue('')

    await user.type(box, '/mo{Enter}')
    expect(box).toHaveValue('/model ')
    expect(fake.submitMessage).toHaveBeenCalledOnce()
  })

  it('puts the list away on Esc and keeps what was typed, until the text changes', async () => {
    const { user, box } = await open()

    await user.type(box, '/c{Escape}')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(box).toHaveValue('/c')

    await user.type(box, 'o')
    expect(offered()).toEqual(['/compactKeep a summary of the conversation'])
  })

  it('picks the command that is clicked', async () => {
    const { user, box } = await open()

    await user.type(box, '/')
    await user.click(screen.getByRole('option', { name: /developer:pr/ }))

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith({ id: String(FAKE_CONVERSATION), generation: 1 }, 1, '/developer:pr')
  })

  it('says so when no command has the typed name, and Enter sends it as typed', async () => {
    const { user, box } = await open()

    await user.type(box, '/nope')
    expect(screen.getByText(/No command by that name/)).toBeInTheDocument()
    await user.keyboard('{Enter}')

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith({ id: String(FAKE_CONVERSATION), generation: 1 }, 1, '/nope')
  })

  it('types the command into an agent that runs in a terminal, where a pasted one would be read as words', async () => {
    const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    const { user, box } = await open({ link: { conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }, typeInto })

    await user.type(box, '/comp{Enter}')

    expect(typeInto).toHaveBeenCalledExactlyOnceWith('/compact\r')
  })

  it('offers the commands only the terminal runs to an agent that has one, and shows the terminal when one is run', async () => {
    const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    const onShowTerminal = vi.fn()
    const { user, box } = await open({ link: { conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }, typeInto, onShowTerminal })

    await user.type(box, '/perm')
    expect(offered()).toEqual(['/permissionsterminalManage allow and deny tool permission rules'])
    await user.keyboard('{Enter}')

    expect(onShowTerminal).toHaveBeenCalledOnce()
    expect(typeInto).toHaveBeenCalledExactlyOnceWith('/permissions\r')
    expect(box).toHaveValue('')
  })

  it('leaves the terminal\'s commands out for a chat session, which has no terminal to run them in', async () => {
    const { user, box } = await open()

    await user.type(box, '/perm')

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })

  it('offers Codex its own commands, and types one with a pause before Enter, as Codex takes it', async () => {
    const typeInto = vi.fn<(text: string) => Promise<void>>().mockResolvedValue(undefined)
    const { user, box } = await open({ agent: 'codex', link: { conversation: FAKE_CONVERSATION, generation: 1, readOnly: true }, typeInto })
    expect(fake.loadAgentCommands).toHaveBeenCalledWith('codex', ROOT)

    await user.type(box, '/comp')
    expect(offered('codex')).toEqual(['/compactSummarize the conversation to prevent hitting the context limit'])
    await user.keyboard('{Enter}')
    await settle(1000)

    expect(typeInto.mock.calls.map(([text]) => text)).toEqual(['/compact', '\r'])
  })

  it('offers nothing in a box that is not a command: a path in a sentence, or a slash after a space', async () => {
    const { user, box } = await open()

    await user.type(box, 'look at /tmp')

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
  })
})
