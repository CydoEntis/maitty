import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { toast } from 'sonner'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { MOST_ATTACHMENTS, type Attached } from '@/api/attachments'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { ATTACHMENT_HEADER } from '../attachmentMessage'
import { dropOnComposer } from '../composerDrop'
import { forgetComposers } from '../composerStore'
import { PENDING_APPROVAL_NOTE, READ_ONLY_NOTE } from '../useChatComposer'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  attachLocalFile: vi.fn<(path: string) => Promise<Attached>>(),
  attachmentsExist: vi.fn<(paths: readonly string[]) => Promise<boolean[]>>(),
  chooseFilesToAttach: vi.fn<() => Promise<string[] | null>>(),
  saveImageLocally: vi.fn<(bytes: Uint8Array) => Promise<string>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: fake.submitMessage,
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

vi.mock('@/api/attachments', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/attachments')>()),
  attachLocalFile: fake.attachLocalFile,
  attachmentsExist: fake.attachmentsExist,
  chooseFilesToAttach: fake.chooseFilesToAttach,
}))

vi.mock('@/api/imagePaste', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/imagePaste')>()),
  saveImageLocally: fake.saveImageLocally,
}))

const TARGET = { id: String(FAKE_CONVERSATION), generation: 1 }
const KEPT_DIR = '/cache/attachments/attach-1730000000000-0a1b2c3d-'

function kept(name: string, size = 2048, kind: Attached['kind'] = 'file'): Attached {
  return { path: `${KEPT_DIR}${name}`, name, size, kind }
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  for (const mock of Object.values(fake)) if (typeof mock === 'function') mock.mockReset()
  fake.submitMessage.mockResolvedValue({})
  fake.attachmentsExist.mockImplementation((paths) => Promise.resolve(paths.map(() => true)))
  fake.attachLocalFile.mockImplementation((path) => Promise.resolve(kept(path.split('/').pop() ?? 'file')))
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  vi.restoreAllMocks()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

type Setup = { online?: boolean; readOnly?: boolean; prepare?: (daemon: FakeDaemonHistory) => void }

async function open({ online = true, readOnly = false, prepare = () => undefined }: Setup = {}) {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
  prepare(daemon)
  fake.current = daemon
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1, ...(readOnly ? { readOnly: true } : {}) }} agent="claude" online={online} />)
  await settle()
  return { daemon, user, box: screen.getByRole('textbox', { name: 'Message claude' }) }
}

/** The part of a paste's clipboard the composer reads; jsdom has no DataTransfer. */
function clipboard(text: string, files: File[]): DataTransfer {
  return {
    getData: (type: string) => (type === 'text/plain' ? text : ''),
    items: files.map((file) => ({ kind: 'file', type: file.type, getAsFile: () => file })),
  } as unknown as DataTransfer
}

const PNG = new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], 'image.png', { type: 'image/png' })

const composerEl = (box: HTMLElement) => box.closest('.mx-composer') as HTMLElement
const paperclip = () => screen.getByRole('button', { name: /^Attach files/ })
const sendButton = () => screen.getByRole('button', { name: /^(Send|Queue) message$/ })

describe('the file picker', () => {
  it('adds the chosen files as chips with name and size, and a chip can be removed with the keyboard', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(['/home/me/Design Final.zip', '/home/me/log.txt'])
    fake.attachLocalFile.mockImplementation((path) => Promise.resolve(
      path.endsWith('.zip') ? kept('Design_Final.zip', 3 * 1024 * 1024, 'archive') : kept('log.txt', 812),
    ))
    const { user } = await open()

    await user.click(paperclip())
    await settle()

    const list = screen.getByRole('list', { name: 'Attached files' })
    expect(within(list).getByText('Design_Final.zip')).toBeInTheDocument()
    expect(within(list).getByText('3.0 MB')).toBeInTheDocument()
    expect(within(list).getByText('812 B')).toBeInTheDocument()

    screen.getByRole('button', { name: 'Remove log.txt' }).focus()
    await user.keyboard('{Enter}')
    expect(within(list).queryByText('log.txt')).not.toBeInTheDocument()
    expect(within(list).getByText('Design_Final.zip')).toBeInTheDocument()
  })

  it('adds nothing when the picker is cancelled', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(null)
    const { user } = await open()

    await user.click(paperclip())
    await settle()

    expect(screen.queryByRole('list', { name: 'Attached files' })).not.toBeInTheDocument()
    expect(fake.attachLocalFile).not.toHaveBeenCalled()
  })

  it('names the host\'s refusal and keeps the files that were accepted', async () => {
    const error = vi.spyOn(toast, 'error')
    fake.chooseFilesToAttach.mockResolvedValue(['/x/folder', '/x/ok.txt'])
    fake.attachLocalFile.mockImplementation((path) => (path === '/x/folder' ? Promise.reject(new Error('folder is not a file; folders cannot be attached')) : Promise.resolve(kept('ok.txt'))))
    const { user } = await open()

    await user.click(paperclip())
    await settle()

    expect(error).toHaveBeenCalledWith('folder is not a file; folders cannot be attached')
    expect(screen.getByText('ok.txt')).toBeInTheDocument()
  })
})

