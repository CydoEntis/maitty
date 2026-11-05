import { Check, SquareTerminal } from 'lucide-react'
import { Spinner } from '@/features/workspace/components/glyphs'
import type { ItemStatus } from '../chatEvents'

/** Small pieces the chat cards share. Colours are theme tokens. */

const STATUS_WORD: Record<ItemStatus, string> = { running: 'Running', completed: 'Done', failed: 'Failed', interrupted: 'Interrupted' }

/** Spinner while running, a green check when done, a terminal glyph otherwise (v18 C.1 tool chip). */
export function StatusMark({ status }: { status: ItemStatus }) {
  if (status === 'running') return <Spinner className="mx-spin text-[var(--accent)]" />
  if (status === 'completed') return <Check className="size-3.5 shrink-0 text-[var(--ok)]" aria-hidden="true" />
  return <SquareTerminal className={`size-3.5 shrink-0 ${status === 'failed' ? 'text-[var(--danger)]' : 'text-dim'}`} aria-hidden="true" />
}

export function StatusWord({ status }: { status: ItemStatus }) {
  return <span className={`text-[11px] ${status === 'failed' ? 'text-[var(--danger)]' : 'text-dim'}`}>{STATUS_WORD[status]}</span>
}

/** Text the provider produced, shown as text in a bounded well (never as markup). */
export function Detail({ text, empty }: { text: string; empty: string }) {
  return text.trim() === ''
    ? <p className="mt-1.5 mb-0 text-[12px] text-dim">{empty}</p>
    : <pre className="mt-1.5 mb-0 max-h-48 overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[11.5px] leading-[1.55] whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{text}</pre>
}
