import { describe, expect, it } from 'vitest'
import { pullRequestHeadline, pullRequestTone, type PullRequest } from './pullRequest'

const open: PullRequest = {
  number: 45,
  url: 'https://example.test/pr/45',
  title: 'Add a thing',
  state: 'Open',
  draft: false,
  checks: { passed: 0, pending: 0, failed: [] },
  review: 'None',
  conflicting: false,
}

describe('a pull request headline', () => {
  it('is only the number when nothing is known', () => {
    expect(pullRequestHeadline(open)).toBe('#45')
    expect(pullRequestTone(open)).toBe('plain')
  })

  it('says merged and is good', () => {
    const merged = { ...open, state: 'Merged' as const }
    expect(pullRequestHeadline(merged)).toBe('#45 merged')
    expect(pullRequestTone(merged)).toBe('good')
  })

  it('puts failing checks before everything else and is bad', () => {
    const failing = { ...open, checks: { passed: 3, pending: 1, failed: ['lint', 'test'] }, review: 'Approved' as const }
    expect(pullRequestHeadline(failing)).toBe('#45 2 failing')
    expect(pullRequestTone(failing)).toBe('bad')
  })

  it('waits on pending checks and marks a draft', () => {
    const pending = { ...open, draft: true, checks: { passed: 1, pending: 2, failed: [] } }
    expect(pullRequestHeadline(pending)).toBe('#45 checking draft')
    expect(pullRequestTone(pending)).toBe('waiting')
  })

  it('does not call a merged pull request a draft', () => {
    expect(pullRequestHeadline({ ...open, state: 'Merged', draft: true })).toBe('#45 merged')
  })
})
