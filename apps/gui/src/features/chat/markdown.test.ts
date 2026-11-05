import { describe, expect, it } from 'vitest'
import { INLINE_PARSE_LIMIT, parseInline, parseMarkdown, safeHref, type Block, type Inline } from './markdown'

describe('safeHref', () => {
  it.each([
    ['https://example.com/a?b=1', 'https://example.com/a?b=1'],
    ['http://example.com', 'http://example.com/'],
    ['javascript:alert(1)', null],
    ['JaVaScRiPt:alert(1)', null],
    ['data:text/html;base64,PHNjcmlwdD4=', null],
    ['file:///etc/passwd', null],
    ['vbscript:x', null],
    ['/relative/path', null],
    ['//evil.example', null],
    ['https://a.example/"onmouseover="x', null],
    ['https://a.example/ b', null],
    ['', null],
  ])('%s -> %s', (input, expected) => {
    expect(safeHref(input)).toBe(expected)
  })
})

const text = (value: string): Inline => ({ kind: 'text', text: value })

describe('inline markdown', () => {
  it.each([
    ['plain', [text('plain')]],
    ['use `ls -la` here', [text('use '), { kind: 'code', text: 'ls -la' }, text(' here')]],
    ['**bold** and *soft* and _under_', [{ kind: 'strong', children: [text('bold')] }, text(' and '), { kind: 'em', children: [text('soft')] }, text(' and '), { kind: 'em', children: [text('under')] }]],
    ['snake_case_name stays', [text('snake_case_name stays')]],
    ['2 * 3 * 4', [text('2 * 3 * 4')]],
    ['a \\*star\\*', [text('a *star*')]],
    ['unclosed **bold', [text('unclosed **bold')]],
    ['unclosed `tick', [text('unclosed `tick')]],
  ] as const)('%s', (input, expected) => {
    expect(parseInline(input)).toEqual(expected)
  })

  it('keeps a safe link and drops an unsafe one to its label', () => {
    expect(parseInline('see [docs](https://example.com/x) now')).toEqual([text('see '), { kind: 'link', href: 'https://example.com/x', children: [text('docs')] }, text(' now')])
    expect(parseInline('[click](javascript:alert(1))')).toEqual([text('click')])
    expect(parseInline('[click](data:text/html,<script>alert(1)</script>)')).toEqual([text('click')])
  })

  it('links a bare address without its trailing punctuation', () => {
    expect(parseInline('go to https://example.com/a.')).toEqual([text('go to '), { kind: 'link', href: 'https://example.com/a', children: [text('https://example.com/a')] }, text('.')])
  })

  it('shows an image as its alt text and loads nothing', () => {
    expect(parseInline('![logo](https://tracker.example/p.png)')).toEqual([text('logo')])
  })

  it('leaves raw HTML as text for the view to escape', () => {
    expect(parseInline('<img src=x onerror=alert(1)>')).toEqual([text('<img src=x onerror=alert(1)>')])
  })

  it('does not scan an enormous run, so a hostile message cannot stall the window', () => {
    const hostile = '['.repeat(INLINE_PARSE_LIMIT + 1)
    const started = performance.now()

    expect(parseInline(hostile)).toEqual([text(hostile)])
    expect(performance.now() - started).toBeLessThan(250)
  })

  it('stops nesting at a fixed depth', () => {
    const nested = `${'*_'.repeat(20)}x${'_*'.repeat(20)}`

    expect(() => parseInline(nested)).not.toThrow()
  })
})

describe('block markdown', () => {
  const kinds = (blocks: readonly Block[]) => blocks.map((block) => block.kind)

  it('reads headings, paragraphs, lists, quotes and rules', () => {
    const blocks = parseMarkdown('# Title\n\nOne line\nsecond line\n\n- a\n- b\n  more\n\n1. x\n2) y\n\n> quoted\n> text\n\n---')

    expect(kinds(blocks)).toEqual(['heading', 'paragraph', 'list', 'list', 'quote', 'rule'])
    expect(blocks[2]).toMatchObject({ ordered: false, items: [[text('a')], [text('b\nmore')]] })
    expect(blocks[3]).toMatchObject({ ordered: true })
    expect(blocks[4]).toMatchObject({ inlines: [text('quoted\ntext')] })
  })

  it('keeps a fenced block as code, blank lines and markup characters included', () => {
    const [block] = parseMarkdown('```ts\nconst a = **x**\n\n<script>alert(1)</script>\n```')

    expect(block).toEqual({ kind: 'code', language: 'ts', text: 'const a = **x**\n\n<script>alert(1)</script>' })
  })

  it('lets an unterminated fence run to the end', () => {
    expect(parseMarkdown('```\nopen\nstill')).toEqual([{ kind: 'code', language: '', text: 'open\nstill' }])
  })

  it('reads any text without throwing', () => {
    for (const hostile of ['', '\u0000‮', '```', '- ', '> ', '#', '[](', '*'.repeat(5000), '\r\n\r\n']) {
      expect(() => parseMarkdown(hostile)).not.toThrow()
    }
  })
})
