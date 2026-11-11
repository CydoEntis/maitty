import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { toast } from '@/lib/toast'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({ current: null as null | { read: (cursor: never) => Promise<unknown> } }))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

const writeText = vi.fn()

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  writeText.mockReset().mockResolvedValue(undefined)
})

afterEach(() => {
  window.getSelection()?.removeAllRanges()
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

function select(node: Node) {
  const range = document.createRange()
  range.selectNodeContents(node)
  const selection = window.getSelection()
  selection?.removeAllRanges()
  selection?.addRange(range)
}

async function open() {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'The lockfile was out of date.'))
  fake.current = daemon
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  // After the setup, which puts a clipboard of its own on the page.
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText } })
  render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1 }} agent="claude" online />)
  await settle()
  const said = screen.getByText('The lockfile was out of date.')
  return { user, said, box: screen.getByRole('textbox', { name: 'Message claude' }) }
}

describe('text selected in the chat', () => {
  it('offers Copy and Quote where the mouse let go, and copies nothing by itself', async () => {
    const { said } = await open()
    select(said)

    fireEvent.mouseUp(said)

    expect(screen.getByRole('group', { name: 'Selected text' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Copy' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Quote' })).toBeInTheDocument()
    expect(writeText).not.toHaveBeenCalled()
  })

  it('copies the text when Copy is pressed, says so, and puts the bar away', async () => {
    const copied = vi.spyOn(toast, 'success').mockImplementation(() => '')
    const { user, said } = await open()
    select(said)
    fireEvent.mouseUp(said)

    await user.click(screen.getByRole('button', { name: 'Copy' }))

    expect(writeText).toHaveBeenCalledExactlyOnceWith('The lockfile was out of date.')
    await waitFor(() => { expect(copied).toHaveBeenCalledWith('Copied', expect.anything()) })
    expect(screen.queryByRole('group', { name: 'Selected text' })).not.toBeInTheDocument()
  })

  it('says when the clipboard refused it', async () => {
    writeText.mockRejectedValue(new Error('denied'))
    const failed = vi.spyOn(toast, 'error').mockImplementation(() => '')
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { user, said } = await open()
    select(said)
    fireEvent.mouseUp(said)

    await user.click(screen.getByRole('button', { name: 'Copy' }))

    await waitFor(() => { expect(failed).toHaveBeenCalledWith('Could not copy the selection') })
  })

  it('quotes the text into the message box under what was already written, with a line to reply on', async () => {
    const { user, said, box } = await open()
    await user.type(box, 'About this:')
    select(said)
    fireEvent.mouseUp(said)

    await user.click(screen.getByRole('button', { name: 'Quote' }))

    expect(box).toHaveValue('About this:\n\n> The lockfile was out of date.\n\n')
    expect(screen.queryByRole('group', { name: 'Selected text' })).not.toBeInTheDocument()
    expect(writeText).not.toHaveBeenCalled()
  })

  it('offers nothing for a plain click, or a release in the message box', async () => {
    const { said, box } = await open()

    fireEvent.mouseUp(said)
    expect(screen.queryByRole('group', { name: 'Selected text' })).not.toBeInTheDocument()

    select(said)
    fireEvent.mouseUp(box)
    expect(screen.queryByRole('group', { name: 'Selected text' })).not.toBeInTheDocument()
  })

  it('puts the bar away on Esc', async () => {
    const { user, said } = await open()
    select(said)
    fireEvent.mouseUp(said)

    await user.keyboard('{Escape}')

    expect(screen.queryByRole('group', { name: 'Selected text' })).not.toBeInTheDocument()
  })
})
