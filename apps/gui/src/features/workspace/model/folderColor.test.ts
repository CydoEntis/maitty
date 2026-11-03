import { describe, expect, it } from 'vitest'
import { assignProjectColors, folderColor } from './folderColor'

describe('folder colour', () => {
  it('gives the same folder the same colour every time', () => {
    expect(folderColor('/home/casey/maitty')).toEqual(folderColor('/home/casey/maitty'))
  })

  it('spreads different folders over several colours', () => {
    const keys = ['/a', '/b', '/c', '/d', '/e', '/f', '/g', '/h', '/i', '/j', '/k', '/l']
    expect(new Set(keys.map((key) => folderColor(key).from)).size).toBeGreaterThan(3)
  })

  it('takes every colour from the theme, so a theme switch recolours the folders', () => {
    const keys = Array.from({ length: 40 }, (_, index) => `/folder/${String(index)}`)
    for (const key of keys) expect(folderColor(key).from).not.toMatch(/#[0-9a-fA-F]{3,8}/)
  })

  it('always returns a light and a dark end', () => {
    const color = folderColor('')
    expect(color.from).toMatch(/^(var|color-mix)\(/)
    expect(color.to).toMatch(/^color-mix\(/)
  })

  it('gives projects different colours, even ones whose paths would pick the same', () => {
    const keys = ['/dev/AeVoxLegal', '/dev/AeVoxGrowth', '/dev/AeVoxPlatform', '/dev/AeVoxAffiliate', '/code/maitty']
    const colours = assignProjectColors(keys, {})
    expect(new Set(colours.values()).size).toBe(keys.length)
  })

  it('keeps the colour you chose and keeps everyone else off it', () => {
    const colours = assignProjectColors(['/a', '/b', '/c'], { '/b': 'blue' })
    expect(colours.get('/b')).toBe('blue')
    expect([...colours.values()].filter((id) => id === 'blue')).toHaveLength(1)
  })

  it('gives the same projects the same colours however they are listed', () => {
    const one = assignProjectColors(['/a', '/b', '/c', '/d'], {})
    const other = assignProjectColors(['/d', '/c', '/b', '/a'], {})
    expect([...one].sort()).toEqual([...other].sort())
  })

  it('reuses colours only once every colour is taken', () => {
    const keys = Array.from({ length: 14 }, (_, index) => `/p${String(index)}`)
    const used = [...assignProjectColors(keys, {}).values()]
    expect(new Set(used).size).toBe(10)
    expect(used).toHaveLength(14)
  })

  it('never uses amber or red for a project', () => {
    const used = [...assignProjectColors(Array.from({ length: 10 }, (_, index) => `/p${String(index)}`), {}).values()]
    for (const id of used) expect(folderColor('', id).from).not.toMatch(/--warn|var\(--danger\)$/)
  })
})
