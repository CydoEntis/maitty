import type { Inline, TableAlign } from './markdown'

/**
 * A block of a reply as the text of a file: a table as Markdown, CSV or a small HTML page, a code block as
 * what it holds. Pure, so what is saved can be tested without a disk.
 */

export type TableBlock = {
  readonly align: readonly TableAlign[]
  readonly header: readonly (readonly Inline[])[]
  readonly rows: readonly (readonly (readonly Inline[])[])[]
}

/** The words of a cell with its marks dropped. */
export function inlineText(nodes: readonly Inline[]): string {
  return nodes.map((node) => ('text' in node ? node.text : inlineText(node.children))).join('')
}

/** A cell as the Markdown it was written in. */
function inlineMarkdown(nodes: readonly Inline[]): string {
  return nodes.map((node) => {
    switch (node.kind) {
      case 'text': return node.text
      case 'code': return `\`${node.text}\``
      case 'strong': return `**${inlineMarkdown(node.children)}**`
      case 'em': return `*${inlineMarkdown(node.children)}*`
      case 'link': return `[${inlineMarkdown(node.children)}](${node.href})`
    }
  }).join('')
}

const RULE: Record<'left' | 'center' | 'right' | 'none', string> = { left: ':---', center: ':---:', right: '---:', none: '---' }

export function tableMarkdown(table: TableBlock): string {
  // A pipe inside a cell would end it.
  const line = (cells: readonly (readonly Inline[])[]) => `| ${cells.map((cell) => inlineMarkdown(cell).replace(/\|/g, '\\|')).join(' | ')} |`
  const rule = `| ${table.header.map((_, column) => RULE[table.align[column] ?? 'none']).join(' | ')} |`
  return [line(table.header), rule, ...table.rows.map(line)].join('\n') + '\n'
}

/** A spreadsheet runs a cell that starts like this as a formula. An agent's text is not trusted to be one. */
const FORMULA_START = /^[=+\-@\t\r]/
const PLAIN_NUMBER = /^[+-]?\d+(?:[.,]\d+)*%?$/

function csvCell(text: string): string {
  const safe = FORMULA_START.test(text) && !PLAIN_NUMBER.test(text) ? `'${text}` : text
  return /[",\r\n]/.test(safe) ? `"${safe.replace(/"/g, '""')}"` : safe
}

export function tableCsv(table: TableBlock): string {
  return [table.header, ...table.rows].map((row) => row.map((cell) => csvCell(inlineText(cell))).join(',')).join('\r\n') + '\r\n'
}

const HTML_ESCAPES: Readonly<Record<string, string>> = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }
const escapeHtml = (text: string) => text.replace(/[&<>"]/g, (character) => HTML_ESCAPES[character] ?? character)

/** A page that holds the table and nothing else: its cells are text, so nothing an agent wrote runs when it is opened. */
export function tableHtml(table: TableBlock): string {
  const cells = (tag: 'th' | 'td', row: readonly (readonly Inline[])[]) => row.map((cell, column) => {
    const align = table.align[column] ?? null
    return `<${tag}${align === null ? '' : ` style="text-align:${align}"`}>${escapeHtml(inlineText(cell))}</${tag}>`
  }).join('')
  const body = table.rows.map((row) => `<tr>${cells('td', row)}</tr>`).join('\n')
  return [
    '<!doctype html>',
    '<html lang="en">',
    '<head><meta charset="utf-8"><title>Table</title>',
    '<style>body{font-family:system-ui,sans-serif;margin:2rem}table{border-collapse:collapse}th,td{border:1px solid;padding:.4rem .7rem;text-align:left}</style>',
    '</head>',
    '<body>',
    '<table>',
    `<thead><tr>${cells('th', table.header)}</tr></thead>`,
    `<tbody>\n${body}\n</tbody>`,
    '</table>',
    '</body>',
    '</html>',
    '',
  ].join('\n')
}

/** The file types the host saves (`export.rs`), by the names a code block's language goes by. Anything else is plain text. */
const CODE_EXTENSION: Readonly<Record<string, string>> = { md: 'md', markdown: 'md', mdx: 'md', html: 'html', htm: 'html', csv: 'csv', json: 'json' }

export function codeExtension(language: string): string {
  return CODE_EXTENSION[language.trim().toLowerCase()] ?? 'txt'
}

const TWO_DIGITS = 2
const pad = (value: number) => String(value).padStart(TWO_DIGITS, '0')

/** "table 2026-11-10 1653.md": what it is and when, in the person's own time, so a folder of exports sorts by day. */
export function exportName(what: string, extension: string, now: Date): string {
  const day = `${String(now.getFullYear())}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
  return `${what} ${day} ${pad(now.getHours())}${pad(now.getMinutes())}.${extension}`
}
