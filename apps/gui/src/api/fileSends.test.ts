import { beforeEach, describe, expect, it, vi } from 'vitest'
import { cancelFileSend, listenFileSends, newTransferId, sendFilesToMachine } from './fileSends'

const shell = vi.hoisted(() => ({
  isTauri: true,
  invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>(),
  handler: null as null | ((event: { payload: unknown }) => void),
}))

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => shell.isTauri, invoke: shell.invoke }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: (_name: string, handler: (event: { payload: unknown }) => void) => {
    shell.handler = handler
    return Promise.resolve(() => undefined)
  },
}))

beforeEach(() => {
  shell.isTauri = true
  shell.invoke.mockReset()
  shell.invoke.mockResolvedValue(undefined)
})

describe('file send commands', () => {
  it('names the machine, the paths and the id the host will use', async () => {
    await sendFilesToMachine('build', ['/a/b.log'], 'send-1')
    expect(shell.invoke).toHaveBeenCalledWith('send_files_to_machine', { machine: 'build', paths: ['/a/b.log'], transfer: 'send-1' })
    await cancelFileSend('send-1')
    expect(shell.invoke).toHaveBeenCalledWith('cancel_file_send', { transfer: 'send-1' })
  })

  it('passes on the words the host refuses with', async () => {
    shell.invoke.mockRejectedValue('docs is not a file; only files can be sent')
    await expect(sendFilesToMachine('build', ['/a/docs'], 'send-1')).rejects.toBe('docs is not a file; only files can be sent')
  })

  it('cannot send from a plain browser', async () => {
    shell.isTauri = false
    await expect(sendFilesToMachine('build', ['/a'], 'send-1')).rejects.toThrow('desktop app')
    await cancelFileSend('send-1')
    expect(shell.invoke).not.toHaveBeenCalled()
  })

  it('makes ids the host accepts: letters, digits and dashes, within 64', () => {
    const id = newTransferId()
    expect(id).toMatch(/^[A-Za-z0-9_-]{1,64}$/)
    expect(newTransferId()).not.toBe(id)
  })
})

describe('what the host says about a send', () => {
  it('passes on an event it can read and drops one it cannot', async () => {
    const heard: unknown[] = []
    await listenFileSends((event) => { heard.push(event) })
    shell.handler?.({ payload: { kind: 'sent', transfer: 't', file: 'a', index: 0, count: 1, remotePath: '/p' } })
    shell.handler?.({ payload: { kind: 'progress', transfer: 't', file: 'a', index: 0, count: 1, percent: 150 } })
    shell.handler?.({ payload: { kind: 'surprise' } })
    expect(heard).toEqual([{ kind: 'sent', transfer: 't', file: 'a', index: 0, count: 1, remotePath: '/p' }])
  })
})
