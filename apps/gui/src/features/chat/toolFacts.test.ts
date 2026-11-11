import { describe, expect, it } from 'vitest'
import { toolEvent } from './eventFixtures'
import { toolFacts } from './toolFacts'

const input = (value: unknown) => JSON.stringify(value)

describe('what a tool step says it did', () => {
  it('shows a read file relative to the project, with the lines asked for', () => {
    const ranged = toolEvent('t', { name: 'Read', input: input({ file_path: '/repo/src/a.ts', offset: 40, limit: 49 }) })
    expect(toolFacts(ranged, '/repo')).toEqual({ kind: 'file', verb: 'Read', path: 'src/a.ts', lines: 'lines 40–88' })

    const fromLine = toolEvent('t', { name: 'Read', input: input({ file_path: 'a.ts', offset: 12 }) })
    expect(toolFacts(fromLine, null)).toMatchObject({ lines: 'from line 12' })
  })

  it('says nothing of lines when the whole file was read', () => {
    expect(toolFacts(toolEvent('t', { name: 'Read', target: 'a.ts' }), null)).toEqual({ kind: 'file', verb: 'Read', path: 'a.ts', lines: null })
  })

  it.each([
    ['Grep', 'Search'],
    ['Glob', 'Find files'],
  ])('names %s as "%s" with its pattern and where it looked', (name, verb) => {
    const event = toolEvent('t', { name, input: input({ pattern: 'TODO', path: '/repo/src' }) })
    expect(toolFacts(event, '/repo')).toEqual({ kind: 'pattern', verb, pattern: 'TODO', where: 'src' })
  })

  it('shows a fetched address without its protocol, and a web search as its query', () => {
    expect(toolFacts(toolEvent('t', { name: 'WebFetch', input: input({ url: 'https://example.com/docs' }) }), null)).toEqual({ kind: 'address', verb: 'Fetch', address: 'example.com/docs' })
    expect(toolFacts(toolEvent('t', { name: 'WebSearch', input: input({ query: 'rust pty' }) }), null)).toEqual({ kind: 'query', verb: 'Web search', query: 'rust pty' })
  })

  it('splits an extension tool into its server and tool', () => {
    expect(toolFacts(toolEvent('t', { name: 'mcp__github__create_issue', target: 'acme/web' }), null)).toEqual({ kind: 'extension', server: 'github', tool: 'create_issue', target: 'acme/web' })
  })

  it('falls back to the tool name and target when the input was cut and is no longer JSON', () => {
    expect(toolFacts(toolEvent('t', { name: 'Grep', target: 'TODO', input: '{"pattern": "TO' }), null)).toEqual({ kind: 'pattern', verb: 'Search', pattern: 'TODO', where: null })
    expect(toolFacts(toolEvent('t', { name: 'Mystery', target: null, input: '{"a":' }), null)).toEqual({ kind: 'other', verb: 'Mystery', target: null })
  })
})
