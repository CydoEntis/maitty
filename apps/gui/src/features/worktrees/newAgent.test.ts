import { describe, expect, it, vi } from 'vitest'
import type { SessionGroup } from '@/features/workspace/model/session'
import { agentBranchProblem, agentProjects, defaultProject, MAX_PROMPT_CHARS, newAgentPaletteItem, promptProblem, promptSlug, suggestedBranch, worktreeFolder } from './newAgent'

function group(key: string, name: string, kind: SessionGroup['kind'], machineName: string | null = null): SessionGroup {
  return { key, name, kind, path: key, machineName, sessions: [], containsFocused: false, needsCount: 0, workingCount: 0 }
}

describe('the branch a prompt suggests', () => {
  it('takes the first four meaningful words', () => {
    expect(promptSlug('Please fix the login bug in the settings page')).toBe('fix-login-bug-settings')
    expect(suggestedBranch('Fix the login bug')).toBe('agent/fix-login-bug')
  })

  it('has nothing to suggest until the prompt has a word', () => {
    expect(suggestedBranch('')).toBe('')
    expect(suggestedBranch('  ?! ')).toBe('')
  })

  it('keeps filler when it is all there is, and drops what git would refuse', () => {
    expect(promptSlug('make it so')).toBe('make-it-so')
    expect(promptSlug('Add "quotes", [brackets] & café!')).toBe('add-quotes-brackets-caf')
  })

  it('stays short', () => {
    expect(promptSlug('internationalization-and-localization refactoring-of-everything')).toHaveLength(40)
  })

  it('always gives a branch the checks accept', () => {
    for (const prompt of ['Fix the login bug', 'refactor: split app.tsx', '../../etc/passwd', 'a.lock b.lock', '--force']) {
      const branch = suggestedBranch(prompt)
      expect(agentBranchProblem(branch), `${prompt} -> ${branch}`).toBeNull()
    }
  })
})

describe('the checks on the branch', () => {
  it('accepts letters, digits, _ . / and -', () => {
    for (const name of ['agent/fix-login', 'feat/a_b.c', 'x']) expect(agentBranchProblem(name)).toBeNull()
  })

  it('refuses what the daemon or git would refuse', () => {
    for (const name of ['', 'a b', 'a~b', 'a:b', '-x', 'a..b', 'x.lock', 'a/.hidden', 'a//b', '/a', 'a/', 'naïve', '@', 'a\\b']) {
      expect(agentBranchProblem(name), JSON.stringify(name)).not.toBeNull()
    }
  })
})

describe('the checks on the prompt', () => {
  it('lets it be empty', () => {
    expect(promptProblem('')).toBeNull()
  })

  it('refuses a very long one and one that starts like an option', () => {
    expect(promptProblem('x'.repeat(MAX_PROMPT_CHARS))).toBeNull()
    expect(promptProblem('x'.repeat(MAX_PROMPT_CHARS + 1))).not.toBeNull()
    expect(promptProblem('--dangerously-skip-permissions')).not.toBeNull()
    expect(promptProblem('  -p')).not.toBeNull()
  })
})

describe('the worktree folder preview', () => {
  it('follows the daemon: beside the repository, slashes in the branch made into dashes', () => {
    expect(worktreeFolder('/home/dev/code/maitty', 'agent/fix-login', '/home/dev')).toBe('~/code/maitty-worktrees/agent-fix-login')
  })

  it('keeps a Windows project\'s separators', () => {
    expect(worktreeFolder('C:\\dev\\maitty', 'agent/x', '')).toBe('C:\\dev\\maitty-worktrees\\agent-x')
  })
})

describe('which projects can take one', () => {
  const groups = [group('/code/app', 'app', 'repo'), group('/code/notes', 'notes', 'folder'), group('build\u0000/srv/api', 'build · api', 'repo', 'build')]

  it('lists only git repositories on this computer', () => {
    expect(agentProjects(groups)).toEqual([{ key: '/code/app', name: 'app', path: '/code/app' }])
  })

  it('defaults to the focused session\'s project, else the first', () => {
    const projects = [{ key: '/a', name: 'a', path: '/a' }, { key: '/b', name: 'b', path: '/b' }]
    expect(defaultProject(projects, '/b')?.key).toBe('/b')
    expect(defaultProject(projects, 'elsewhere')?.key).toBe('/a')
    expect(defaultProject([], null)).toBeUndefined()
  })

  it('puts "New agent on a branch…" in the palette only when there is a project to put it in', () => {
    const run = vi.fn()
    const projects = agentProjects(groups)
    const item = newAgentPaletteItem(projects, true, undefined, run)
    expect(item?.label).toBe('New agent on a branch…')
    item?.run()
    expect(run).toHaveBeenCalled()
    expect(newAgentPaletteItem([], true, undefined, run)).toBeNull()
    expect(newAgentPaletteItem(projects, false, undefined, run)).toBeNull()
  })
})
