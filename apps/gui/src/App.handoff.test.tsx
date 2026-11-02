import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation, Message } from '@/data/preview'
import App from './App'

const daemon = vi.hoisted(() => ({
  answerUserInput: vi.fn(),
  approveRequest: vi.fn(),
  cancelTurn: vi.fn(),
  createConversation: vi.fn(),
  classifySubmissionFailure: vi.fn(),
  loadConversationDiff: vi.fn(),
  loadConversationFile: vi.fn(),
  loadConversationFiles: vi.fn(),
  loadClaudeExecutablePath: vi.fn(),
  loadCodexExecutablePath: vi.fn(),
  loadConversations: vi.fn(),
  loadDaemonRestartStatus: vi.fn(),
  loadMessages: vi.fn(),
  restartMismatchedDaemon: vi.fn(),
  resumeConversation: vi.fn(),
  startTerminalSession: vi.fn(),
  submitMessage: vi.fn(),
}))

const folderPicker = vi.hoisted(() => ({ chooseProjectFolder: vi.fn() }))

vi.mock('@/api/daemon', () => ({
  ...daemon,
  isDesktop: () => true,
}))

vi.mock('@/api/folderPicker', () => folderPicker)

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation }: { conversation: Conversation }) => (
    <div
      className="terminal-screen"
      role="region"
      aria-label={`Native ${conversation.provider} terminal`}
    >
      Pane {conversation.nativePane}
    </div>
  ),
}))

const structuredConversation: Conversation = {
  id: '42',
  generation: 7,
  execution: 'Structured',
  title: 'Example conversation',
  project: 'maitty',
  folder: '/home/casey/maitty',
  branch: 'main',
  provider: 'Codex',
  status: 'idle',
  updated: null,
  model: 'Codex default',
  permission: 'Provider permissions',
  context: null,
  nativePane: null,
  hasDiff: false,
}

