import { memo, useContext, useState, type ReactNode } from 'react'
import { Copy, Download } from 'lucide-react'
import { HighlightedLine } from '@/components/HighlightedLine'
import { fileTag } from '@/features/inspect/fileTag'
import { extensionForLanguage } from '@/lib/highlight'
import { toast } from '@/lib/toast'
import { parseMarkdown, parseInline, type Block, type Inline, type TableAlign } from '../markdown'
import { codeExtension, tableCsv, tableHtml, tableMarkdown } from '../exportBlock'
import { OpenFileContext } from '../openFile'
import { copyBlock, saveBlock } from '../saveBlock'
import { FileBadge } from './cardParts'

/**
 * Renders the agent's markdown as React elements (never as HTML; see `markdown.ts`). A link is
 * a button, not an anchor: the app has no route that opens an address outside the window, and an
 * anchor would navigate the webview away from the app. Choosing a link copies its address.
 */

function copyLink(href: string): void {
  const clipboard = typeof navigator === 'undefined' ? undefined : navigator.clipboard
  if (clipboard === undefined) {
    toast.message(href)
    return
  }
  clipboard.writeText(href).then(
    () => { toast.message('Link copied'); },
    () => { toast.message(href) },
  )
}

// A path with a file name whose type is known, and a line number after it or not: `src/tax.js`, `src/tax.js:42`.
const FILE_NAME = /^((?:[\w@.-]+[\\/])*[\w@.-]+\.[A-Za-z0-9]+)(?::(\d+))?$/

/** Inline code. A file's name reads as a file badge that opens the file; anything else, or a name of no known type, stays code. */
function InlineCode({ text }: { text: string }) {
  const open = useContext(OpenFileContext)
  const named = FILE_NAME.exec(text)
  const path = named?.[1]
  if (path === undefined || fileTag(path) === null) return <code className="mx-code-chip">{text}</code>
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
          <button key={index} type="button" title={`${node.href} (click to copy)`} className="cursor-pointer border-0 bg-transparent p-0 text-left text-[var(--accent-text)] underline underline-offset-2 hover:opacity-80" onClick={() => { copyLink(node.href) }}>
            <Inlines nodes={node.children} />
          </button>
        )
    }
  })
}

const ALIGN: Record<'left' | 'center' | 'right', string> = { left: 'text-left', center: 'text-center', right: 'text-right whitespace-nowrap' }
const alignClass = (align: TableAlign | undefined) => ALIGN[align ?? 'left']

const HEADING_SIZE = ['mt-1.5 text-[20px] tracking-[-.015em]', 'text-[16.5px]', 'text-[14px]', 'text-[14px]', 'text-[14px]', 'text-[14px]'] as const

const CODE_FOLD_LINES = 24
/** A block only a few lines past the fold is shown whole: a button that reveals three lines is noise. */
const CODE_FOLD_SLACK = 6

/** One line of a code block. A reply being typed changes only its last line, so the lines above are not coloured again. */
const CodeLine = memo(function CodeLine({ line, hint }: { line: string; hint: string }) {
  return <span className="code-line"><HighlightedLine line={line} path={hint} />{'\n'}</span>
})

