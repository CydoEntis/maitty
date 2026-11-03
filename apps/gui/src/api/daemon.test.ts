import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { startTerminalSession, loadTerminalScreen, loadTerminalReplay, sendTerminalInput, resizeTerminal, archiveConversationOnDaemon, renameConversationOnDaemon, classifySubmissionFailure, createConversation, loadClaudeExecutablePath, loadCodexExecutablePath, loadConversationDiff, loadConversationFile, loadConversationFiles, loadConversations, loadDaemonRestartStatus, loadMessages, restartMismatchedDaemon, resumeConversation } from './daemon'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}))

const conversation = {
  id: '42',
  generation: 7,
  title: 'Example conversation',
  project: 'maitty',
  folder: '/home/casey/maitty',
  branch: 'main',
  provider: 'Codex' as const,
  status: 'idle' as const,
  updated: null,
  model: 'Codex default',
  permission: 'Provider permissions',
  context: null,
  nativePane: 12,
  hasDiff: false,
}

describe('workspace Git state', () => {
  it('marks a conversation folder without Git so Changes can explain why it is unavailable', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue({
      conversations: [{
        id: 42,
        generation: 7,
        execution: 'Structured',
        provider: 'Claude',
        cwd: 'C:\\dev\\autobattler',
        worktree: null,
        state: 'Running',
        status: 'Working',
        native_term: null,
        checks: [],
      }],
      workspaces: [{ cwd: 'C:\\dev\\autobattler', git: null }],
    })

    await expect(loadConversations()).resolves.toMatchObject([
      { folder: 'C:\\dev\\autobattler', isGitRepository: false },
    ])
  })
})

describe('read-only conversation file browser', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset()
  })

  it('lists and reads files using the selected checkout identity', async () => {
    vi.mocked(invoke).mockResolvedValueOnce([{ name: 'src', is_directory: true }])
      .mockResolvedValueOnce('export const ready = true')

    await expect(loadConversationFiles(conversation, 'src')).resolves.toEqual([
      { name: 'src', is_directory: true },
    ])
    await expect(loadConversationFile(conversation, 'src/index.ts')).resolves.toContain('ready')
    expect(invoke).toHaveBeenNthCalledWith(1, 'conversation_files', {
      id: 42,
      generation: 7,
      path: 'src',
    })
    expect(invoke).toHaveBeenNthCalledWith(2, 'conversation_file', {
      id: 42,
      generation: 7,
      path: 'src/index.ts',
    })
  })
})

describe('conversation changes query', () => {
  it('sends only the selected conversation id and generation to the daemon', async () => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue({
      files: [],
      additions: 0,
      deletions: 0,
      total_files: 0,
      truncated: false,
    })

    await expect(loadConversationDiff(conversation)).resolves.toMatchObject({ total_files: 0 })
    expect(invoke).toHaveBeenCalledWith('conversation_diff', { id: 42, generation: 7 })
  })
})

describe('Codex executable lookup', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue('C:\\Users\\casey\\AppData\\Roaming\\npm\\node_modules\\@openai\\codex\\vendor\\codex.exe')
  })

  it('loads the executable path from the desktop host', async () => {
    await expect(loadCodexExecutablePath()).resolves.toContain('codex.exe')
    expect(invoke).toHaveBeenCalledWith('codex_executable_path', undefined)
  })

  it('refuses executable lookup in the browser preview', async () => {
    vi.mocked(isTauri).mockReturnValue(false)

    await expect(loadCodexExecutablePath()).rejects.toThrow('Codex executable lookup is available in the desktop app')
    expect(invoke).not.toHaveBeenCalled()
  })
})

describe('Claude executable lookup and creation', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue('C:\\Users\\casey\\.local\\bin\\claude.exe')
  })

  it('loads Claude Code from the desktop host', async () => {
    await expect(loadClaudeExecutablePath()).resolves.toContain('claude.exe')
    expect(invoke).toHaveBeenCalledWith('claude_executable_path', undefined)
  })

  it('passes the selected provider to the daemon-backed create command', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 44, generation: 1 })

    await expect(createConversation('/work/project', 'Claude', 'full-access')).resolves.toEqual({ id: 44, generation: 1 })
    expect(invoke).toHaveBeenCalledWith('create_conversation', {
      cwd: '/work/project',
      provider: 'Claude',
      permissions: 'full-access',
    })
  })
})

