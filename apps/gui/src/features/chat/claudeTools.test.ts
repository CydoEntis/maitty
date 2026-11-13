import { describe, expect, it } from 'vitest'
import { mapClaudeTool } from './claudeTools'

const TARGET_LIMIT = 160

describe('what a Skill call names', () => {
  it('names the skill it runs', () => {
    expect(mapClaudeTool('Skill', { skill: 'kitchen:kitchen' })).toMatchObject({ kind: 'tool', name: 'Skill', target: 'kitchen:kitchen' })
  })

  it('puts the arguments after the name', () => {
    expect(mapClaudeTool('Skill', { skill: 'plan-pull', args: 'brief MAITTY-12' })).toMatchObject({ target: 'plan-pull brief MAITTY-12' })
  })

  it('cuts long arguments at the length every tool target is cut at', () => {
    const mapping = mapClaudeTool('Skill', { skill: 'roadmap', args: 'x'.repeat(TARGET_LIMIT * 2) })

    expect(mapping).toMatchObject({ target: `roadmap ${'x'.repeat(TARGET_LIMIT - 'roadmap '.length)}` })
  })

  it('keeps only the first line of arguments that span several', () => {
    expect(mapClaudeTool('Skill', { skill: 'tdd', args: 'fix the parser\nand the lexer' })).toMatchObject({ target: 'tdd fix the parser' })
  })

  it.each([
    ['no input', undefined],
    ['an empty input', {}],
    ['a blank name', { skill: '  ', args: 'brief MAITTY-12' }],
    ['a name that is not text', { skill: 7 }],
  ])('shows only "Skill" for %s', (_case, input) => {
    expect(mapClaudeTool('Skill', input)).toMatchObject({ kind: 'tool', name: 'Skill', target: null })
  })

  it('leaves another tool with the same fields alone', () => {
    expect(mapClaudeTool('Mystery', { skill: 'kitchen:kitchen' })).toMatchObject({ kind: 'tool', name: 'Mystery', target: null })
  })
})