describe('desktop chat and terminal sessions', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    daemon.classifySubmissionFailure.mockReturnValue('unknown')
    daemon.loadConversations.mockResolvedValue([structuredConversation])
    daemon.loadCodexExecutablePath.mockResolvedValue(
      'C:\\Program Files\\Codex\\codex.exe',
    )
    daemon.loadClaudeExecutablePath.mockResolvedValue(
      'C:\\Users\\casey\\.local\\bin\\claude.exe',
    )
    daemon.loadDaemonRestartStatus.mockResolvedValue({
      daemonProtocol: 49,
      guiProtocol: 59,
      canRestart: false,
      sessionCount: 2,
      activeTurnCount: 1,
    })
    daemon.loadMessages.mockResolvedValue({
      generation: 7,
      receipt: null,
      messages: [],
      nextOperation: 1,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    daemon.loadConversationDiff.mockResolvedValue({
      files: [],
      additions: 0,
      deletions: 0,
      total_files: 0,
      truncated: false,
    })
    daemon.loadConversationFiles.mockResolvedValue([
      { name: 'src', is_directory: true },
      { name: 'README.md', is_directory: false },
    ])
    daemon.loadConversationFile.mockResolvedValue('# Read only preview')
  })

  it('has no switch between chat and terminal on a chat session', async () => {
    renderDesktopApp()

    await screen.findAllByText(structuredConversation.title)
    expect(screen.queryByRole('group', { name: 'Conversation view' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Open in native terminal' })).not.toBeInTheDocument()
  })

  it('starts a terminal session, not a chat, when Terminal is chosen', async () => {
    const user = userEvent.setup()
    const folder = '/home/casey/projects/terminal-project'
    folderPicker.chooseProjectFolder.mockResolvedValue(folder)
    daemon.startTerminalSession.mockResolvedValue(undefined)
    renderDesktopApp()

    await user.keyboard('{Control>}n{/Control}')
    await user.click(
      screen.getByRole('button', { name: 'Browse for project folder' }),
    )
    await screen.findByDisplayValue(folder)
    await user.click(
      within(screen.getByRole('group', { name: 'Session type' })).getByRole(
        'button',
        { name: /^Terminal/ },
      ),
    )
    await user.click(
      screen.getByRole('button', { name: 'Start terminal session' }),
    )

    await waitFor(() => {
      expect(daemon.startTerminalSession).toHaveBeenCalledWith(folder, 'Codex')
    })
    expect(daemon.createConversation).not.toHaveBeenCalled()
  })

  it('shows a terminal session as a terminal with no Changes button', async () => {
    daemon.loadConversations.mockResolvedValue([
      {
        ...structuredConversation,
        id: 't9',
        generation: 0,
        kind: 'terminal',
        execution: 'Native',
        nativePane: 9,
      },
    ])
    renderDesktopApp()

    expect(
      await screen.findByRole('region', { name: 'Native Codex terminal' }),
    ).toHaveTextContent('Pane 9')
    expect(screen.queryByRole('button', { name: /Review/ })).not.toBeInTheDocument()
    expect(daemon.loadMessages).not.toHaveBeenCalled()
  })

  it('keeps Claude identified when its native terminal is open', async () => {
    const claudeConversation: Conversation = {
      ...structuredConversation,
      provider: 'Claude',
      execution: 'Native',
      nativePane: 43,
    }
    daemon.loadConversations.mockResolvedValue([claudeConversation])
    renderDesktopApp()

    expect(await screen.findByRole('region', { name: 'Native Claude terminal' })).toHaveTextContent('Pane 43')
  })

  it('shows first-launch readiness, opens a Linux project folder, and keeps Settings available', async () => {
    const user = userEvent.setup()
    const folder = '/home/casey/projects/maitty'
    daemon.loadConversations.mockResolvedValue([])
    folderPicker.chooseProjectFolder.mockResolvedValue(folder)
    renderDesktopApp()

    expect(
      await screen.findByRole('heading', {
        name: 'Run your coding agents side by side.',
      }),
    ).toBeInTheDocument()
    expect(await screen.findByText('Codex CLI ready')).toBeInTheDocument()
    expect(screen.getByText('Claude Code CLI ready')).toBeInTheDocument()

    await user.click(
      screen.getByRole('button', { name: /Open a project folder/ }),
    )
    await user.click(await screen.findByRole('button', { name: 'Browse…' }))
    expect(
      await screen.findByRole('textbox', { name: 'Project folder' }),
    ).toHaveValue(folder)
    await user.click(screen.getByRole('button', { name: 'Cancel' }))

    await user.click(screen.getByRole('button', { name: 'Open settings' }))
    expect(screen.getByRole('dialog')).toHaveTextContent('Settings')
    expect(daemon.loadCodexExecutablePath).toHaveBeenCalledOnce()
  })

  it('labels a working Claude conversation as Claude', async () => {
    daemon.loadConversations.mockResolvedValue([
      { ...structuredConversation, provider: 'Claude', status: 'working' },
    ])
    renderDesktopApp()

    expect(await screen.findByText('Claude is working')).toBeInTheDocument()
    expect(screen.queryByText('Codex is working')).not.toBeInTheDocument()
  })

  it('stops polling failed changes until the user retries', async () => {
    const user = userEvent.setup()
    daemon.loadConversationDiff.mockRejectedValue(new Error('Git failed'))
    renderDesktopApp()
    await userEvent.setup().click(
      await screen.findByRole('button', { name: /^Review/ }),
    )

    expect(
      await screen.findByRole('heading', { name: "Couldn?t load changes" }),
    ).toBeInTheDocument()
    expect(daemon.loadConversationDiff).toHaveBeenCalledOnce()

    await user.click(screen.getByRole('button', { name: 'Retry' }))

    await waitFor(() => {
      expect(daemon.loadConversationDiff).toHaveBeenCalledTimes(2)
    })
  })

  it('explains when Changes is unavailable because the project has no Git repository', async () => {
    
    daemon.loadConversations.mockResolvedValue([
      { ...structuredConversation, isGitRepository: false },
    ])
    renderDesktopApp()
    await userEvent.setup().click(
      await screen.findByRole('button', { name: /^Review/ }),
    )

    expect(
      await screen.findByRole('heading', { name: 'Changes need Git' }),
    ).toBeInTheDocument()
    expect(daemon.loadConversationDiff).not.toHaveBeenCalled()
    expect(
      screen.getByText(/Initialize Git here to view file changes/),
    ).toBeInTheDocument()
  })

  it('opens the read-only file browser on demand and previews a daemon file', async () => {
    const user = userEvent.setup()
    renderDesktopApp()

    expect(screen.queryByRole('tab', { name: 'Files' })).not.toBeInTheDocument()
    await user.click(await screen.findByRole('button', { name: /^Review/ }))
    await user.click(await screen.findByRole('tab', { name: 'Files' }))
    await user.click(await screen.findByRole('button', { name: 'Preview file README.md' }))

    expect(await screen.findByText('# Read only preview')).toBeInTheDocument()
    expect(screen.getByText(/Read-only preview/)).toBeInTheDocument()
    expect(daemon.loadConversationFile).toHaveBeenCalledWith(expect.objectContaining({ id: '42', generation: 7 }), 'README.md')
    expect(screen.queryByRole('textbox', { name: /file/i })).not.toBeInTheDocument()
  })

  it('recovers an empty workspace after the daemon reconnects', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockReset()
    daemon.loadConversations
      .mockRejectedValueOnce(
        'connecting to the local daemon: the running server is another maitty version (protocol v49, this one v59); run maitty in a terminal to restart it, or `maitty kill-server` and run maitty again',
      )
      .mockResolvedValue([])
    renderDesktopApp()

    expect(
      await screen.findByRole('heading', { name: 'Daemon disconnected' }),
    ).toBeInTheDocument()
    expect(
      screen.getByText(/service uses protocol v49, but this GUI needs v59/),
    ).toBeInTheDocument()
    expect(
      screen.getByText(/leave the service running while it owns sessions/),
    ).toBeInTheDocument()
    await new Promise<void>((resolve) => {
      window.setTimeout(resolve, 1600)
    })
    expect(daemon.loadConversations).toHaveBeenCalledOnce()
    await user.click(screen.getByRole('button', { name: 'Open settings' }))
    expect(screen.getByRole('dialog')).toHaveTextContent('Settings')
    await user.click(screen.getByRole('button', { name: 'Done' }))
    await user.click(screen.getByRole('button', { name: 'Reconnect' }))

    expect(
      await screen.findByRole('heading', {
        name: 'Run your coding agents side by side.',
      }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('status', { name: 'Daemon connected' }),
    ).toBeInTheDocument()
    expect(daemon.loadConversations).toHaveBeenCalledTimes(2)
  })

  it('explains how to start when the workspace has no conversations', async () => {
    daemon.loadConversations.mockResolvedValue([])
    renderDesktopApp()

    expect(
      await screen.findByText(
        'No conversations yet. Open a project folder to get started.',
      ),
    ).toHaveAttribute('role', 'status')
  })

  it('does not offer to restart an old daemon while it owns TUI sessions', async () => {
    daemon.loadConversations
      .mockRejectedValueOnce(
        'connecting to the local daemon: the running server is another maitty version (protocol v49, this one v59); run maitty in a terminal to restart it, or `maitty kill-server` and run maitty again',
      )
      .mockResolvedValue([])
    daemon.restartMismatchedDaemon.mockResolvedValue(undefined)
    daemon.loadDaemonRestartStatus.mockResolvedValue({
      daemonProtocol: 49,
      guiProtocol: 62,
      canRestart: false,
      sessionCount: 2,
      activeTurnCount: 1,
    })
    renderDesktopApp()

    expect(
      await screen.findByRole('heading', { name: 'Daemon disconnected' }),
    ).toBeInTheDocument()
    expect(
      await screen.findByRole('button', { name: 'Service in use' }),
    ).toBeDisabled()
    expect(
      screen.getByText(/owns 2 sessions\. Maitty will leave it running/),
    ).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
    expect(daemon.restartMismatchedDaemon).not.toHaveBeenCalled()
  })

  it('offers restart for the current local protocol mismatch message', async () => {
    daemon.loadConversations.mockRejectedValueOnce(
      'connecting to the local daemon: the running maitty service uses protocol v49, while this GUI needs v62',
    )
    daemon.loadDaemonRestartStatus.mockResolvedValue({
      daemonProtocol: 49,
      guiProtocol: 62,
      canRestart: true,
      sessionCount: 0,
      activeTurnCount: 0,
    })
    renderDesktopApp()

    await screen.findByRole('heading', { name: 'Daemon disconnected' })
    expect(
      await screen.findByRole('button', { name: 'Restart service' }),
    ).toBeInTheDocument()
  })

  it('starts and selects a conversation in the folder chosen from the GUI', async () => {
    const user = userEvent.setup()
    const folder = '/home/casey/projects/new-project'
    const createdConversation: Conversation = {
      ...structuredConversation,
      id: '43',
      generation: 1,
      title: 'Codex conversation 43',
      project: 'new-project',
      folder,
    }
    folderPicker.chooseProjectFolder.mockResolvedValue(folder)
    daemon.createConversation.mockResolvedValue({ id: 43, generation: 1 })
    daemon.submitMessage.mockResolvedValue(undefined)
    daemon.loadConversations.mockResolvedValue([
      structuredConversation,
      createdConversation,
    ])
    daemon.loadMessages.mockImplementation((id: string) =>
      Promise.resolve({
        generation: id === '43' ? 1 : 7,
        receipt: null,
        messages: [],
        nextOperation: 1,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      }),
    )
    renderDesktopApp()

    await user.keyboard('{Control>}n{/Control}')
    await user.click(
      screen.getByRole('button', { name: 'Browse for project folder' }),
    )
    expect(
      await screen.findByRole('textbox', { name: 'Project folder' }),
    ).toHaveValue(folder)
    await user.click(screen.getByRole('button', { name: 'Start conversation' }))

    await waitFor(() => {
      expect(daemon.createConversation).toHaveBeenCalledWith(folder, 'Codex')
    })
    expect(
      await screen.findByRole('heading', { name: 'Codex conversation 43' }),
    ).toBeInTheDocument()
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()

    const composer = screen.getByRole('textbox', { name: 'Message Codex' })
    await user.type(composer, 'Start the first Codex turn')
    await user.click(screen.getByRole('button', { name: 'Send message' }))

    await waitFor(() => {
      expect(daemon.submitMessage).toHaveBeenCalledWith(
        expect.objectContaining({ id: '43', generation: 1 }),
        1,
        'Start the first Codex turn',
      )
    })
    expect(composer).toHaveValue('')
  })

  it('creates a Claude conversation using the selected provider', async () => {
    const user = userEvent.setup()
    const folder = '/home/casey/projects/claude-project'
    const createdConversation: Conversation = {
      ...structuredConversation,
      id: '44',
      generation: 1,
      title: 'Claude conversation 44',
      project: 'claude-project',
      folder,
      provider: 'Claude',
    }
    folderPicker.chooseProjectFolder.mockResolvedValue(folder)
    daemon.createConversation.mockResolvedValue({ id: 44, generation: 1 })
    daemon.loadConversations.mockResolvedValue([
      structuredConversation,
      createdConversation,
    ])
    daemon.loadMessages.mockImplementation((id: string) =>
      Promise.resolve({
        generation: id === '44' ? 1 : 7,
        receipt: null,
        messages: [],
        nextOperation: 1,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      }),
    )
    renderDesktopApp()

    await user.keyboard('{Control>}n{/Control}')
    await user.click(screen.getByRole('button', { name: 'Browse for project folder' }))
    await screen.findByRole('textbox', { name: 'Project folder' })
    await user.click(screen.getByRole('button', { name: /Claude Code CLI and saved sign-in/ }))
    await user.click(screen.getByRole('button', { name: 'Start conversation' }))

    await waitFor(() => {
      expect(daemon.createConversation).toHaveBeenCalledWith(folder, 'Claude')
    })
    expect(await screen.findByRole('heading', { name: 'Claude conversation 44' })).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message Claude' })).toBeInTheDocument()
  })

  it('blocks duplicate conversation creation while the first start is pending', async () => {
    const user = userEvent.setup()
    const folder = '/home/casey/projects/new-project'
    const createdConversation: Conversation = {
      ...structuredConversation,
      id: '43',
      generation: 1,
      title: 'Codex conversation 43',
      project: 'new-project',
      folder,
    }
    let resolveCreation: (value: { id: number; generation: number }) => void =
      () => {}
    folderPicker.chooseProjectFolder.mockResolvedValue(folder)
    daemon.createConversation.mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveCreation = resolve
        }),
    )
    daemon.loadConversations.mockResolvedValue([
      structuredConversation,
      createdConversation,
    ])
    daemon.loadMessages.mockImplementation((id: string) =>
      Promise.resolve({
        generation: id === '43' ? 1 : 7,
        receipt: null,
        messages: [],
        nextOperation: 1,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      }),
    )
    renderDesktopApp()

    await user.keyboard('{Control>}n{/Control}')
    await user.click(
      screen.getByRole('button', { name: 'Browse for project folder' }),
    )
    await screen.findByRole('textbox', { name: 'Project folder' })
    const start = screen.getByRole('button', { name: 'Start conversation' })
    await user.click(start)

    await waitFor(() => {
      expect(daemon.createConversation).toHaveBeenCalledTimes(1)
    })
    expect(start).toBeDisabled()
    await user.click(start)
    expect(daemon.createConversation).toHaveBeenCalledTimes(1)

    resolveCreation({ id: 43, generation: 1 })
    expect(
      await screen.findByRole('heading', { name: 'Codex conversation 43' }),
    ).toBeInTheDocument()
  })

  it('keeps the current conversation open when starting a new one fails', async () => {
    const user = userEvent.setup()
    folderPicker.chooseProjectFolder.mockResolvedValue(
      '/home/casey/projects/new-project',
    )
    daemon.createConversation.mockRejectedValue(
      new Error('Codex was not found'),
    )
    renderDesktopApp()

    await user.keyboard('{Control>}n{/Control}')
    await user.click(
      screen.getByRole('button', { name: 'Browse for project folder' }),
    )
    await user.click(
      await screen.findByRole('button', { name: 'Start conversation' }),
    )

    expect(await screen.findByText('Codex was not found')).toBeInTheDocument()
    expect(screen.getByRole('dialog')).toHaveTextContent('New conversation')
    await user.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(
      screen.getByRole('heading', { name: 'Example conversation' }),
    ).toBeInTheDocument()
    expect(
      screen.getByRole('textbox', { name: 'Message Codex' }),
    ).toBeInTheDocument()
  })

  it('renders assistant code blocks and tool output as distinct code surfaces', async () => {
    const user = userEvent.setup()
    daemon.loadMessages.mockResolvedValue({
      generation: 7,
      receipt: null,
      messages: [
        {
          id: 'assistant-code',
          role: 'assistant',
          content: 'Call `fmt.Println` like this:\n```go\nfmt.Println("GUI_OK")\n```',
          time: 'Now',
        },
        {
          id: 'tool-output',
          role: 'activity',
          content: 'Run command',
          detail: 'GUI_TOOL_OK',
          activityState: 'Completed',
          time: 'Now',
        },
      ],
      nextOperation: 2,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    renderDesktopApp()

    const code = await screen.findByText('fmt.Println("GUI_OK")')
    expect(code.closest('pre')).toBeInTheDocument()
    expect(screen.getByText('go')).toBeInTheDocument()
    expect(screen.getByText('fmt.Println').tagName).toBe('CODE')

    await user.click(screen.getByRole('button', { name: /Codex · Run command · Completed/ }))
    expect(screen.getByText('GUI_TOOL_OK').closest('pre')).toBeInTheDocument()
  })

  it('re-enables the composer after the daemon marks a stopped turn interrupted', async () => {
    const user = userEvent.setup()
    const workingConversation = {
      ...structuredConversation,
      status: 'working' as const,
    }
    const idleConversation = {
      ...structuredConversation,
      status: 'idle' as const,
    }
    daemon.cancelTurn.mockResolvedValue(undefined)
    daemon.loadConversations
      .mockResolvedValueOnce([workingConversation])
      .mockResolvedValue([idleConversation])
    daemon.loadMessages
      .mockResolvedValueOnce({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: { Acknowledged: { turn_id: 'turn-1' } },
        },
        messages: [],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
      .mockResolvedValue({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: { Acknowledged: { turn_id: 'turn-1' } },
        },
        messages: [
          {
            id: 'turn-1:command',
            role: 'activity',
            content: 'Run command',
            detail: '',
            activityState: 'Interrupted',
            time: 'Recent',
          },
        ],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
    renderDesktopApp()

    await user.click(await screen.findByRole('button', { name: 'Stop' }))

    await waitFor(() => {
      expect(daemon.cancelTurn).toHaveBeenCalledWith(workingConversation)
    })
    expect(await screen.findByText('Interrupted')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Message Codex' })).toBeEnabled()
  })

  it('refreshes a running activity after the conversation becomes idle', async () => {
    const runningActivity: Message = {
      id: 'turn-1:command',
      role: 'activity',
      content: 'Run command',
      detail: '',
      activityState: 'Running',
      time: 'Recent',
    }
    const interruptedActivity = {
      ...runningActivity,
      activityState: 'Interrupted' as const,
    }
    daemon.loadMessages
      .mockResolvedValueOnce({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: { Acknowledged: { turn_id: 'turn-1' } },
        },
        messages: [runningActivity],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
      .mockResolvedValue({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: { Acknowledged: { turn_id: 'turn-1' } },
        },
        messages: [interruptedActivity],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
    renderDesktopApp()

    expect(await screen.findByText('Running')).toBeInTheDocument()
    expect(
      await screen.findByText('Interrupted', {}, { timeout: 4000 }),
    ).toBeInTheDocument()
    expect(daemon.loadMessages).toHaveBeenCalledTimes(2)
  })

  it('reloads final conversation history when a streamed turn finishes', async () => {
    const workingConversation = {
      ...structuredConversation,
      provider: 'Claude' as const,
      status: 'working' as const,
    }
    const completedConversation = {
      ...workingConversation,
      status: 'done' as const,
    }
    const history = (content: string) => ({
      generation: 7,
      receipt: null,
      messages: [
        {
          id: 'turn-1:assistant',
          role: 'assistant' as const,
          content,
          time: 'Recent',
        },
      ],
      nextOperation: 1,
      error: null,
      cancelling: false,
      completeness: 'Complete' as const,
    })
    daemon.loadConversations
      .mockResolvedValueOnce([workingConversation])
      .mockResolvedValue([completedConversation])
    daemon.loadMessages
      .mockResolvedValueOnce(history('GUI_CLAU'))
      .mockResolvedValue(history('GUI_CLAUDE_LIVE_OK'))
    renderDesktopApp()

    expect(await screen.findByText('GUI_CLAU')).toBeInTheDocument()
    expect(
      await screen.findByText('GUI_CLAUDE_LIVE_OK', {}, { timeout: 5000 }),
    ).toBeInTheDocument()
    expect(daemon.loadMessages).toHaveBeenCalledTimes(2)
  })

  it('groups consecutive tool steps under the active provider and expands their output', async () => {
    const user = userEvent.setup()
    daemon.loadConversations.mockResolvedValue([
      { ...structuredConversation, provider: 'Claude' },
    ])
    daemon.loadMessages.mockResolvedValue({
      generation: 7,
      receipt: null,
      messages: [
        {
          id: 'turn-1:search',
          role: 'activity',
          content: 'Search files',
          detail: 'Found src/App.tsx',
          activityState: 'Completed',
          time: 'Recent',
        },
        {
          id: 'turn-1:command',
          role: 'activity',
          content: 'Run tests',
          detail: 'Tests passed',
          activityState: 'Completed',
          time: 'Recent',
        },
        {
          id: 'turn-1:reply',
          role: 'assistant',
          content: 'The tests pass.',
          time: 'Recent',
        },
      ],
      nextOperation: 1,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    renderDesktopApp()

    const group = await screen.findByRole('button', {
      name: 'Claude · 2 actions completed · Completed',
    })
    expect(screen.getByRole('region', { name: 'Claude tool activity' })).toBeInTheDocument()
    await user.click(group)

    expect(screen.getByText('Search files')).toBeInTheDocument()
    expect(screen.getByText('Found src/App.tsx')).toBeInTheDocument()
    expect(screen.getByText('Run tests')).toBeInTheDocument()
    expect(screen.getByText('Tests passed')).toBeInTheDocument()
  })

  it('only runs an approval shortcut when Codex offered that exact choice', async () => {
    const user = userEvent.setup()
    const approval: Message = {
      id: 'approval-1',
      role: 'approval',
      content: 'Codex requests approval',
      time: 'Needs your decision',
      providerRequestId: 'request-1',
      approvalKind: 'Command',
      approvalChoices: ['Decline'],
    }
    daemon.loadConversations.mockResolvedValue([
      { ...structuredConversation, status: 'needs-you' },
    ])
    daemon.loadMessages.mockResolvedValue({
      generation: 7,
      receipt: null,
      messages: [approval],
      nextOperation: 1,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    daemon.approveRequest.mockResolvedValue(undefined)
    renderDesktopApp()

    expect(
      await screen.findByRole('button', { name: 'Decline' }),
    ).toBeInTheDocument()
    await user.keyboard('y')
    expect(daemon.approveRequest).not.toHaveBeenCalled()

    await user.keyboard('n')
    await waitFor(() => {
      expect(daemon.approveRequest).toHaveBeenCalledWith(
        expect.objectContaining({ id: '42' }),
        'request-1',
        'Decline',
      )
    })
  })

  it('passes Ctrl+backslash through while the native terminal has focus', async () => {
    daemon.loadConversations.mockResolvedValue([
      { ...structuredConversation, execution: 'Native', nativePane: 43 },
    ])
    renderDesktopApp()
    const terminal = await screen.findByRole('region', {
      name: 'Native Codex terminal',
    })
    const sidebar = screen.getByRole('complementary', { name: 'Conversations' })

    fireEvent.keyDown(terminal, { key: '\\', ctrlKey: true })

    expect(sidebar).not.toHaveClass('is-desktop-collapsed')
  })

  it('resumes the exact structured session from the recovery notice', async () => {
    const user = userEvent.setup()
    const recoveryConversation = {
      ...structuredConversation,
      status: 'recovery' as const,
    }
    daemon.loadConversations.mockResolvedValue([recoveryConversation])
    daemon.resumeConversation.mockResolvedValue({ id: 42, generation: 8 })
    renderDesktopApp()

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Maitty will not replay the interrupted turn',
    )
    await user.click(screen.getByRole('button', { name: 'Resume session' }))

    await waitFor(() => {
      expect(daemon.resumeConversation).toHaveBeenCalledWith(
        recoveryConversation,
      )
    })
    expect(
      await screen.findByRole('button', { name: 'Resume session' }),
    ).toBeEnabled()
  })

  it('keeps a failed draft and offers retry when the daemon confirms no delivery', async () => {
    const user = userEvent.setup()
    daemon.submitMessage.mockRejectedValue(new Error('send failed'))
    daemon.classifySubmissionFailure.mockReturnValue('not-delivered')
    renderDesktopApp()

    const composer = await screen.findByRole('textbox', {
      name: 'Message Codex',
    })
    await user.type(composer, 'Please inspect this change')
    await user.click(screen.getByRole('button', { name: 'Send message' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Message was not delivered',
    )
    expect(composer).toHaveValue('Please inspect this change')
    expect(screen.getByRole('button', { name: 'Retry' })).toBeEnabled()
  })

  it('keeps uncertain delivery blocked and preserves the draft', async () => {
    const user = userEvent.setup()
    daemon.submitMessage.mockRejectedValue(new Error('send failed'))
    daemon.classifySubmissionFailure.mockReturnValue('unknown')
    daemon.loadMessages.mockResolvedValueOnce({
      generation: 7,
      receipt: null,
      messages: [],
      nextOperation: 1,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    daemon.loadMessages.mockResolvedValue({
      generation: 7,
      receipt: {
        conversation: 42,
        generation: 7,
        operation: 1,
        delivery: 'SubmissionUncertain',
      },
      messages: [],
      nextOperation: 2,
      error: null,
      cancelling: false,
      completeness: 'Complete',
    })
    renderDesktopApp()

    const composer = await screen.findByRole('textbox', {
      name: 'Message Codex',
    })
    await user.type(composer, 'Do not duplicate this prompt')
    await user.click(screen.getByRole('button', { name: 'Send message' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Delivery is unknown',
    )
    expect(composer).toHaveValue('Do not duplicate this prompt')
    expect(composer).toBeDisabled()
    expect(
      screen.queryByRole('button', { name: 'Retry' }),
    ).not.toBeInTheDocument()
  })

  it('clears a blocked draft only after a delivery recheck confirms acknowledgement', async () => {
    const user = userEvent.setup()
    window.localStorage.setItem(
      'maitty.gui.drafts.v1',
      JSON.stringify({ '42': 'Prompt already submitted' }),
    )
    daemon.classifySubmissionFailure.mockReturnValue('delivered')
    daemon.loadMessages
      .mockResolvedValueOnce({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: 'SubmissionUncertain',
        },
        messages: [],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
      .mockResolvedValue({
        generation: 7,
        receipt: {
          conversation: 42,
          generation: 7,
          operation: 1,
          delivery: { Acknowledged: { turn_id: 'turn-1' } },
        },
        messages: [],
        nextOperation: 2,
        error: null,
        cancelling: false,
        completeness: 'Complete',
      })
    renderDesktopApp()

    const composer = await screen.findByRole('textbox', {
      name: 'Message Codex',
    })
    expect(composer).toHaveValue('Prompt already submitted')
    await user.click(
      await screen.findByRole('button', { name: 'Check delivery' }),
    )

    await waitFor(() => {
      expect(composer).toHaveValue('')
    })
    expect(screen.getByRole('button', { name: 'Send message' })).toBeDisabled()
    expect(
      screen.queryByText(
        'Delivery is unknown. Maitty will not retry this prompt. Check the native terminal before continuing.',
      ),
    ).not.toBeInTheDocument()
  })
})

function renderDesktopApp(conversations?: Conversation[]) {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  })
  if (conversations) client.setQueryData(['conversations'], conversations)
  return render(
    <QueryClientProvider client={client}>
      <App />
    </QueryClientProvider>,
  )
}