describe('daemon protocol recovery', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue({ daemonProtocol: 49, guiProtocol: 59, canRestart: true, sessionCount: 2, activeTurnCount: 1 })
  })

  it('loads restart details from the desktop host', async () => {
    await expect(loadDaemonRestartStatus()).resolves.toMatchObject({ daemonProtocol: 49, canRestart: true, activeTurnCount: 1 })
    expect(invoke).toHaveBeenCalledWith('daemon_restart_status', undefined)
  })

  it('confirms only the protocol shown to the user', async () => {
    await restartMismatchedDaemon(49)
    expect(invoke).toHaveBeenCalledWith('restart_mismatched_daemon', { expectedProtocol: 49 })
  })

  it('rejects invalid protocol versions and browser-side restart requests', async () => {
    await expect(restartMismatchedDaemon(0)).rejects.toThrow('Daemon protocol is invalid')
    vi.mocked(isTauri).mockReturnValue(false)
    await expect(loadDaemonRestartStatus()).rejects.toThrow('Daemon restart status is available in the desktop app')
    await expect(restartMismatchedDaemon(49)).rejects.toThrow('Daemon restart is available in the desktop app')
    expect(invoke).not.toHaveBeenCalled()
  })
})

describe('conversation recovery', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue({ id: 42, generation: 8 })
  })

  it('resumes only the exact saved conversation generation', async () => {
    await expect(resumeConversation(conversation)).resolves.toEqual({ id: 42, generation: 8 })

    expect(invoke).toHaveBeenCalledWith('resume_codex_conversation', { id: 42, generation: 7 })
  })

  it('refuses recovery without a desktop host or conversation generation', async () => {
    vi.mocked(isTauri).mockReturnValue(false)
    await expect(resumeConversation(conversation)).rejects.toThrow('Conversation recovery is only available in the desktop app')

    vi.mocked(isTauri).mockReturnValue(true)
    const { generation, ...conversationWithoutGeneration } = conversation
    expect(generation).toBe(7)
    await expect(resumeConversation(conversationWithoutGeneration)).rejects.toThrow('Conversation generation is unavailable')
  })
})

describe('submission delivery', () => {
  const uncertainHistory = {
    generation: 7,
    receipt: { conversation: 42, generation: 7, operation: 3, delivery: 'SubmissionUncertain' as const },
  }

  it('offers no retry when the daemon says delivery is uncertain', () => {
    expect(classifySubmissionFailure(uncertainHistory, 7, 3)).toBe('unknown')
  })

  it('allows retry when there is no receipt for the failed operation', () => {
    expect(classifySubmissionFailure({ generation: 7, receipt: null }, 7, 3)).toBe('not-delivered')
    expect(classifySubmissionFailure(uncertainHistory, 7, 4)).toBe('not-delivered')
  })

  it('recognizes acknowledged delivery and rejects mismatched history', () => {
    expect(classifySubmissionFailure({
      generation: 7,
      receipt: { conversation: 42, generation: 7, operation: 3, delivery: { Acknowledged: { turn_id: 'turn-1' } } },
    }, 7, 3)).toBe('delivered')
    expect(classifySubmissionFailure(uncertainHistory, 8, 3)).toBe('unknown')
    expect(classifySubmissionFailure({ generation: 7, receipt: { ...uncertainHistory.receipt, generation: 8 } }, 7, 3)).toBe('unknown')
  })
})

