import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
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
      <InspectPanel term={5} project="maitty" branch="main" changedFiles={2} view={view} agentWorking={extra.agentWorking ?? false} timeline={[]} lookAt={null} onView={vi.fn()} onClose={vi.fn()} onNotice={onNotice} />
    </QueryClientProvider>,
  )
  return { onNotice }
}

beforeEach(() => { vi.mocked(isTauri).mockReturnValue(true) })

describe('Inspect panel', () => {
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

    expect(await screen.findByText('a.txt')).toBeTruthy()
    expect(screen.getByLabelText('Diff').textContent).toContain('+new')
    fireEvent.click(screen.getByRole('button', { name: /b\.txt/ }))
    expect(screen.getByLabelText('Diff').textContent).toContain('+z')
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

  it('opens a folder and previews a file in it', async () => {
    daemonAnswers({
      Tree: [{ name: 'src', is_directory: true }, { name: 'README.md', is_directory: false }],
      File: 'hello\nworld\n',
    })
    renderPanel('files')

    fireEvent.click(await screen.findByRole('button', { name: 'README.md' }))
    expect(await screen.findByLabelText('Preview of README.md')).toBeTruthy()
    await waitFor(() => { expect(screen.getByLabelText('Preview of README.md').textContent).toContain('world') })
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
})
