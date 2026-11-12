import { describe, expect, it } from 'vitest'
import { CheckSchema, checkActions, checkStateWords, checkTone, checksNeedingYou } from './checks'

const check = (state: string, extra: Record<string, unknown> = {}) => CheckSchema.parse({ name: 'test', command: 'cargo test', state, ...extra })

describe('a pane’s checks', () => {
  it('reads what the daemon sends, with the daemon’s defaults for what it leaves out', () => {
    const parsed = CheckSchema.parse({ state: 'Failed', source: { Repo: { file: '.maitty/checks.toml' } } })
    expect(parsed).toMatchObject({ state: 'Failed', name: '', exit_code: null })
    expect(parsed.source).toEqual({ Repo: { file: '.maitty/checks.toml', uncommitted: false } })
    expect(CheckSchema.parse({}).state).toBe('CouldNotRun')
  })

  it('offers each state only what makes sense there', () => {
    expect(checkActions(check('Failed'))).toEqual(['rerun', 'override'])
    expect(checkActions(check('Overridden'))).toEqual(['undo', 'rerun'])
    expect(checkActions(check('NeedsApproval'))).toEqual(['approve'])
    expect(checkActions(check('Checking'))).toEqual([])
    expect(checkActions(check('Passed'))).toEqual(['rerun'])
  })

  it('counts the ones that want you and says when a rerun is queued', () => {
    expect(checksNeedingYou([check('Failed'), check('Passed'), check('NeedsApproval'), check('CouldNotRun')])).toBe(3)
    expect(checkTone('Overridden')).toBe('good')
    expect(checkStateWords(check('Checking', { rerun_queued: true }))).toBe('Running (again after this)')
  })
})
