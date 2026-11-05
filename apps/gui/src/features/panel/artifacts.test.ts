import { describe, expect, it } from 'vitest'
import { artifactsOf, canPreview, isMarkdown } from './artifacts'
import { touchedFilesOf } from './panelModel'
import { editEvent, toolEvent } from '@/features/chat/eventFixtures'

const created = (file: string) => ({ file, kind: 'created' as const, added: 1, removed: 0 })
const same = (file: string) => file

describe('artifactsOf', () => {
  it.each([
    ['docs/plan.md', 'document', 'plan.md', 'Document · docs'],
    ['reports/coverage.html', 'report', 'coverage.html', 'Report · reports'],
    ['out/results.json', 'data', 'results.json', 'Data file · out'],
    ['shots/home.PNG', 'image', 'home.PNG', 'Image · shots'],
    ['NOTES.txt', 'document', 'NOTES.txt', 'Document'],
  ] as const)('takes the created file %s', (file, kind, title, subtitle) => {
    expect(artifactsOf([created(file)], same)).toEqual([{ file, kind, title, subtitle }])
  })

  it('takes neither source files nor files that were only edited or read', () => {
    expect(artifactsOf([
      created('src/app.ts'),
      created('Makefile'),
      created('.gitignore'),
      { file: 'docs/old.md', kind: 'edited', added: 1, removed: 1 },
      { file: 'docs/seen.md', kind: 'read', added: 0, removed: 0 },
    ], same)).toEqual([])
  })

  it('shows the project-relative folder', () => {
    expect(artifactsOf([created('/repo/docs/plan.md')], (file) => file.replace('/repo/', ''))[0]?.subtitle).toBe('Document · docs')
  })

  it('previews text but not pictures', () => {
    expect(canPreview('document')).toBe(true)
    expect(canPreview('data')).toBe(true)
    expect(canPreview('image')).toBe(false)
    expect(isMarkdown('a/b.MD')).toBe(true)
    expect(isMarkdown('a/b.txt')).toBe(false)
  })
})

describe('a Claude Write as created or edited', () => {
  const write = (id: string, file: string) => editEvent(id, { tool: 'Write', file, change: 'unknown' })

  it('is a new file when the agent had not touched it, and an edit when it had read it first', () => {
    const files = touchedFilesOf([
      write('w1', 'docs/plan.md'),
      toolEvent('r', { name: 'Read', target: 'src/a.ts' }),
      write('w2', 'src/a.ts'),
    ])

    expect(files.map((file) => [file.file, file.kind])).toEqual([['docs/plan.md', 'created'], ['src/a.ts', 'edited']])
  })

  it('keeps a file written twice as created', () => {
    expect(touchedFilesOf([write('w1', 'a.md'), write('w2', 'a.md')]).map((file) => file.kind)).toEqual(['created'])
  })
})
