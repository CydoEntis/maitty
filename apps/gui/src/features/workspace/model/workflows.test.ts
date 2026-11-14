import { describe, expect, it } from 'vitest'
import { namesHelper } from './workflows'

describe('a step naming its helper', () => {
  it('matches the task or the name, ignoring case and punctuation', () => {
    expect(namesHelper('MAITTY-42 builder', { task: 'maitty_42_builder', name: 'x' })).toBe(true)
    expect(namesHelper('MAITTY-42 builder', { task: null, name: 'MAITTY-42 builder' })).toBe(true)
    expect(namesHelper('MAITTY-42 code reviewer', { task: 'maitty_42_code_reviewer', name: 'Volta' })).toBe(true)
  })

  it('does not match a different helper, or anything for a label of punctuation only', () => {
    expect(namesHelper('MAITTY-42 builder', { task: 'maitty_421_builder', name: 'builder' })).toBe(false)
    expect(namesHelper('--', { task: '_', name: '' })).toBe(false)
  })
})
