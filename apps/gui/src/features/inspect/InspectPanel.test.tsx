import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { InspectPanel, type InspectView } from './InspectPanel'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }))

const NOW = 1_800_000_000

type Question = string | Record<string, unknown>

/** Answers each question the panel asks, the way the daemon would. */
function daemonAnswers(replies: Record<string, unknown>) {
  vi.mocked(invoke).mockReset().mockImplementation((command, args) => {
    if (command === 'restore_snapshot') return Promise.resolve(2)
    const question = (args as { question: Question }).question
    const kind = typeof question === 'string' ? question : (Object.keys(question)[0] ?? '')
    if (kind in replies) return Promise.resolve({ [kind === 'DiffRange' ? 'Diff' : kind === 'Locate' ? 'Located' : kind]: replies[kind] })
    return Promise.reject(new Error(`unexpected ${kind}`))
  })
}

function renderPanel(view: InspectView, extra: { agentWorking?: boolean } = {}) {
  const onNotice = vi.fn()
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <InspectPanel term={5} terminals={null} changedFiles={2} view={view} agentWorking={extra.agentWorking ?? false} timeline={[]} lookAt={null} onView={vi.fn()} onClose={vi.fn()} onNotice={onNotice} />
    </QueryClientProvider>,
  )
  return { onNotice }
}

beforeEach(() => { vi.mocked(isTauri).mockReturnValue(true) })

