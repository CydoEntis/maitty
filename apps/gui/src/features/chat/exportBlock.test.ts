import { describe, expect, it } from 'vitest'
import { codeExtension, exportName, tableCsv, tableHtml, tableMarkdown, type TableBlock } from './exportBlock'
import type { Inline } from './markdown'

const text = (value: string): Inline[] => [{ kind: 'text', text: value }]

const table: TableBlock = {
  align: [null, 'right'],
  header: [text('File'), text('Added')],
  rows: [
    [[{ kind: 'code', text: 'src/tax.js' }], text('14')],
    [[{ kind: 'strong', children: text('a | b') }], text('2')],
  ],
}

describe('a table saved as a file', () => {
  it('is written back as the Markdown it came from, with a pipe in a cell escaped', () => {
    expect(tableMarkdown(table)).toBe('| File | Added |\n| --- | ---: |\n| `src/tax.js` | 14 |\n| **a \\| b** | 2 |\n')
  })

  it('is CSV with the marks dropped and awkward cells quoted', () => {
    const awkward: TableBlock = { align: [], header: [text('Name'), text('Note')], rows: [[text('a, b'), text('says "hi"')]] }

    expect(tableCsv(table)).toBe('File,Added\r\nsrc/tax.js,14\r\na | b,2\r\n')
    expect(tableCsv(awkward)).toBe('Name,Note\r\n"a, b","says ""hi"""\r\n')
  })

  it('keeps a spreadsheet from running a cell as a formula, and leaves a number a number', () => {
    const hostile: TableBlock = { align: [], header: [text('Cell')], rows: [[text('=HYPERLINK("http://x")')], [text('-5')], [text('@cmd')]] }

    expect(tableCsv(hostile)).toBe('Cell\r\n"\'=HYPERLINK(""http://x"")"\r\n-5\r\n\'@cmd\r\n')
  })

  it('is an HTML page whose cells are text, never markup', () => {
    const hostile: TableBlock = { align: ['right'], header: [text('<b>h</b>')], rows: [[text('<script>alert(1)</script>')]] }
    const page = tableHtml(hostile)

    expect(page).toContain('<th style="text-align:right">&lt;b&gt;h&lt;/b&gt;</th>')
    expect(page).toContain('&lt;script&gt;alert(1)&lt;/script&gt;')
    expect(page).not.toContain('<script>')
  })
})

describe('the file an export is saved as', () => {
  it.each([
    ['markdown', 'md'],
    ['MD', 'md'],
    ['html', 'html'],
    ['json', 'json'],
    ['ts', 'txt'],
    ['sh', 'txt'],
    ['', 'txt'],
  ])('saves a %s block as .%s', (language, extension) => {
    expect(codeExtension(language)).toBe(extension)
  })

  it('is named for what it is and when', () => {
    expect(exportName('table', 'csv', new Date(2026, 10, 4, 9, 5))).toBe('table 2026-11-04 0905.csv')
  })
})
