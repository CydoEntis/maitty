import { useId, useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'
import { Markdown } from './Markdown'

/**
 * Where the agent ran out of room and carried on from its own summary of the conversation. The summary
 * is long and written for the agent, so the chat shows one line that opens to it.
 */
export function ContextSummary({ text }: { text: string }) {
  const [open, setOpen] = useState(false)
  const body = useId()
  return (
    <div className="mx-summary">
      <div className="mx-divider">
        <span className="mx-divider-line" />
        <button type="button" className="mx-summary-toggle" aria-expanded={open} aria-controls={body} title={open ? 'Hide the summary' : 'Show the summary the agent carried on from'} onClick={() => { setOpen(!open) }}>
          Earlier conversation summarised to make room
          <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
        </button>
        <span className="mx-divider-line" />
      </div>
      {open && <div id={body} className="mx-summary-body"><Markdown text={text} /></div>}
    </div>
  )
}