describe('a drop on the composer', () => {
  it('attaches the dropped paths, and refuses a drop that would pass the count whole', async () => {
    const error = vi.spyOn(toast, 'error')
    const { box } = await open()
    const el = composerEl(box)

    act(() => { dropOnComposer(['/d/a.png', '/d/b.txt'], el) })
    await settle()
    expect(within(screen.getByRole('list', { name: 'Attached files' })).getAllByRole('listitem')).toHaveLength(2)

    const many = Array.from({ length: MOST_ATTACHMENTS }, (_, index) => `/d/many-${String(index)}.txt`)
    act(() => { dropOnComposer(many, el) })
    await settle()
    expect(error).toHaveBeenCalledWith(expect.stringContaining(`at most ${String(MOST_ATTACHMENTS)}`))
    expect(within(screen.getByRole('list', { name: 'Attached files' })).getAllByRole('listitem')).toHaveLength(2)
  })

  it('shows the host\'s cleaned name, not the dropped path', async () => {
    fake.attachLocalFile.mockResolvedValue(kept('passwd', 10))
    const { box } = await open()

    act(() => { dropOnComposer(['../../etc/passwd'], composerEl(box)) })
    await settle()

    expect(screen.getByText('passwd')).toBeInTheDocument()
    expect(screen.queryByText(/etc/)).not.toBeInTheDocument()
  })

  it('is refused with the reason while the composer is off', async () => {
    const error = vi.spyOn(toast, 'error')
    const { box } = await open({ online: false })

    act(() => { dropOnComposer(['/d/a.png'], composerEl(box)) })
    await settle()

    expect(error).toHaveBeenCalledWith('The daemon is offline')
    expect(fake.attachLocalFile).not.toHaveBeenCalled()
  })
})

const revoke = vi.fn<(url: string) => void>()

describe('pasting an image', () => {
  beforeEach(() => {
    URL.createObjectURL = vi.fn(() => 'blob:thumb')
    URL.revokeObjectURL = revoke
    revoke.mockReset()
  })

  it('attaches a bare image with a thumbnail made from the pasted bytes, and revokes the thumbnail on remove', async () => {
    fake.saveImageLocally.mockResolvedValue('/cache/pastes/paste-1730000000001-0a1b2c3d.png')
    const { user, box } = await open()

    fireEvent.paste(box, { clipboardData: clipboard('', [PNG]) })
    await settle()

    expect(fake.saveImageLocally).toHaveBeenCalledWith(expect.any(Uint8Array))
    const list = screen.getByRole('list', { name: 'Attached files' })
    expect(list.querySelector('img')).toHaveAttribute('src', 'blob:thumb')
    await user.click(screen.getByRole('button', { name: 'Remove Pasted image' }))
    expect(revoke).toHaveBeenCalledWith('blob:thumb')
    expect(screen.queryByRole('list', { name: 'Attached files' })).not.toBeInTheDocument()
  })

  it('leaves the paste to the text box when the clipboard holds text as well', async () => {
    const { box } = await open()

    fireEvent.paste(box, { clipboardData: clipboard('=SUM(A1:A3)', [PNG]) })
    await settle()

    expect(fake.saveImageLocally).not.toHaveBeenCalled()
    expect(screen.queryByRole('list', { name: 'Attached files' })).not.toBeInTheDocument()
  })

  it('refuses an image over 25 MB before reading it, and an unsupported format by name', async () => {
    const error = vi.spyOn(toast, 'error')
    const { box } = await open()
    const big = new File([new Uint8Array(1)], 'big.png', { type: 'image/png' })
    Object.defineProperty(big, 'size', { value: 26 * 1024 * 1024 })

    fireEvent.paste(box, { clipboardData: clipboard('', [big]) })
    fireEvent.paste(box, { clipboardData: clipboard('', [new File(['x'], 'a.bmp', { type: 'image/bmp' })]) })
    await settle()

    expect(error).toHaveBeenCalledWith(expect.stringContaining('26 MB'))
    expect(error).toHaveBeenCalledWith(expect.stringContaining('BMP'))
    expect(fake.saveImageLocally).not.toHaveBeenCalled()
  })

  it('tells why when the host refuses the bytes, and adds no chip', async () => {
    const error = vi.spyOn(toast, 'error')
    fake.saveImageLocally.mockRejectedValue(new Error('the clipboard holds something that is not a PNG'))
    const { box } = await open()

    fireEvent.paste(box, { clipboardData: clipboard('', [PNG]) })
    await settle()

    expect(error).toHaveBeenCalledWith('the clipboard holds something that is not a PNG', expect.anything())
    expect(screen.queryByRole('list', { name: 'Attached files' })).not.toBeInTheDocument()
  })
})