describe('Inspect panel', () => {
  it('offers every view, and shows the session’s terminals on that tab', () => {
    daemonAnswers({})
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <InspectPanel term={5} terminals={<p>its terminals</p>} changedFiles={0} view="terminal" agentWorking={false} timeline={[]} lookAt={null} onView={vi.fn()} onClose={vi.fn()} onNotice={vi.fn()} />
      </QueryClientProvider>,
    )
    expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toEqual(['Changes', 'Files', 'Terminal', 'Checks', 'Branches', 'History', 'Timeline', 'References'])
    expect(screen.getByText('its terminals')).toBeTruthy()
    cleanup()
  })

  it('shows only the Terminal tab for a session with no repository to ask about', () => {
    daemonAnswers({})
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <InspectPanel term={null} terminals={<p>its terminals</p>} changedFiles={0} view="changes" agentWorking={false} timeline={[]} lookAt={null} onView={vi.fn()} onClose={vi.fn()} onNotice={vi.fn()} />
      </QueryClientProvider>,
    )
    expect(screen.getAllByRole('tab').map((tab) => tab.textContent)).toEqual(['Terminal'])
    expect(screen.getByText('its terminals')).toBeTruthy()
    cleanup()
  })

  it('lists the changed files and shows the picked file’s diff', async () => {
    daemonAnswers({
      Diff: {
        files: [
          { path: 'a.txt', additions: 1, deletions: 1, binary: false, patch: 'diff --git a/a.txt b/a.txt\n@@ -1 +1 @@\n-old\n+new\n' },
          { path: 'b.txt', additions: 3, deletions: 0, binary: false, patch: '@@ -0,0 +1,3 @@\n+x\n+y\n+z\n' },
        ],
        additions: 4, deletions: 1, total_files: 2, truncated: false,
      },
    })
    renderPanel('changes')

    const files = await screen.findByRole('list', { name: 'Changed files' })
    expect(within(files).getByText('a.txt')).toBeTruthy()
    expect(screen.getByLabelText('Diff').textContent).toContain('+new')
    expect(screen.getByLabelText('Diff').textContent).not.toContain('diff --git')
    fireEvent.click(within(files).getByRole('button', { name: /b\.txt/ }))
    expect(screen.getByLabelText('Diff').textContent).toContain('+z')
  })

  it('opens a comment box on a clicked diff line, sends the comment to the agent and keeps it under the line', async () => {
    daemonAnswers({
      Diff: { files: [{ path: 'src/a.ts', additions: 1, deletions: 1, binary: false, patch: '@@ -3 +3 @@\n-old\n+new\n' }], additions: 1, deletions: 1, total_files: 1, truncated: false },
    })
    const onSend = vi.fn(() => Promise.resolve())
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <InspectPanel term={5} terminals={null} changedFiles={1} view="changes" agentWorking={false} timeline={[]} lookAt={null} comments={{ agent: 'Claude', onSend }} onView={vi.fn()} onClose={vi.fn()} onNotice={vi.fn()} />
      </QueryClientProvider>,
    )
    fireEvent.click(await screen.findByText(/\+new/))
    const send = screen.getByRole('button', { name: 'Send to Claude' })
    expect(send).toBeDisabled()
    fireEvent.change(screen.getByLabelText('Comment on line 3'), { target: { value: 'call it fresh' } })
    fireEvent.click(send)
    await waitFor(() => { expect(onSend).toHaveBeenCalledWith('About src/a.ts, line 3:\n> new\n\ncall it fresh') })
    expect(await screen.findByText(/sent to Claude/)).toBeTruthy()
    expect(screen.getByText('1 comment')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Send to Claude' })).toBeNull()
    cleanup()
  })

  it('commits the changed files with the suggested message, then opens a pull request and shows where it is', async () => {
    const asked: Array<[string, unknown]> = []
    vi.mocked(invoke).mockReset().mockImplementation((command, args) => {
      asked.push([command, args])
      if (command === 'git_commit_all') return Promise.resolve('abc1234')
      if (command === 'git_create_pull_request') return Promise.resolve('https://github.com/o/r/pull/7')
      return Promise.resolve({ Diff: { files: [{ path: 'a.txt', additions: 1, deletions: 0, binary: false, patch: '@@ -0,0 +1 @@\n+new\n' }], additions: 1, deletions: 0, total_files: 1, truncated: false } })
    })
    const onNotice = vi.fn()
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <InspectPanel term={5} terminals={null} changedFiles={1} view="changes" agentWorking={false} timeline={[]} lookAt={null} changeActions={{ folder: '/home/me/app', machine: 0, suggested: 'Add refunds' }} onView={vi.fn()} onClose={vi.fn()} onNotice={onNotice} />
      </QueryClientProvider>,
    )
    expect(await screen.findByLabelText('Commit message')).toHaveValue('Add refunds')
    fireEvent.click(screen.getByRole('button', { name: 'Commit 1 file' }))
    await waitFor(() => { expect(onNotice).toHaveBeenCalledWith('Committed 1 file as abc1234') })
    expect(asked).toContainEqual(['git_commit_all', { machine: 0, folder: '/home/me/app', message: 'Add refunds', paths: null }])
    fireEvent.click(screen.getByLabelText('Commit a.txt'))
    expect(screen.getByRole('button', { name: 'Commit 0 files' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Create PR' }))
    expect(await screen.findByText('https://github.com/o/r/pull/7')).toBeTruthy()
    expect(asked).toContainEqual(['git_create_pull_request', { machine: 0, folder: '/home/me/app' }])
    cleanup()
  })

  it('says what git said when a commit fails, and offers neither button without a checkout on this computer', async () => {
    vi.mocked(invoke).mockReset().mockImplementation((command) => {
      if (command === 'git_commit_all') return Promise.reject(new Error('nothing to commit'))
      return Promise.resolve({ Diff: { files: [{ path: 'a.txt', additions: 1, deletions: 0, binary: false, patch: '@@ -0,0 +1 @@\n+new\n' }], additions: 1, deletions: 0, total_files: 1, truncated: false } })
    })
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <InspectPanel term={5} terminals={null} changedFiles={1} view="changes" agentWorking={false} timeline={[]} lookAt={null} changeActions={{ folder: '/home/me/app', machine: 0, suggested: 'x' }} onView={vi.fn()} onClose={vi.fn()} onNotice={vi.fn()} />
      </QueryClientProvider>,
    )
    fireEvent.click(await screen.findByRole('button', { name: 'Commit 1 file' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('nothing to commit')
    cleanup()
    renderPanel('changes')
    await screen.findByRole('list', { name: 'Changed files' })
    expect(screen.queryByRole('button', { name: /^Commit/ })).toBeNull()
    expect(screen.queryByRole('button', { name: 'Create PR' })).toBeNull()
  })

  it('offers no comment box without an agent to tell', async () => {
    daemonAnswers({
      Diff: { files: [{ path: 'a.txt', additions: 1, deletions: 0, binary: false, patch: '@@ -0,0 +1 @@\n+new\n' }], additions: 1, deletions: 0, total_files: 1, truncated: false },
    })
    renderPanel('changes')
    fireEvent.click(await screen.findByText(/\+new/))
    expect(screen.queryByRole('textbox')).toBeNull()
  })

  it('says so when the working tree is clean', async () => {
    daemonAnswers({ Diff: { files: [], additions: 0, deletions: 0, total_files: 0, truncated: false } })
    renderPanel('changes')
    expect(await screen.findByText(/No changes/)).toBeTruthy()
  })

  it('shows a daemon error instead of an empty panel', async () => {
    vi.mocked(invoke).mockReset().mockRejectedValue('Not inside a Git repository')
    renderPanel('changes')
    expect(await screen.findByRole('alert')).toBeTruthy()
  })

  it('shows what the daemon said when it fails with a plain string, not a generic message', async () => {
    vi.mocked(invoke).mockReset().mockRejectedValue('git diff: fatal: bad object HEAD')
    renderPanel('changes')
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Maitty couldn’t load your changes')
    // The daemon's own words are kept for support, behind one link, and not in the headline.
    expect(alert).not.toHaveTextContent('bad object HEAD')
    fireEvent.click(screen.getByRole('button', { name: 'Show details for support' }))
    expect(screen.getByRole('alert')).toHaveTextContent('git diff: fatal: bad object HEAD')
  })

  it('says a busy git is not an error and tries again by itself', async () => {
    vi.mocked(invoke).mockReset().mockRejectedValue("fatal: Unable to create '/code/app/.git/index.lock': File exists. Another git process seems to be running")
    renderPanel('changes')
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Git is finishing something')
    expect(alert).toHaveTextContent('Nothing in your project was changed')
    expect(alert).toHaveTextContent('Trying again automatically in 3s')
    expect(screen.getByRole('button', { name: 'Try again' })).toBeTruthy()
  })

  it('asks for permission rather than retrying when a folder cannot be read', async () => {
    vi.mocked(invoke).mockReset().mockRejectedValue('read /code/app: Permission denied (os error 13)')
    renderPanel('files')
    const alert = await screen.findByRole('alert')
    expect(alert).toHaveTextContent('Maitty can’t read this folder')
    expect(alert).not.toHaveTextContent('Trying again automatically')
  })

  it.each(['files', 'changes', 'branches', 'history', 'references'] as const)(
    'explains a plain folder on the %s tab instead of calling it an error, and says what to do',
    async (view) => {
      vi.mocked(invoke).mockReset().mockRejectedValue('git rev-parse --show-toplevel: fatal: not a git repository (or any of the parent directories): .git')
      renderPanel(view)
      const note = await screen.findByRole('status')
      expect(note).toHaveTextContent('This folder isn’t a git project yet')
      expect(note).toHaveTextContent('git init')
      expect(screen.queryByRole('alert')).toBeNull()
    },
  )

  it('shows the file tree with a file beside it, opens folders in place and highlights the file', async () => {
    daemonAnswers({
      Tree: [{ name: 'src', is_directory: true }, { name: 'README.md', is_directory: false }],
      File: 'const answer = 42 // the answer\n',
    })
    renderPanel('files')

    const folder = await screen.findByRole('button', { name: 'src' })
    expect(folder).toHaveAttribute('aria-expanded', 'false')
    fireEvent.click(folder)
    expect(folder).toHaveAttribute('aria-expanded', 'true')

    fireEvent.click(await screen.findByRole('button', { name: 'README.md' }))
    expect(await screen.findByLabelText('Preview of README.md')).toBeTruthy()
    await waitFor(() => { expect(screen.getByLabelText('Preview of README.md').textContent).toContain('answer') })
    // The tree stays on screen beside the file, and the file is read-only (no editor).
    expect(screen.getByRole('navigation', { name: 'Files' })).toBeTruthy()
    expect(screen.queryByRole('textbox')).toBeNull()
  })

  it('opens the README by default, or the first file when there is none', async () => {
    daemonAnswers({ Tree: [{ name: 'src', is_directory: true }, { name: 'main.go', is_directory: false }, { name: 'README.md', is_directory: false }], File: '# hello\n' })
    renderPanel('files')
    expect(await screen.findByLabelText('Preview of README.md')).toBeTruthy()
    cleanup()

    daemonAnswers({ Tree: [{ name: 'src', is_directory: true }, { name: 'main.go', is_directory: false }], File: 'package main\n' })
    renderPanel('files')
    expect(await screen.findByLabelText('Preview of main.go')).toBeTruthy()
  })

  it('marks the current branch and its distance from the upstream', async () => {
    daemonAnswers({
      Status: { branch: 'main', ahead: 2, behind: 0, files: [] },
      Branches: [{ name: 'feature', current: false }, { name: 'main', current: true }],
    })
    renderPanel('branches')
    expect(await screen.findByText('↑2')).toBeTruthy()
    expect(screen.getByText('current')).toBeTruthy()
  })

  it('lists reference files, or explains where they go', async () => {
    daemonAnswers({ References: [{ name: 'home.png', kind: 'image', bytes: 2048, modified: NOW }] })
    renderPanel('references')
    expect(await screen.findByText('home.png')).toBeTruthy()
    expect(screen.getByText(/image · 2 KB/)).toBeTruthy()
  })
})

describe('Inspect history', () => {
  const history = {
    Log: [{ hash: 'abcdef0123456789', subject: 'first commit', author: 'Test', time: NOW - 100 }],
    Snapshots: [{ commit: 'feed1234', at: NOW - 10, why: "before claude's turn" }],
    DiffRange: { files: [{ path: 'a.txt', additions: 1, deletions: 0, binary: false, patch: '@@ -0,0 +1 @@\n+hello\n' }], additions: 1, deletions: 0, total_files: 1, truncated: false },
  }

  it('asks before restoring a snapshot, then restores it and says how many files changed', async () => {
    daemonAnswers(history)
    const { onNotice } = renderPanel('history')

    fireEvent.click(await screen.findByRole('button', { name: 'Restore' }))
    expect(screen.getByRole('group', { name: 'Confirm restore' }).textContent).toContain('What is there now is saved first')
    expect(vi.mocked(invoke)).not.toHaveBeenCalledWith('restore_snapshot', expect.anything())

    fireEvent.click(screen.getByRole('button', { name: 'Restore files' }))
    await waitFor(() => { expect(onNotice).toHaveBeenCalledWith(expect.stringContaining('Restored 2 files')) })
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('restore_snapshot', { term: 5, commit: 'feed1234' })
  })

  it('will not restore while the agent is working', async () => {
    daemonAnswers(history)
    renderPanel('history', { agentWorking: true })
    expect((await screen.findByRole('button', { name: 'Restore' })).hasAttribute('disabled')).toBe(true)
  })

  it('compares two picked entries and shows what differs', async () => {
    daemonAnswers(history)
    renderPanel('history')

    fireEvent.click(await screen.findByLabelText(/Compare/))
    fireEvent.click(screen.getByRole('button', { name: /first commit/ }))
    fireEvent.click(screen.getByRole('button', { name: /before claude's turn/ }))

    expect(await screen.findByLabelText('Comparison')).toBeTruthy()
    await waitFor(() => { expect(screen.getByLabelText('Diff').textContent).toContain('+hello') })
    expect(vi.mocked(invoke)).toHaveBeenCalledWith('repo_query', { term: 5, question: { DiffRange: { from: 'abcdef0123456789', to: 'feed1234', path: null } } })
  })

  describe('a turn’s changes', () => {
    const diffOf = (path: string) => ({ files: [{ path, additions: 1, deletions: 0, binary: false, patch: '@@ -0,0 +1 @@\n+new\n' }], additions: 1, deletions: 0, total_files: 1, truncated: false })
    type Turn = { startedAtMs: number; endedAtMs: number } | null
    const turnPanel = (turn: Turn, onAllChanges = vi.fn()) => {
      const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
      const panel = (shown: Turn) => (
        <QueryClientProvider client={client}>
          <InspectPanel term={5} terminals={null} changedFiles={2} view="changes" agentWorking={false} timeline={[]} lookAt={null} changeActions={{ folder: '/home/me/app', machine: 0, suggested: 'x' }} turnScope={{ turn: shown, onAllChanges }} onView={vi.fn()} onClose={vi.fn()} onNotice={vi.fn()} />
        </QueryClientProvider>
      )
      const { rerender } = render(panel(turn))
      return { onAllChanges, showTurn: (next: Turn) => { rerender(panel(next)) } }
    }

    it('shows what the turn changed between its snapshots, with no commit, and goes back to all changes', async () => {
      daemonAnswers({
        Snapshots: [{ commit: 'after', at: NOW + 60, why: 'after claude’s turn' }, { commit: 'before', at: NOW, why: 'before claude’s turn' }],
        DiffRange: diffOf('turn.ts'),
        Diff: diffOf('everything.ts'),
      })
      const { onAllChanges } = turnPanel({ startedAtMs: NOW * 1000, endedAtMs: (NOW + 59) * 1000 })
      expect(within(await screen.findByRole('list', { name: 'Changed files' })).getByText('turn.ts')).toBeTruthy()
      expect(screen.getByText("This turn's changes")).toBeTruthy()
      expect(vi.mocked(invoke)).toHaveBeenCalledWith('repo_query', { term: 5, question: { DiffRange: { from: 'before', to: 'after', path: null } } })
      expect(screen.queryByRole('button', { name: /^Commit/ })).toBeNull()
      fireEvent.click(screen.getByRole('button', { name: 'All changes' }))
      expect(onAllChanges).toHaveBeenCalled()
    })

    it('shows the whole working tree and says so when the turn has no snapshot before it', async () => {
      daemonAnswers({ Snapshots: [], Diff: diffOf('everything.ts') })
      turnPanel({ startedAtMs: NOW * 1000, endedAtMs: (NOW + 59) * 1000 })
      expect(within(await screen.findByRole('list', { name: 'Changed files' })).getByText('everything.ts')).toBeTruthy()
      expect(screen.getByText(/^Whole working tree/)).toBeTruthy()
      expect(screen.getByRole('button', { name: 'Commit 1 file' })).toBeTruthy()
    })

    it('shows the whole working tree for a turn without the daemon’s times, without asking for snapshots', async () => {
      daemonAnswers({ Diff: diffOf('everything.ts') })
      turnPanel(null)
      expect(within(await screen.findByRole('list', { name: 'Changed files' })).getByText('everything.ts')).toBeTruthy()
      expect(screen.getByText(/^Whole working tree/)).toBeTruthy()
      expect(vi.mocked(invoke)).not.toHaveBeenCalledWith('repo_query', { term: 5, question: 'Snapshots' })
    })

    it('shows the whole working tree and says so when no snapshot from after the turn is kept', async () => {
      daemonAnswers({ Snapshots: [{ commit: 'before', at: NOW, why: 'before claude’s turn' }], Diff: diffOf('everything.ts') })
      turnPanel({ startedAtMs: NOW * 1000, endedAtMs: (NOW + 59) * 1000 })
      expect(within(await screen.findByRole('list', { name: 'Changed files' })).getByText('everything.ts')).toBeTruthy()
      expect(screen.getByText(/^Whole working tree · no snapshot from after this turn is kept/)).toBeTruthy()
    })

    it('reads the snapshots again for a later turn, so its own end snapshot is found', async () => {
      const first = [{ commit: 'after1', at: NOW + 60, why: 'after' }, { commit: 'before1', at: NOW, why: 'before' }]
      daemonAnswers({ Snapshots: first, DiffRange: diffOf('one.ts') })
      const { showTurn } = turnPanel({ startedAtMs: NOW * 1000, endedAtMs: (NOW + 59) * 1000 })
      expect(within(await screen.findByRole('list', { name: 'Changed files' })).getByText('one.ts')).toBeTruthy()
      daemonAnswers({ Snapshots: [{ commit: 'after2', at: NOW + 200, why: 'after' }, ...first], DiffRange: diffOf('two.ts') })
      showTurn({ startedAtMs: (NOW + 100) * 1000, endedAtMs: (NOW + 199) * 1000 })
      await waitFor(() => { expect(within(screen.getByRole('list', { name: 'Changed files' })).getByText('two.ts')).toBeTruthy() })
      await waitFor(() => { expect(vi.mocked(invoke)).toHaveBeenCalledWith('repo_query', { term: 5, question: { DiffRange: { from: 'after1', to: 'after2', path: null } } }) })
    })
  })
})
