import { describe, expect, it } from 'vitest'
import { branchNameFromMessage, branchSlug, suggestBranchName } from './worktreeName'

describe('a suggested branch name', () => {
  it('is two short words', () => {
    expect(suggestBranchName(new Set(), () => 0)).toBe('bold-badger')
    expect(suggestBranchName(new Set(), () => 0.999)).toBe('young-yak')
  })

  it('is never a name already in use', () => {
    const rolls = [0, 0, 0.5, 0.5]
    let at = 0
    const random = () => rolls[at++] ?? 0.5
    expect(suggestBranchName(new Set(['bold-badger']), random)).toBe('quiet-marten')
  })

  it('takes a number when it keeps landing on names in use', () => {
    expect(suggestBranchName(new Set(['bold-badger', 'bold-badger-2']), () => 0)).toBe('bold-badger-3')
  })
})

describe('a branch slug of the first message', () => {
  it('is its words in lowercase, joined by dashes', () => {
    expect(branchSlug('Fix the login bug')).toBe('fix-the-login-bug')
  })

  it('treats punctuation, paths and command marks as gaps between words', () => {
    expect(branchSlug('  /review: src/App.tsx -- now!! ')).toBe('review-src-app-tsx-now')
    expect(branchSlug('!ls ..')).toBe('ls')
  })

  it('keeps accented letters as their plain ones and drops other scripts', () => {
    expect(branchSlug('Café crème 日本語 fix')).toBe('cafe-creme-fix')
  })

  it('stops at 40 characters on a word boundary', () => {
    const slug = branchSlug('Add a button that exports the current chat history to a markdown file')
    expect(slug).toBe('add-a-button-that-exports-the-current')
    expect(slug?.length).toBeLessThanOrEqual(40)
  })

  it('cuts a first word longer than 40 characters', () => {
    expect(branchSlug('a'.repeat(50))).toBe('a'.repeat(40))
  })

  it('is nothing for an empty message or one with no usable words', () => {
    expect(branchSlug('')).toBeNull()
    expect(branchSlug('  ?! … 日本語 ')).toBeNull()
  })

  it('is always a name git takes', () => {
    for (const message of ['Fix the login bug', '.lock', 'x.lock', '--force', 'a..b', 'HEAD@{1}', 'ünïcødé ok']) {
      const slug = branchSlug(message)
      if (slug !== null) expect(slug).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/)
    }
  })
})

describe('a branch name from the first message', () => {
  it('is the slug when nothing has it', () => {
    expect(branchNameFromMessage('Fix the login bug', new Set(['main']))).toBe('fix-the-login-bug')
  })

  it('takes -2, -3 and so on when the slug is in use', () => {
    expect(branchNameFromMessage('Fix it', new Set(['fix-it']))).toBe('fix-it-2')
    expect(branchNameFromMessage('Fix it', new Set(['fix-it', 'fix-it-2']))).toBe('fix-it-3')
  })

  it('counts a branch under the slug as a folder as in use, since git cannot make both', () => {
    expect(branchNameFromMessage('feature', new Set(['feature/login']))).toBe('feature-2')
  })

  it('is nothing when the message has no usable words, so the two-word name is used', () => {
    expect(branchNameFromMessage('   ', new Set())).toBeNull()
  })
})
