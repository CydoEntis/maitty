import { describe, expect, it } from 'vitest'
import { appliedEffect, type EnabledEffects } from './motion'

const ALL: EnabledEffects = { working: 'waves', needs: 'sweep', done: 'sweep', workingScope: 'all', cap: 'waves' }

describe('applied status effect', () => {
  it('gives each status the style chosen for it', () => {
    expect(appliedEffect(ALL, 'working', false)).toEqual({ className: 'mx-fx-working', status: 'working', style: 'waves' })
    expect(appliedEffect(ALL, 'needs', true)).toEqual({ className: 'mx-fx-needs mx-fx-strong', status: 'needs', style: 'sweep' })
    expect(appliedEffect(ALL, 'done', false).style).toBe('sweep')
  })

  it('shows nothing for statuses without an effect', () => {
    for (const status of ['idle', 'error', 'unknown'] as const) expect(appliedEffect(ALL, status, true).style).toBeNull()
  })

  it('shows nothing for an effect that is off', () => {
    expect(appliedEffect({ ...ALL, done: null }, 'done', true).style).toBeNull()
  })

  it('limits the working effect to the selected row when asked, and drops it when off', () => {
    const selectedOnly = { ...ALL, workingScope: 'selected' } as const
    expect(appliedEffect(selectedOnly, 'working', false).style).toBeNull()
    expect(appliedEffect(selectedOnly, 'working', true).style).toBe('waves')
    expect(appliedEffect({ ...ALL, workingScope: 'off' }, 'working', true).style).toBeNull()
    expect(appliedEffect({ ...ALL, workingScope: 'off' }, 'needs', false).style).toBe('sweep')
  })
})
