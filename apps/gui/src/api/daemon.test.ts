import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { classifySubmissionFailure, createConversation, handoffToChat, handoffToNative, loadClaudeExecutablePath, loadCodexExecutablePath, loadConversations, loadDaemonRestartStatus, loadMessages, restartMismatchedDaemon, resumeConversation } from './daemon'

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

describe('conversation handoff', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockResolvedValue(null)
  })

  it('hands the exact generation to the native terminal command', async () => {
    await handoffToNative(conversation)

    expect(invoke).toHaveBeenCalledWith('handoff_conversation_to_native', { id: 42, generation: 7 })
  })

  it('hands the exact generation back to structured chat', async () => {
    await handoffToChat(conversation)

    expect(invoke).toHaveBeenCalledWith('handoff_conversation_to_chat', { id: 42, generation: 7 })
  })

  it('refuses handoff outside the desktop host', async () => {
    vi.mocked(isTauri).mockReturnValue(false)

    await expect(handoffToNative(conversation)).rejects.toThrow('Native handoff is only available in the desktop app')
    expect(invoke).not.toHaveBeenCalled()
  })
})

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

    await expect(createConversation('/work/project', 'Claude')).resolves.toEqual({ id: 44, generation: 1 })
    expect(invoke).toHaveBeenCalledWith('create_conversation', {
      cwd: '/work/project',
      provider: 'Claude',
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
    const history = await loadMessages('42')

    expect(history.messages.map(({ content, activityState }) => [content, activityState])).toEqual([
      ['Running cargo test', 'Running'],
      ['Failed cargo test', 'Failed'],
      ['Interrupted command', 'Interrupted'],
    ])
  })

  it('loads every history page before returning the conversation', async () => {
    const cursor = { conversation: 42, generation: 7, revision: 3, next_position: 128 }
    const firstPage = Array.from({ length: 128 }, (_, index) => toolEntry(`item-${String(index)}`, `Task ${String(index)}`, '', 'Completed'))
    vi.mocked(invoke).mockReset()
      .mockResolvedValueOnce(readResult({ entries: firstPage, next: cursor }))
      .mockResolvedValueOnce(readResult({ entries: [toolEntry('item-128', 'Task 128', '', 'Completed')] }))

    const history = await loadMessages('42')

    expect(history.messages).toHaveLength(129)
    expect(history.completeness).toBe('Complete')
    expect(invoke).toHaveBeenNthCalledWith(2, 'read_conversation', { id: 42, cursor, limit: 128 })
  })

  it('keeps history incomplete when a page reports a missing section', async () => {
    vi.mocked(invoke).mockReset().mockResolvedValue(readResult({ completeness: { Incomplete: 'MissingPages' } }))

    const history = await loadMessages('42')

    expect(history.completeness).toEqual({ Incomplete: 'MissingPages' })
  })

  it('rejects a cursor that repeats instead of looping forever', async () => {
    const cursor = { conversation: 42, generation: 7, revision: 3, next_position: 128 }
    vi.mocked(invoke).mockReset()
      .mockResolvedValueOnce(readResult({ next: cursor }))
      .mockResolvedValueOnce(readResult({ next: cursor }))

    await expect(loadMessages('42')).rejects.toThrow('Conversation history cursor repeated')
    expect(invoke).toHaveBeenCalledTimes(2)
  })

  it('rejects malformed daemon history at the boundary', async () => {
    const validResult = readResult()
    vi.mocked(invoke).mockReset().mockResolvedValue({
      ...validResult,
      history: { ...validResult.history, completeness: null },
    })

    await expect(loadMessages('42')).rejects.toThrow()
  })
})

type HistoryCompleteness = 'Unknown' | 'Loading' | 'Complete' | { Incomplete: 'Disconnected' | 'UnsupportedItem' | 'InvalidEvent' | 'MissingPages' | 'RevisionExhausted' }
type HistoryCursor = { conversation: number; generation: number; revision: number; next_position: number }

function readResult({
  entries = [],
  next = null,
  completeness = 'Complete',
}: { entries?: object[]; next?: HistoryCursor | null; completeness?: HistoryCompleteness } = {}) {
  return {
    conversation: 42,
    generation: 7,
    status: 'Idle',
    receipt: null,
    error: null,
    cancelling: false,
    history: { entries, next, completeness },
    checks: [],
    approvals: [],
  }
}

function toolEntry(item: string, kind: string, output: string, state: 'Running' | 'Completed' | 'Failed' | 'Interrupted') {
  return {
    source: { conversation: 42, turn: 'turn-1', item },
    content: { Tool: { kind, output } },
    state,
  }
}
