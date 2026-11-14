import { memo, useContext, useState, type ReactNode } from 'react'
import { ChevronDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardHead } from '@/components/ui/card'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Fold } from '@/components/ui/fold'
import { Counter } from '@/components/ui/progress'
import { CodeChip } from '@/components/ui/tag'
import { HighlightedLine } from '@/components/HighlightedLine'
import { fileTag } from '@/features/inspect/fileTag'
import { extensionForLanguage } from '@/lib/highlight'
import { toast } from '@/lib/toast'
import { copyText } from '@/lib/clipboard'
import { errorMessage } from '@/lib/error-message'
import { isDesktop, openLink } from '@/api/daemon'
import { parseMarkdown, parseInline, type Block, type Inline, type TableAlign } from '../markdown'
import { codeExtension, tableCsv, tableHtml, tableMarkdown } from '../exportBlock'
import { OpenFileContext } from '../openFile'
import { copyBlock, saveBlock } from '../saveBlock'
import { FileBadge } from './cardParts'

/**
 * Renders the agent's markdown as React elements (never as HTML; see `markdown.ts`). A link is
 * a button, not an anchor: an anchor would navigate the webview away from the app. Choosing a link
 * opens it in the person's own browser, through the host; outside the desktop app, where there is
 * no host to ask, it copies the address.
 */

function copyLink(href: string): void {
  copyText(href).then(
    () => { toast.message('Link copied') },
    () => { toast.error('Could not copy the link') },
  )
}

function followLink(href: string): void {
  if (!isDesktop()) {
    copyLink(href)
    return
  }
  openLink(href).then(
    () => { toast.message('Opened in your browser', { action: { label: 'Copy link', onClick: () => { copyLink(href) } } }) },
    (error: unknown) => { toast.error(errorMessage(error, 'Could not open the link'), { action: { label: 'Copy link', onClick: () => { copyLink(href) } } }) },
  )
}

// A path with a file name whose type is known, and a line number after it or not: `src/tax.js`, `src/tax.js:42`.
const FILE_NAME = /^((?:[\w@.-]+[\\/])*[\w@.-]+\.[A-Za-z0-9]+)(?::(\d+))?$/

/** Inline code. A file's name reads as a file badge that opens the file; anything else, or a name of no known type, stays code. */
function InlineCode({ text }: { text: string }) {
  const open = useContext(OpenFileContext)
  const named = FILE_NAME.exec(text)
  const path = named?.[1]
  if (path === undefined || fileTag(path) === null) return <CodeChip>{text}</CodeChip>
  const line = named?.[2] === undefined ? null : Number(named[2])
  return <FileBadge path={path} line={line} onOpen={open === null ? undefined : () => { open(path, line) }} />
}

function Inlines({ nodes }: { nodes: readonly Inline[] }): ReactNode {
  return nodes.map((node, index): ReactNode => {
    switch (node.kind) {
      case 'text':
        return node.text
      case 'code':
        return <InlineCode key={index} text={node.text} />
      case 'strong':
        return <strong key={index} className="font-semibold text-fg"><Inlines nodes={node.children} /></strong>
      case 'em':
        return <em key={index}><Inlines nodes={node.children} /></em>
      case 'link':
        return (
          <button key={index} type="button" title={`${node.href} (click to open in your browser)`} className="cursor-pointer border-0 bg-transparent p-0 text-left text-(--link) underline decoration-[color-mix(in_oklab,var(--link)_45%,transparent)] underline-offset-2 hover:decoration-(--link)" onClick={() => { followLink(node.href) }}>
            <Inlines nodes={node.children} />
          </button>
        )
    }
  })
}

const ALIGN: Record<'left' | 'center' | 'right', string> = { left: 'text-left', center: 'text-center', right: 'text-right whitespace-nowrap' }
const alignClass = (align: TableAlign | undefined) => ALIGN[align ?? 'left']

/** Headings 1 to 6 as the design sizes them: 22, 18, 16, 14.5, 13 and 12 (the last in capitals, muted). */
const HEADING_SIZE = [
  'mt-2 text-[22px] leading-[1.3] font-[650] tracking-[-.01em] text-fg',
  'mt-2 text-[18px] leading-[1.3] font-[650] tracking-[-.01em] text-fg',
  'mt-1 text-[16px] leading-[1.3] font-[650] text-fg',
  'mt-1 text-[14.5px] leading-[1.3] font-[650] text-fg',
  'mt-1 text-[13px] leading-[1.3] font-semibold text-fg',
  'mt-1 text-[12px] leading-[1.3] font-semibold uppercase tracking-[.04em] text-(--muted)',
] as const

const CODE_FOLD_LINES = 24
/** A block only a few lines past the fold is shown whole: a button that reveals three lines is noise. */
const CODE_FOLD_SLACK = 6

/** One line of a code block with its number. A reply being typed changes only its last line, so the lines above are not coloured again. */
const CodeLine = memo(function CodeLine({ line, number, hint }: { line: string; number: number; hint: string }) {
  return <span className="code-line"><span className="mx-code-num" aria-hidden="true">{number}</span><span className="mx-code-text"><HighlightedLine line={line} path={hint} /></span>{'\n'}</span>
})