/** A fenced code block: its language, Copy and Save over the code, folded past its first lines when it is long. */
function CodeBlock({ language, text }: { language: string; text: string }) {
  const [all, setAll] = useState(false)
  const lines = text.split('\n')
  const folds = lines.length > CODE_FOLD_LINES + CODE_FOLD_SLACK
  const shown = folds && !all ? lines.slice(0, CODE_FOLD_LINES) : lines
  const hint = `code.${extensionForLanguage(language)}`
  return (
    <div className="mx-step-card">
      <div className="mx-step-head">
        <span className="mx-code-lang">{language === '' ? 'code' : language}</span>
        <span className="mx-spacer" />
        <button type="button" className="mx-ghost-btn" onClick={() => { copyBlock(text, 'Could not copy the code') }}><Copy className="size-3" aria-hidden="true" />Copy</button>
        <button type="button" className="mx-ghost-btn" title="Save as a file in your export folder" onClick={() => { saveBlock(language === '' ? 'code' : language, codeExtension(language), text) }}><Download className="size-3" aria-hidden="true" />Save</button>
      </div>
      <pre className="mx-code-body" aria-label={language === '' ? 'Code' : `Code, ${language}`}>
        <code>{shown.map((line, index) => <CodeLine key={index} line={line} hint={hint} />)}</code>
      </pre>
      {folds && (
        <button type="button" aria-expanded={all} className="mx-step-more" onClick={() => { setAll((current) => !current) }}>
          {all ? 'Show less' : `Show ${String(lines.length - CODE_FOLD_LINES)} more lines`}
        </button>
      )}
    </div>
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
      return <p role="heading" aria-level={block.level} className={`m-0 font-semibold text-fg ${HEADING_SIZE[block.level - 1] ?? 'text-[14px]'}`}><Inlines nodes={block.inlines} /></p>
    case 'code':
      return <CodeBlock language={block.language} text={block.text} />
    case 'list': {
      const Tag = block.ordered ? 'ol' : 'ul'
      return (
        <Tag className={`m-0 flex flex-col gap-1 pl-[22px] ${block.ordered ? 'list-decimal' : 'list-disc'}`}>
          {block.items.map((item, index) => <li key={index} className="whitespace-pre-wrap"><Inlines nodes={item} /></li>)}
        </Tag>
      )
    }
    case 'quote':
      return <blockquote className="m-0 border-l-[3px] border-line2 py-0.5 pl-3.5 whitespace-pre-wrap text-muted"><Inlines nodes={block.inlines} /></blockquote>
    case 'rule':
      return <hr className="mx-md-rule" />
    case 'table':
      // A table wider than the message scrolls inside its own region, which the keyboard can reach; the message never scrolls sideways.
      return (
        <div className="mx-md-tablewrap">
        <div role="region" aria-label="Table" tabIndex={0} className="mx-md-table max-w-full overflow-x-auto">
          <table className="w-full border-collapse text-[13px] leading-[1.5]">
            <thead>
              <tr>
                {block.header.map((cell, column) => (
                  <th key={column} scope="col" className={`px-3 py-2 align-top font-semibold text-fg ${alignClass(block.align[column])}`}><Inlines nodes={cell} /></th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row, at) => (
                <tr key={at}>
                  {row.map((cell, column) => (
                    <td key={column} className={`px-3 py-2 align-top text-fg2 ${alignClass(block.align[column])}`}><Inlines nodes={cell} /></td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <div className="mx-md-tableacts" role="group" aria-label="Copy or save this table">
          <button type="button" className="mx-ghost-btn" onClick={() => { copyBlock(tableMarkdown(block), 'Could not copy the table') }}><Copy className="size-3" aria-hidden="true" />Copy</button>
          <button type="button" className="mx-ghost-btn" title="Save as a Markdown file in your export folder" onClick={() => { saveBlock('table', 'md', tableMarkdown(block)) }}><Download className="size-3" aria-hidden="true" />Markdown</button>
          <button type="button" className="mx-ghost-btn" title="Save as a CSV file in your export folder" onClick={() => { saveBlock('table', 'csv', tableCsv(block)) }}><Download className="size-3" aria-hidden="true" />CSV</button>
          <button type="button" className="mx-ghost-btn" title="Save as an HTML page in your export folder" onClick={() => { saveBlock('table', 'html', tableHtml(block)) }}><Download className="size-3" aria-hidden="true" />HTML</button>
        </div>
        </div>
      )
  }
}, (before, after) => sameParsed(before.block, after.block))

/** A message's text as markdown blocks, spaced like paragraphs. */
export const Markdown = memo(function Markdown({ text, className = '' }: { text: string; className?: string }) {
  return (
    <div className={`flex flex-col gap-3 ${className}`}>
      {parseMarkdown(text).map((block, index) => <BlockView key={index} block={block} />)}
    </div>
  )
})

/** One line of text with inline markdown only (the lead sentence of a message). */
export const InlineMarkdown = memo(function InlineMarkdown({ text }: { text: string }) {
  return <Inlines nodes={parseInline(text)} />
})
