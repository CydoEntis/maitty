import { describe, expect, it } from 'vitest'
import { BUNDLED_FONT_STACK, fontStackFor, installedMonospaceFamilies, isFontFamily, isFontWeight } from './terminalFont'

describe('terminal font', () => {
  it('accepts plain family names and an empty one', () => {
    for (const name of ['', '  ', 'JetBrains Mono', 'Fira Code', 'SF Mono', 'Iosevka-Term', 'MesloLGS NF']) expect(isFontFamily(name)).toBe(true)
  })

  it('refuses anything that could break out of a CSS font list', () => {
    for (const name of ['a", evil', "x'; y", 'a;b', 'a{b}', 'a\\b', 'a,b', '<b>', 'x'.repeat(65), 42, null, undefined]) expect(isFontFamily(name)).toBe(false)
  })

  it('puts the chosen family in front of the bundled font, and uses the bundled font alone for none', () => {
    expect(fontStackFor('Fira Code')).toBe(`"Fira Code", ${BUNDLED_FONT_STACK}`)
    expect(fontStackFor('  ')).toBe(BUNDLED_FONT_STACK)
    expect(fontStackFor('bad;name')).toBe(BUNDLED_FONT_STACK)
  })

  it('knows the weights xterm is given', () => {
    expect([300, 400, 500, 600, 700, 800].every(isFontWeight)).toBe(true)
    for (const weight of [0, 450, 900, '400', 'bold', null]) expect(isFontWeight(weight)).toBe(false)
  })

  it('reports no installed fonts where there is no canvas to measure with', () => {
    expect(installedMonospaceFamilies(['Fira Code'])).toEqual([])
  })
})