describe('conversation activity history', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue(readResult({
      entries: [
        toolEntry('item-1', 'Running cargo test', 'Compiling…', 'Running'),
        toolEntry('item-2', 'Failed cargo test', 'exit code 1', 'Failed'),
        toolEntry('item-3', 'Interrupted command', '', 'Interrupted'),
      ],
    }))
  })

  it('preserves daemon tool states so running and failed work are not shown as completed', async () => {
    const history = await loadMessages('42', 'Codex')

    expect(history.messages.map(({ content, activityState }) => [content, activityState])).toEqual([
      ['Running cargo test', 'Running'],
      ['Failed cargo test', 'Failed'],
      ['Interrupted command', 'Interrupted'],
    ])
  })

  it('shows retained command labels and gives older history a readable fallback', async () => {
    vi.mocked(invoke).mockReset().mockResolvedValue(readResult({
      entries: [
        {
          ...toolEntry('item-1', 'commandExecution', 'GUI_CODEX_TOOL_OK', 'Completed'),
          content: {
            Tool: {
              kind: 'commandExecution',
              label: 'pwsh -NoProfile -Command Write-Output GUI_CODEX_TOOL_OK',
              output: 'GUI_CODEX_TOOL_OK',
            },
          },
        },
        toolEntry('item-2', 'commandExecution', 'older output', 'Completed'),
      ],
    }))

    const history = await loadMessages('42', 'Codex')

    expect(history.messages.map(({ content, detail }) => [content, detail])).toEqual([
      ['pwsh -NoProfile -Command Write-Output GUI_CODEX_TOOL_OK', 'GUI_CODEX_TOOL_OK'],
      ['Run command', 'older output'],
    ])
  })

  it('uses the conversation provider in approval labels', async () => {
    vi.mocked(invoke).mockReset().mockResolvedValue(readResult({
      approvals: [{
        request_id: 'request-1',
        kind: 'Command',
        command: 'npm test',
        reason: null,
        cwd: null,
        changes: [],
        questions: [],
        choices: ['Accept', 'Decline'],
      }],
    }))

    const history = await loadMessages('42', 'Claude')

    expect(history.messages[0]?.content).toBe('Claude wants to run a command')
  })

  it('loads every history page before returning the conversation', async () => {
    const cursor = { conversation: 42, generation: 7, revision: 3, next_position: 128 }
    const firstPage = Array.from({ length: 128 }, (_, index) => toolEntry(`item-${String(index)}`, `Task ${String(index)}`, '', 'Completed'))
    vi.mocked(invoke).mockReset()
      .mockResolvedValueOnce(readResult({ entries: firstPage, next: cursor }))
      .mockResolvedValueOnce(readResult({ entries: [toolEntry('item-128', 'Task 128', '', 'Completed')] }))

    const history = await loadMessages('42', 'Codex')

    expect(history.messages).toHaveLength(129)
    expect(history.completeness).toBe('Complete')
    expect(invoke).toHaveBeenNthCalledWith(2, 'read_conversation', { id: 42, cursor, limit: 128 })
  })

  it('keeps history incomplete when a page reports a missing section', async () => {
    vi.mocked(invoke).mockReset().mockResolvedValue(readResult({ completeness: { Incomplete: 'MissingPages' } }))

    const history = await loadMessages('42', 'Codex')

    expect(history.completeness).toEqual({ Incomplete: 'MissingPages' })
  })

  it('rejects a cursor that repeats instead of looping forever', async () => {
    const cursor = { conversation: 42, generation: 7, revision: 3, next_position: 128 }
    vi.mocked(invoke).mockReset()
      .mockResolvedValueOnce(readResult({ next: cursor }))
      .mockResolvedValueOnce(readResult({ next: cursor }))

    await expect(loadMessages('42', 'Codex')).rejects.toThrow('Conversation history cursor repeated')
    expect(invoke).toHaveBeenCalledTimes(2)
  })

  it('rejects malformed daemon history at the boundary', async () => {
    const validResult = readResult()
    vi.mocked(invoke).mockReset().mockResolvedValue({
      ...validResult,
      history: { ...validResult.history, completeness: null },
    })

    await expect(loadMessages('42', 'Codex')).rejects.toThrow()
  })
})

type HistoryCompleteness = 'Unknown' | 'Loading' | 'Complete' | { Incomplete: 'Disconnected' | 'UnsupportedItem' | 'InvalidEvent' | 'MissingPages' | 'RevisionExhausted' }
type HistoryCursor = { conversation: number; generation: number; revision: number; next_position: number }

function readResult({
  entries = [],
  next = null,
  completeness = 'Complete',
  approvals = [],
}: { entries?: object[]; next?: HistoryCursor | null; completeness?: HistoryCompleteness; approvals?: object[] } = {}) {
  return {
    conversation: 42,
    generation: 7,
    status: 'Idle',
    receipt: null,
    error: null,
    cancelling: false,
    history: { entries, next, completeness },
    checks: [],
    approvals,
  }
}

function toolEntry(item: string, kind: string, output: string, state: 'Running' | 'Completed' | 'Failed' | 'Interrupted') {
  return {
    source: { conversation: 42, turn: 'turn-1', item },
    content: { Tool: { kind, output } },
    state,
  }
}

describe('names and closed sessions shared through the daemon', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset()
  })

  const summary = {
    id: 42,
    generation: 7,
    execution: 'Structured',
    provider: 'Codex',
    cwd: '/home/casey/sample',
    worktree: null,
    state: 'Running',
    status: 'Idle',
    native_term: null,
    checks: [],
  }

  it('shows the name any client gave a session, and its closed state', async () => {
    vi.mocked(invoke).mockResolvedValue({
      conversations: [{ ...summary, title: 'Fix the login bug', archived: true }],
      workspaces: [],
    })

    await expect(loadConversations()).resolves.toMatchObject([
      { id: '42', title: 'Fix the login bug', archived: true },
    ])
  })

  it('words an unnamed session itself and treats an older daemon as not closed', async () => {
    vi.mocked(invoke).mockResolvedValue({ conversations: [summary], workspaces: [] })

    await expect(loadConversations()).resolves.toMatchObject([
      { title: 'Codex conversation 42', archived: false },
    ])
  })

  it('sends a rename and a close to the daemon rather than keeping them in the browser', async () => {
    vi.mocked(invoke).mockResolvedValue(null)

    await renameConversationOnDaemon({ id: '42' }, 'Fix the login bug')
    await archiveConversationOnDaemon({ id: '42' }, true)

    expect(invoke).toHaveBeenNthCalledWith(1, 'rename_conversation', { id: 42, title: 'Fix the login bug' })
    expect(invoke).toHaveBeenNthCalledWith(2, 'archive_conversation', { id: 42, archived: true })
  })
})

