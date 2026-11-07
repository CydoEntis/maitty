import { describe, expect, it } from 'vitest'
import { fuzzyFind } from './fuzzy'

const paths = (hits: ReturnType<typeof fuzzyFind>) => hits.map((hit) => hit.path)

describe('fuzzyFind', () => {
  it('finds README.md for rdme and ranks it above looser matches', () => {
    const found = fuzzyFind('rdme', ['src/render_dom_element.ts', 'docs/some-readme-notes.md', 'README.md'], 10)
    expect(found[0]?.path).toBe('README.md')
    // The others also match, one inside a word and one as word initials, in either order.
    expect(paths(found).slice(1).sort()).toEqual(['docs/some-readme-notes.md', 'src/render_dom_element.ts'])
  })

  it('ignores case in what is typed and in the path', () => {
    expect(paths(fuzzyFind('READ', ['readme.md', 'other.txt'], 10))).toEqual(['readme.md'])
  })

  it('returns nothing when the letters do not all appear in order', () => {
    expect(fuzzyFind('zq', ['README.md', 'src/main.rs'], 10)).toEqual([])
    expect(fuzzyFind('mr', ['rm.txt'], 10)).toEqual([])
  })

  it('keeps the first paths unchanged for an empty or blank query', () => {
    const all = ['a', 'b', 'c']
    expect(paths(fuzzyFind('', all, 2))).toEqual(['a', 'b'])
    expect(paths(fuzzyFind('   ', all, 10))).toEqual(all)
  })

  it('prefers a match in the file name to one in the folders', () => {
    const found = fuzzyFind('mod', ['modules/readme.txt', 'src/mod.rs'], 10)
    expect(paths(found)).toEqual(['src/mod.rs', 'modules/readme.txt'])
  })

  it('reports where the letters landed, in the full path', () => {
    const [hit] = fuzzyFind('mrs', ['src/main.rs'], 10)
    expect(hit?.path).toBe('src/main.rs')
    expect(hit?.positions.map((at) => 'src/main.rs'.charAt(at)).join('')).toBe('mrs')
    expect(hit?.positions).toEqual([4, 9, 10])
  })

  it('takes a word start a little further on over the first plain letter', () => {
    const [hit] = fuzzyFind('b', ['abc_b.txt'], 10)
    expect(hit?.positions).toEqual([4])
  })

  it('keeps the order of the input for equal scores, so results do not jump about', () => {
    expect(paths(fuzzyFind('a', ['x/a', 'y/a', 'z/a'], 10))).toEqual(['x/a', 'y/a', 'z/a'])
  })

  it('stops at the limit', () => {
    const many = Array.from({ length: 50 }, (_, n) => `dir/file${String(n)}.txt`)
    expect(fuzzyFind('file', many, 7)).toHaveLength(7)
  })

  it('is quick enough for 5000 paths and a five letter query', () => {
    const many = Array.from({ length: 5000 }, (_, n) => `project/packages/module${String(n % 90)}/src/components/widget${String(n)}.tsx`)
    const started = performance.now()
    const found = fuzzyFind('wdgt1', many, 50)
    const took = performance.now() - started
    expect(found.length).toBeGreaterThan(0)
    // The aim is well under 50 ms; the bound is loose so a slow machine does not fail the run.
    expect(took).toBeLessThan(500)
  })
})
