import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { conversationsFromSnapshot } from '@/api/daemon'
import { forgetMachines, registerMachine } from '@/api/machines'
import { toSessionView } from '../model/session'
import { InboxPanel } from './InboxPanel'

function entry(machine?: { id: number; name: string }) {
  const term = {
    id: 3, cols: 80, rows: 24, title: 't', process: 'claude', agent: null, status: 'Blocked', cwd: '/srv', summary: '', name: 'job',
    label: '', pinned: false, archived: false, model: '', said: '', branch: null, linked: false, subagents: [],
    conversation_id: null, popup: false, remote: null, root: null, top: null, since: 0, asleep: false, win32_input: false,
  }
  if (machine) registerMachine(machine.name, machine.id)
  const [conversation] = conversationsFromSnapshot({ terms: { '3': term }, workspaces: [], conversations: [] }, machine)
  if (conversation === undefined) throw new Error('one session expected')
  return { session: toSessionView(conversation, '/home/me'), projectName: 'srv', projectColor: 'red' }
}

function show(entries: ReturnType<typeof entry>[]) {
  const none = vi.fn()
  render(<InboxPanel entries={entries} tracks={new Map()} online onAnswer={none} onClear={none} onClearDone={none} onOpen={none} onClose={none} />)
}

describe('the Inbox', () => {
  it('says a session of another machine is on it', () => {
    show([entry({ id: 1, name: 'build' })])
    expect(screen.getByText(/claude · on build/)).toBeTruthy()
    forgetMachines()
  })

  it('says nothing of a machine for a session of this computer', () => {
    show([entry()])
    expect(screen.queryByText(/ on /)).toBeNull()
  })
})
