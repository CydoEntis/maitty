import { describe, expect, it } from 'vitest'
import { fileTag, foldersHolding } from './fileTag'

describe('a file in the tree', () => {
  it('is tagged by its type, and not at all when the type is not known', () => {
    expect(fileTag('App.tsx')).toBe('TSX')
    expect(fileTag('package.json')).toBe('{ }')
    expect(fileTag('README.MD')).toBe('MD')
    expect(fileTag('Makefile')).toBeNull()
    expect(fileTag('.gitignore')).toBeNull()
  })

  it('marks every folder above a changed file', () => {
    expect([...foldersHolding(['src/a/x.ts', 'src/y.ts', 'z.ts'])].sort()).toEqual(['src', 'src/a'])
  })
})
