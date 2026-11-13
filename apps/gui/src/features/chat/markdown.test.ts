import { describe, expect, it } from 'vitest'
import { INLINE_PARSE_LIMIT, parseInline, parseMarkdown, safeHref, streamingText, type Block, type Inline } from './markdown'

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

  it('keeps numbered entries with blank lines between them as one list', () => {
    const blocks = parseMarkdown('1. one\n\n2. two\n   more\n\n3. three\n\nafter')

    expect(kinds(blocks)).toEqual(['list', 'paragraph'])
    expect(blocks[0]).toMatchObject({ ordered: true, start: 1, items: [[text('one')], [text('two\nmore')], [text('three')]] })
  })

  it('counts a numbered list from the number it was written with', () => {
    const blocks = parseMarkdown('1. one\n\n```\ncode\n```\n\n2. two')

    expect(kinds(blocks)).toEqual(['list', 'code', 'list'])
    expect(blocks[2]).toMatchObject({ ordered: true, start: 2 })
  })

  it('keeps a bullet list and a numbered list apart across a blank line', () => {
    expect(kinds(parseMarkdown('- a\n\n1. b'))).toEqual(['list', 'list'])
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

describe('tables', () => {
  const [word, count, note] = ['Name', 'Qty', 'Note']
  const cell = (value: string): readonly Inline[] => [{ kind: 'text', text: value }]

  it('reads a header, the alignments and the rows', () => {
    const [block] = parseMarkdown(`| ${word} | ${count} | ${note} |\n| :--- | ---: | :---: |\n| a | 1 | x |\n| b | 22 | y |`)

    expect(block).toEqual({
      kind: 'table',
      align: ['left', 'right', 'center'],
      header: [cell(word), cell(count), cell(note)],
      rows: [
        [cell('a'), cell('1'), cell('x')],
        [cell('b'), cell('22'), cell('y')],
      ],
    })
  })

  it('reads a table with no outer pipes, and leaves an unaligned column unaligned', () => {
    const [block] = parseMarkdown('a | b\n--- | ---\n1 | 2')

    expect(block).toMatchObject({ kind: 'table', align: [null, null], header: [cell('a'), cell('b')], rows: [[cell('1'), cell('2')]] })
  })

  it('leaves a pipe line with no delimiter row under it as a paragraph', () => {
    expect(parseMarkdown('a | b\nc | d').map((block) => block.kind)).toEqual(['paragraph'])
  })

  it('leaves a header and a delimiter row of different widths as a paragraph', () => {
    expect(parseMarkdown('| a | b |\n| --- |\n| 1 | 2 |').map((block) => block.kind)).toEqual(['paragraph'])
  })

  it('ends the table at a blank line or a line without a pipe', () => {
    const blocks = parseMarkdown('| a | b |\n|---|---|\n| 1 | 2 |\n\nafter\n\n| c | d |\n|---|---|\n| 3 | 4 |\nnot a row')

    expect(blocks.map((block) => block.kind)).toEqual(['table', 'paragraph', 'table', 'paragraph'])
  })

  it('makes every row as wide as the header', () => {
    const [block] = parseMarkdown('| a | b |\n|---|---|\n| 1 |\n| 1 | 2 | 3 |')

    expect(block).toMatchObject({ rows: [[cell('1'), []], [cell('1'), cell('2')]] })
  })

  it('reads inline markdown in a cell, and an escaped pipe as a pipe', () => {
    const [block] = parseMarkdown('| `x` | a \\| b |\n|---|---|\n| **bold** | [l](https://example.com) |')

    expect(block).toMatchObject({
      header: [[{ kind: 'code', text: 'x' }], cell('a | b')],
      rows: [[[{ kind: 'strong' }], [{ kind: 'link', href: 'https://example.com/' }]]],
    })
  })

  it('keeps markup in a cell as text, never as elements', () => {
    const [block] = parseMarkdown('| h |\n|---|\n| <script>alert(1)</script> |')

    expect(block).toMatchObject({ rows: [[cell('<script>alert(1)</script>')]] })
  })

  it('shows a cell longer than the inline limit as plain text', () => {
    const long = `**${'x'.repeat(INLINE_PARSE_LIMIT)}**`
    const [block] = parseMarkdown(`| h |\n|---|\n| ${long} |`)

    expect(block).toMatchObject({ rows: [[cell(long)]] })
  })

  it('does not read a table inside a fence', () => {
    expect(parseMarkdown('```\n| a | b |\n|---|---|\n```').map((block) => block.kind)).toEqual(['code'])
  })
})

describe('a reply that is still streaming', () => {
  const inlines = (text: string) => parseInline(streamingText(text))
  const textOf = (nodes: readonly Inline[]): string[] => nodes.flatMap((node) => (node.kind === 'text' ? [node.text] : node.kind === 'code' ? [] : textOf(node.children)))

  it('reads an opened bold, italic or code span as if it closed at the end', () => {
    expect(inlines('This is **bol')).toMatchObject([{ kind: 'text' }, { kind: 'strong', children: [{ kind: 'text', text: 'bol' }] }])
    expect(inlines('an *it')).toMatchObject([{ kind: 'text' }, { kind: 'em', children: [{ kind: 'text', text: 'it' }] }])
    expect(inlines('run `cargo te')).toMatchObject([{ kind: 'text' }, { kind: 'code', text: 'cargo te' }])
    expect(inlines('**bold and `co')).toMatchObject([{ kind: 'strong', children: [{ kind: 'text' }, { kind: 'code', text: 'co' }, { kind: 'text' }] }])
    expect(inlines('**bold and *ital')).toMatchObject([{ kind: 'strong', children: [{ kind: 'text', text: 'bold and ' }, { kind: 'em' }, { kind: 'text' }] }])
  })

  it('reads a link with its address still arriving as a link', () => {
    expect(inlines('see [the docs](https://example.com/gu')).toMatchObject([{ kind: 'text' }, { kind: 'link', children: [{ kind: 'text', text: 'the docs' }] }])
    expect(textOf(inlines('see [the docs]('))).not.toContain('](')
  })

  it('drops a mark that has nothing after it yet, rather than showing it', () => {
    expect(streamingText('Use **')).toBe('Use ')
    expect(streamingText('Use `')).toBe('Use ')
    expect(streamingText('Use **bold*')).toBe('Use **bold**')
  })

  it('leaves a complete reply as it is', () => {
    const done = 'Use **bold**, *it*, `code` and [a link](https://example.com).'
    expect(streamingText(done)).toBe(done)
  })

  it('leaves marks that are not emphasis alone', () => {
    expect(streamingText('snake_case_name and 2 * 3')).toBe('snake_case_name and 2 * 3')
    expect(streamingText('- one *item*\n* two items')).toBe('- one *item*\n* two items')
  })

  it('shows no raw mark at any point while a reply whose marks all close grows', () => {
    const reply = 'Use **bold** and `code` and *it* and **more *nested* text** to finish.'
    for (let end = 1; end <= reply.length; end += 1) {
      const shown = textOf(inlines(reply.slice(0, end))).join('')
      expect(shown, `after ${String(end)} characters`).not.toMatch(/[*`]/)
    }
  })

  it('holds a table back until its delimiter row is complete for every header column', () => {
    expect(streamingText('| a | b |')).toBe('')
    expect(streamingText('| a | b |\n|')).toBe('')
    expect(streamingText('| a | b |\n|---')).toBe('')
    expect(streamingText('| a | b |\n|---|')).toBe('')
    expect(streamingText('Intro\n| a | b |\n|---')).toBe('Intro')
    expect(parseMarkdown(streamingText('| a | b |\n|---|--'))[0]?.kind).toBe('table')
    const table = '| a | b |\n|---|---|'
    expect(parseMarkdown(streamingText(table))[0]?.kind).toBe('table')
    const rows = `${table}\n| 1 | 2 |\n| 3 |`
    expect(parseMarkdown(streamingText(rows))[0]).toMatchObject({ kind: 'table', rows: [expect.anything(), expect.anything()] })
  })

  it('does not hold back a line of prose with a pipe in it', () => {
    expect(streamingText('pipe it through grep | sort')).toBe('pipe it through grep | sort')
  })

  it('holds back a last line that is only a list, quote or heading marker', () => {
    expect(streamingText('-')).toBe('')
    expect(streamingText('Items:\n-')).toBe('Items:')
    expect(streamingText('Steps\n1.')).toBe('Steps')
    expect(streamingText('text\n#')).toBe('text')
    expect(streamingText('text\n>')).toBe('text')
    expect(streamingText('- one\n- ')).toBe('- one')
    expect(streamingText('- one\n- two')).toBe('- one\n- two')
  })

  it('changes nothing inside an open code fence', () => {
    for (const open of ['```ts\nconst a = **x', '```\n| a | b |\n|---', '~~~\n-', 'before\n\n```\n`tick']) {
      expect(streamingText(open)).toBe(open)
    }
    expect(streamingText('```\ncode\n```\nafter **bo')).toBe('```\ncode\n```\nafter **bo**')
  })
})

