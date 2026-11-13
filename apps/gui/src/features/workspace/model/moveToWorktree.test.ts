import { describe, expect, it } from 'vitest'
import { moveRefusal, resumeLine, resumeWords, worktreeBranch } from './moveToWorktree'
import type { SessionView } from './session'

const ID = '0d0f5c1e-1111-4222-8333-444455556666'
const session = (patch: Partial<SessionView>) => ({ process: 'claude', agentSession: ID, status: 'idle', ...patch }) as SessionView

describe('moveRefusal', () => {
  it('allows an idle Claude or Codex chat with a saved conversation', () => {
    expect(moveRefusal(session({}))).toBeNull()
    expect(moveRefusal(session({ process: 'codex', status: 'done' }))).toBeNull()
  })
  it('refuses a working agent, a missing id, a bad id and another program', () => {
    expect(moveRefusal(session({ status: 'working' }))).toMatch(/idle/)
    expect(moveRefusal(session({ status: 'needs' }))).toMatch(/idle/)
    expect(moveRefusal(session({ agentSession: null }))).toMatch(/saved conversation/)
    expect(moveRefusal(session({ agentSession: 'x; rm -rf ~' }))).toMatch(/saved conversation/)
    expect(moveRefusal(session({ process: 'gemini' }))).toMatch(/Claude and Codex/)
  })
})

describe('resumeLine', () => {
  it('goes to the folder then resumes', () => {
    expect(resumeLine('claude', ID, '/w/task', false)).toBe(`cd "/w/task" && claude --resume ${ID}`)
    expect(resumeLine('codex', ID, '/w/task', false)).toBe(`cd "/w/task" && codex resume ${ID} -C "/w/task"`)
  })
  it('uses ; for PowerShell', () => {
    expect(resumeLine('claude', ID, 'C:\\w\\task', true)).toBe(`cd "C:\\w\\task" ; claude --resume ${ID}`)
  })
  it('refuses folders and ids that are not safe to type', () => {
    expect(resumeLine('claude', ID, '/w/a"b', false)).toBeNull()
    expect(resumeLine('claude', ID, '/w/$(x)', false)).toBeNull()
    expect(resumeLine('claude', ID, '/w/a\nb', false)).toBeNull()
    expect(resumeLine('claude', 'a b', '/w', false)).toBeNull()
  })
})

describe('resumeWords', () => {
  it('names the conversation with each agent\'s own resume option', () => {
    expect(resumeWords('claude', ID)).toEqual(['--resume', ID])
    expect(resumeWords('codex', ID)).toEqual(['resume', ID])
  })
  it('refuses an id that is not safe to type', () => {
    expect(resumeWords('claude', 'x; rm -rf ~')).toBeNull()
    expect(resumeWords('codex', '')).toBeNull()
  })
  it('refuses an id that would be read as a flag', () => {
    expect(resumeWords('claude', '--dangerously-skip-permissions')).toBeNull()
    expect(resumeWords('codex', '-yolo-0000-1111')).toBeNull()
    expect(resumeWords('codex', '_leading')).toBeNull()
  })
  it('takes every id the daemon lists: letters, digits, - and _, up to 128, a letter or digit first', () => {
    expect(resumeWords('codex', 'b7')).toEqual(['resume', 'b7'])
    expect(resumeWords('claude', 'agent_run-1')).toEqual(['--resume', 'agent_run-1'])
    expect(resumeWords('claude', 'a'.repeat(128))).not.toBeNull()
    expect(resumeWords('claude', 'a'.repeat(129))).toBeNull()
  })
})

describe('worktreeBranch', () => {
  it('avoids taken names', () => {
    expect(worktreeBranch(['main'])).not.toBe('main')
  })
})
