import { memo, type ReactNode } from 'react'
import { toast } from '@/lib/toast'
import { parseMarkdown, parseInline, type Block, type Inline, type TableAlign } from '../markdown'

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

function Inlines({ nodes }: { nodes: readonly Inline[] }): ReactNode {
  return nodes.map((node, index): ReactNode => {
    switch (node.kind) {
      case 'text':
        return node.text
      case 'code':
        return <code key={index} className="rounded border border-line2 bg-raise px-1 py-px font-mono text-[12.5px] text-fg">{node.text}</code>
      case 'strong':
        return <strong key={index} className="font-semibold text-fg"><Inlines nodes={node.children} /></strong>
      case 'em':
        return <em key={index}><Inlines nodes={node.children} /></em>
      case 'link':
        return (
          <button key={index} type="button" title={`${node.href} (click to copy)`} className="cursor-pointer border-0 bg-transparent p-0 text-left text-[var(--accent)] underline underline-offset-2 hover:opacity-80" onClick={() => { copyLink(node.href) }}>
            <Inlines nodes={node.children} />
          </button>
        )
    }
  })
}

const ALIGN: Record<'left' | 'center' | 'right', string> = { left: 'text-left', center: 'text-center', right: 'text-right whitespace-nowrap' }
const alignClass = (align: TableAlign | undefined) => ALIGN[align ?? 'left']

const HEADING_SIZE = ['text-[17px]', 'text-[16px]', 'text-[15px]', 'text-[14px]', 'text-[14px]', 'text-[14px]'] as const

function BlockView({ block }: { block: Block }) {
  switch (block.kind) {
    case 'paragraph':
      return <p className="m-0 whitespace-pre-wrap"><Inlines nodes={block.inlines} /></p>
    case 'heading':
      return <p role="heading" aria-level={block.level} className={`m-0 font-semibold text-fg ${HEADING_SIZE[block.level - 1] ?? 'text-[14px]'}`}><Inlines nodes={block.inlines} /></p>
    case 'code':
      return (
        <pre className="m-0 max-h-96 overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[12px] leading-[1.55] whitespace-pre text-fg" aria-label={block.language === '' ? 'Code' : `Code, ${block.language}`}>
          <code>{block.text}</code>
        </pre>
      )
    case 'list': {
      const Tag = block.ordered ? 'ol' : 'ul'
      return (
        <Tag className={`m-0 pl-5 ${block.ordered ? 'list-decimal' : 'list-disc'}`}>
          {block.items.map((item, index) => <li key={index} className="whitespace-pre-wrap py-px"><Inlines nodes={item} /></li>)}
        </Tag>
      )
    }
    case 'quote':
      return <blockquote className="m-0 border-l-2 border-line2 pl-3 whitespace-pre-wrap text-dim"><Inlines nodes={block.inlines} /></blockquote>
    case 'rule':
      return <hr className="m-0 border-0 border-t border-line2" />
    case 'table':
      // A table wider than the message scrolls inside its own region, which the keyboard can reach; the message never scrolls sideways.
      return (
        <div role="region" aria-label="Table" tabIndex={0} className="max-w-full overflow-x-auto">
          <table className="w-full border-collapse text-[13px] leading-[1.5]">
            <thead>
              <tr>
                {block.header.map((cell, column) => (
                  <th key={column} scope="col" className={`border border-line2 bg-raise px-3 py-1.5 align-top font-semibold text-fg ${alignClass(block.align[column])}`}><Inlines nodes={cell} /></th>
                ))}
              </tr>
            </thead>
            <tbody>
              {block.rows.map((row, at) => (
                <tr key={at}>
                  {row.map((cell, column) => (
                    <td key={column} className={`border border-line2 px-3 py-1.5 align-top text-fg2 ${alignClass(block.align[column])}`}><Inlines nodes={cell} /></td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )
  }
}

/** A message's text as markdown blocks, spaced like paragraphs. */
export const Markdown = memo(function Markdown({ text, className = '' }: { text: string; className?: string }) {
  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      {parseMarkdown(text).map((block, index) => <BlockView key={index} block={block} />)}
    </div>
  )
})

/** One line of text with inline markdown only (the lead sentence of a message). */
export const InlineMarkdown = memo(function InlineMarkdown({ text }: { text: string }) {
  return <Inlines nodes={parseInline(text)} />
})
