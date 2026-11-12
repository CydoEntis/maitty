/** The pull request of a session's branch, as the daemon polls it (`PrInfo` in the protocol). */
export type PullRequest = {
  number: number
  url: string
  title: string
  state: 'Open' | 'Merged' | 'Closed'
  draft: boolean
  checks: { passed: number; pending: number; failed: string[] }
  review: 'None' | 'Required' | 'Approved' | 'ChangesRequested'
  conflicting: boolean
}

/** How a pull request is going: `bad` needs the person, `waiting` is on checks, `good` can go. */
export type PullRequestTone = 'bad' | 'waiting' | 'good' | 'plain'

/** `#45 merged`, `#12 2 failing draft`: the same words as the terminal app (`PrInfo::headline`). */
export function pullRequestHeadline(pr: PullRequest): string {
  const fails = pr.checks.failed.length
  const open = pr.state === 'Open'
  let what = ''
  if (pr.state === 'Merged') what = 'merged'
  else if (pr.state === 'Closed') what = 'closed'
  else if (fails > 0) what = `${String(fails)} failing`
  else if (pr.conflicting) what = 'conflict'
  else if (pr.review === 'ChangesRequested') what = 'changes requested'
  else if (pr.checks.pending > 0) what = 'checking'
  else if (pr.review === 'Approved') what = 'approved'
  else if (pr.checks.passed > 0) what = 'passing'
  const draft = pr.draft && open ? ' draft' : ''
  return `#${String(pr.number)}${what === '' ? '' : ' '}${what}${draft}`
}

/** The tone of the headline (`PrInfo::tone`). */
export function pullRequestTone(pr: PullRequest): PullRequestTone {
  if (pr.state === 'Merged') return 'good'
  if (pr.state === 'Closed') return 'bad'
  if (pr.checks.failed.length > 0 || pr.conflicting || pr.review === 'ChangesRequested') return 'bad'
  if (pr.checks.pending > 0) return 'waiting'
  if (pr.review === 'Approved' || pr.checks.passed > 0) return 'good'
  return 'plain'
}