/** A fenced code block: its language, Copy and Save over the code, folded past its first lines when it is long. */
function CodeBlock({ language, text }: { language: string; text: string }) {
  const [all, setAll] = useState(false)
  const lines = text.split('\n')
  const folds = lines.length > CODE_FOLD_LINES + CODE_FOLD_SLACK
  const shown = folds && !all ? lines.slice(0, CODE_FOLD_LINES) : lines
  const hint = `code.${extensionForLanguage(language)}`
  return (
    <Card className="mx-step-card">
      <CardHead>
        <span className="mx-code-lang">{language === '' ? 'code' : language}</span>
        <span className="mx-spacer" />
        <Button variant="ghost" size="sm" onClick={() => { copyBlock(text, 'Could not copy the code') }}>Copy</Button>
        <Button variant="ghost" size="sm" title="Save as a file in your export folder" onClick={() => { saveBlock(language === '' ? 'code' : language, codeExtension(language), text) }}>Save</Button>
      </CardHead>
      <pre className="mx-code-body" aria-label={language === '' ? 'Code' : `Code, ${language}`}>
        <code>{shown.map((line, index) => <CodeLine key={index} line={line} number={index + 1} hint={hint} />)}</code>
      </pre>
      {folds && <Fold open={all} onToggle={() => { setAll((current) => !current) }} more={`Show ${String(lines.length - CODE_FOLD_LINES)} more lines`} less="Show less" />}
    </Card>
  )
}

type TableBlock = Extract<Block, { kind: 'table' }>

/**
 * A table (`TableCard`): its head says how many rows it has, with Copy and Save as (Markdown, CSV, HTML). Cells never
 * wrap; a table wider than the card scrolls sideways inside its own region, which the keyboard can reach.
 */
function TableCard({ block }: { block: TableBlock }) {
  const rows = block.rows.length
  return (
    <Card className="mx-step-card mx-md-tablecard">
      <CardHead className="mx-md-tableacts" role="group" aria-label="Copy or save this table">
        <span className="mx-md-tabletitle">Table</span>
        <Counter>{rows} {rows === 1 ? 'row' : 'rows'}</Counter>
        <span className="mx-spacer" />
        <Button variant="ghost" size="sm" onClick={() => { copyBlock(tableMarkdown(block), 'Could not copy the table') }}>Copy</Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm">Save as<ChevronDown aria-hidden="true" /></Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" container={document.querySelector<HTMLElement>('.mx-app')} className="text-fg">
            <DropdownMenuItem title="Save as a Markdown file in your export folder" onSelect={() => { saveBlock('table', 'md', tableMarkdown(block)) }}>Markdown</DropdownMenuItem>
            <DropdownMenuItem title="Save as a CSV file in your export folder" onSelect={() => { saveBlock('table', 'csv', tableCsv(block)) }}>CSV</DropdownMenuItem>
            <DropdownMenuItem title="Save as an HTML page in your export folder" onSelect={() => { saveBlock('table', 'html', tableHtml(block)) }}>HTML</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </CardHead>
      <div role="region" aria-label="Table" tabIndex={0} className="mx-md-table max-w-full overflow-x-auto">
        <table className="w-full border-collapse">
          <thead>
            <tr>
              {block.header.map((cell, column) => (
                <th key={column} scope="col" className={alignClass(block.align[column])}><Inlines nodes={cell} /></th>
              ))}
            </tr>
          </thead>
          <tbody>
            {block.rows.map((row, at) => (
              <tr key={at}>
                {row.map((cell, column) => (
                  <td key={column} className={alignClass(block.align[column])}><Inlines nodes={cell} /></td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  )
}

/** Whether two parsed values say the same thing: plain data only, as the parser makes it. */
function sameParsed(a: unknown, b: unknown): boolean {
  if (a === b) return true
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false
  if (Array.isArray(a) !== Array.isArray(b)) return false
  const [left, right] = [a as Record<string, unknown>, b as Record<string, unknown>]
  const keys = Object.keys(left)
  return keys.length === Object.keys(right).length && keys.every((key) => sameParsed(left[key], right[key]))
}

/**
 * One block. A reply that is being typed out is parsed again about forty times a second, and only its last block
 * differs each time: comparing a block's parse is far cheaper than rendering it again, so the ones above stay as
 * they are.
 */
const BlockView = memo(function BlockView({ block }: { block: Block }) {
  switch (block.kind) {
    case 'paragraph':
      return <p className="m-0 whitespace-pre-wrap"><Inlines nodes={block.inlines} /></p>
    case 'heading':
      return <p role="heading" aria-level={block.level} className={`mb-0 ${HEADING_SIZE[block.level - 1] ?? HEADING_SIZE[3]}`}><Inlines nodes={block.inlines} /></p>
    case 'code':
      return <CodeBlock language={block.language} text={block.text} />
    case 'list': {
      const Tag = block.ordered ? 'ol' : 'ul'
      return (
        <Tag className={`m-0 flex flex-col gap-1 ${block.ordered ? 'list-decimal pl-[22px]' : 'list-disc pl-5'}`}>
          {block.items.map((item, index) => <li key={index} className="whitespace-pre-wrap"><Inlines nodes={item} /></li>)}
        </Tag>
      )
    }
    case 'quote':
      return <blockquote className="m-0 border-l-2 border-line2 py-0.5 pl-3.5 whitespace-pre-wrap text-muted"><Inlines nodes={block.inlines} /></blockquote>
    case 'rule':
      return <hr className="mx-md-rule" />
    case 'table':
      return <TableCard block={block} />
  }
}, (before, after) => sameParsed(before.block, after.block))

/** A message's text as markdown blocks, spaced like paragraphs. */
export const Markdown = memo(function Markdown({ text, className = '' }: { text: string; className?: string }) {
  return (
    <div className={`flex flex-col gap-2.5 ${className}`}>
      {parseMarkdown(text).map((block, index) => <BlockView key={index} block={block} />)}
    </div>
  )
})

/** One line of text with inline markdown only (the lead sentence of a message). */
export const InlineMarkdown = memo(function InlineMarkdown({ text }: { text: string }) {
  return <Inlines nodes={parseInline(text)} />
})
