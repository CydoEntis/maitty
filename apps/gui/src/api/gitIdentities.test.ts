import { describe, expect, it } from 'vitest'
import { ruleCovering, type GitIdentityRule } from './gitIdentities'

const WORK = { name: 'Casey', email: 'casey@work.invalid', login: 'work-me' }
const CLIENT = { name: 'Casey', email: 'casey@client.invalid', login: 'client-me' }

describe('the folder rule that covers a folder', () => {
  const rules: GitIdentityRule[] = [
    { folder: '/srv/work', identity: WORK },
    { folder: '/srv/work/client', identity: CLIENT },
    { folder: '/srv/oss', identity: null },
  ]

  it('is the deepest rule whose folder is it or holds it', () => {
    expect(ruleCovering(rules, '/srv/work')?.login).toBe('work-me')
    expect(ruleCovering(rules, '/srv/work/app/')?.login).toBe('work-me')
    expect(ruleCovering(rules, '/srv/work/client/app')?.login).toBe('client-me')
  })

  it('is none for a folder whose name only starts the same, or one only a rule set outside Maitty covers', () => {
    expect(ruleCovering(rules, '/srv/workshop')).toBeNull()
    expect(ruleCovering(rules, '/srv/oss/thing')).toBeNull()
    expect(ruleCovering(rules, '/srv')).toBeNull()
  })

  it('matches a Windows folder whatever its case and slashes', () => {
    const windows: GitIdentityRule[] = [{ folder: 'C:/Work', identity: WORK }]
    expect(ruleCovering(windows, 'c:\\work\\app')?.login).toBe('work-me')
  })
})