describe('terminal sessions shown beside chats', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset()
  })

  const term = (overrides: Record<string, unknown>) => ({
    id: 5,
    agent: 'claude',
    status: 'Working',
    cwd: '/home/casey/sample',
    root: null,
    branch: 'main',
    label: '',
    conversation_id: null,
    popup: false,
    remote: null,
    ...overrides,
  })

  it('keeps a session in the list with its last good state when one refresh cannot read it', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const snapshot = (terms: Record<string, unknown>) => ({ conversations: [], workspaces: [], terms })
    vi.mocked(invoke).mockResolvedValueOnce(snapshot({ '77': term({ id: 77, label: 'Server goodness' }) }))
    expect((await loadConversations()).map((item) => item.title)).toEqual(['Server goodness'])

    vi.mocked(invoke).mockResolvedValueOnce(snapshot({ '77': term({ id: 77, cwd: null }) }))
    expect((await loadConversations()).map((item) => item.title)).toEqual(['Server goodness'])
    expect(warn).toHaveBeenCalledTimes(1)

    vi.mocked(invoke).mockResolvedValueOnce(snapshot({}))
    expect(await loadConversations()).toEqual([])
    vi.mocked(invoke).mockResolvedValueOnce(snapshot({ '77': term({ id: 77, cwd: null }) }))
    expect(await loadConversations()).toEqual([])
    warn.mockRestore()
  })

  it('still lists the terminals when one chat record cannot be read', async () => {
    vi.mocked(invoke).mockResolvedValue({
      conversations: [{ id: 'not a conversation' }],
      workspaces: [],
      terms: { '5': term({ label: 'Fix login' }) },
    })

    const listed = await loadConversations()

    expect(listed.map((item) => item.title)).toEqual(['Fix login'])
  })

  it('lists shells and agents in terminals of their own, and leaves out popups, other machines and a chat terminal', async () => {
    vi.mocked(invoke).mockResolvedValue({
      conversations: [],
      workspaces: [],
      terms: {
        '5': term({ label: 'Fix login' }),
        '6': term({ id: 6, agent: null, status: 'Idle' }),
        '7': term({ id: 7, popup: true }),
        '8': term({ id: 8, remote: 'build-box' }),
        '9': term({ id: 9, conversation_id: 3 }),
        '10': term({ id: 10, agent: 'gemini' }),
        '11': { nonsense: true },
      },
    })

    const sessions = await loadConversations()

    expect(sessions.map((session) => session.id)).toEqual(['t5', 't6', 't10'])
    expect(sessions[0]).toMatchObject({
      id: 't5',
      kind: 'terminal',
      title: 'Fix login',
      provider: 'Claude',
      status: 'working',
      nativePane: 5,
      named: true,
      agent: 'claude',
    })
    expect(sessions[1]).toMatchObject({ id: 't6', title: 'Shell 6', agent: null, status: 'idle', named: false })
    expect(sessions[2]).toMatchObject({ id: 't10', agent: 'gemini' })
  })

  it('reads, types into, resizes, names and closes a terminal session by its pane number', async () => {
    vi.mocked(invoke).mockResolvedValue(null)
    const target = { id: 't5', generation: 0 }

    vi.mocked(invoke).mockResolvedValueOnce('screen')
    await expect(loadTerminalScreen(target)).resolves.toBe('screen')
    vi.mocked(invoke).mockResolvedValueOnce([1, 2])
    await expect(loadTerminalReplay(target)).resolves.toEqual(Uint8Array.from([1, 2]))
    await sendTerminalInput(target, 'ls\r')
    await resizeTerminal(target, 100, 30)
    await renameConversationOnDaemon({ id: 't5' }, 'Fix login')
    await archiveConversationOnDaemon({ id: 't5' }, true)
    await archiveConversationOnDaemon({ id: 't5' }, false)
    await startTerminalSession('/home/casey/sample', 'Claude')

    expect(vi.mocked(invoke).mock.calls.map(([name, args]) => [name, args])).toEqual([
      ['pane_terminal_screen', { term: 5 }],
      ['pane_terminal_replay', { term: 5 }],
      ['terminal_input', { term: 5, data: [108, 115, 13] }],
      ['pane_terminal_resize', { term: 5, cols: 100, rows: 30 }],
      ['rename_terminal_session', { term: 5, name: 'Fix login' }],
      ['close_terminal_session', { term: 5 }],
      ['start_terminal_session', { cwd: '/home/casey/sample', provider: 'Claude' }],
    ])
  })
})