describe('sending', () => {
  it('sends the text with the files named by path, clears the draft, and shows the files on the sent message', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(['/home/me/Design Final.zip'])
    fake.attachLocalFile.mockResolvedValue(kept('Design_Final.zip', 4096, 'archive'))
    const { user, box } = await open()

    await user.click(paperclip())
    await settle()
    await user.type(box, 'use this design{Enter}')

    expect(fake.submitMessage).toHaveBeenCalledWith(TARGET, 1, `use this design\n\n${ATTACHMENT_HEADER}\n- ${KEPT_DIR}Design_Final.zip`)
    expect(screen.queryByRole('list', { name: 'Attached files' })).not.toBeInTheDocument()
    expect(box).toHaveValue('')
  })

  it('allows a message that is only files', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(['/home/me/log.txt'])
    const { user } = await open()

    expect(sendButton()).toBeDisabled()
    await user.click(paperclip())
    await settle()
    expect(sendButton()).toBeEnabled()
    await user.click(sendButton())

    expect(fake.submitMessage).toHaveBeenCalledWith(TARGET, 1, `${ATTACHMENT_HEADER}\n- ${KEPT_DIR}log.txt`)
  })

  it('gives the files back with the text when the send fails, and retries the same message', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(['/home/me/log.txt'])
    fake.submitMessage.mockRejectedValueOnce(new Error('the daemon went away'))
    const { user, box } = await open()

    await user.click(paperclip())
    await settle()
    await user.type(box, 'see log{Enter}')
    expect(await screen.findByRole('alert')).toHaveTextContent('the daemon went away')

    expect(box).toHaveValue('see log')
    expect(screen.getByText('log.txt')).toBeInTheDocument()
    await user.type(box, '{Enter}')
    expect(fake.submitMessage).toHaveBeenNthCalledWith(2, TARGET, 1, expect.stringContaining('see log'))
    expect(fake.submitMessage.mock.calls[1]?.[1]).toBe(fake.submitMessage.mock.calls[0]?.[1])
  })

  it('keeps the draft files when the view is switched away and back', async () => {
    fake.chooseFilesToAttach.mockResolvedValue(['/home/me/log.txt'])
    const { user } = await open()
    await user.click(paperclip())
    await settle()
    cleanup()

    render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1 }} agent="claude" online />)
    await settle()

    expect(screen.getByText('log.txt')).toBeInTheDocument()
  })
})

describe('a sent message', () => {
  const message = (daemon: FakeDaemonHistory) => {
    daemon.upsert(daemon.message('t2', 'u2', 'User', `check these\n\n${ATTACHMENT_HEADER}\n- ${KEPT_DIR}Design_Final.zip\n- /cache/pastes/paste-1730000000001-0a1b2c3d.png`))
  }

  it('shows its text and its files as chips, not as paths', async () => {
    await open({ prepare: message })

    expect(screen.getByText('check these')).toBeInTheDocument()
    expect(screen.getByText('Design_Final.zip')).toBeInTheDocument()
    expect(screen.getByText('Pasted image.png')).toBeInTheDocument()
    expect(screen.queryByText(/attachments\/attach-/)).not.toBeInTheDocument()
  })

  it('says a file is no longer available when the host no longer has it', async () => {
    fake.attachmentsExist.mockResolvedValue([false, true])
    await open({ prepare: message })

    const gone = screen.getByText('Design_Final.zip').closest('li')
    expect(gone).toHaveTextContent('no longer available')
    expect(screen.getByText('Pasted image.png').closest('li')).not.toHaveTextContent('no longer available')
  })
})

describe('when files cannot be attached', () => {
  it('says why on the paperclip while the daemon is offline and picks nothing', async () => {
    const error = vi.spyOn(toast, 'error')
    const { user } = await open({ online: false })

    expect(paperclip()).toHaveAccessibleName('Attach files: The daemon is offline')
    await user.click(paperclip())

    expect(fake.chooseFilesToAttach).not.toHaveBeenCalled()
    expect(error).not.toHaveBeenCalled()
  })

  it('says why on the paperclip while an approval waits', async () => {
    await open({
      prepare: (daemon) => {
        daemon.status = 'Blocked'
        daemon.approvals = [{ request_id: 9, kind: 'Command', turn_id: 't1', command: 'rm -rf build', choices: ['Accept', 'Decline'] }]
      },
    })

    expect(paperclip()).toHaveAccessibleName(`Attach files: ${PENDING_APPROVAL_NOTE}`)
    expect(paperclip()).toHaveAttribute('aria-disabled', 'true')
  })

  it('offers no paperclip in a read-only chat, whose composer already states the reason', async () => {
    const { box } = await open({ readOnly: true })

    expect(screen.queryByRole('button', { name: /^Attach files/ })).not.toBeInTheDocument()
    expect(box).toHaveAttribute('placeholder', READ_ONLY_NOTE)
  })
})
