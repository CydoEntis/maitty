import { useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'

/**
 * The agent's reasoning, folded under one line labelled Thought. The text is model output, so it is drawn as plain
 * text in paragraphs (never as Markdown or HTML) and goes nowhere else.
 */
export function ThoughtRow({ texts }: { texts: readonly string[] }) {
  const [open, setOpen] = useState(false)
  return (
    <div className="mx-work">
      <button type="button" className="mx-work-head" aria-expanded={open} title={open ? 'Hide the thought' : 'Show the thought'} onClick={() => { setOpen(!open) }}>
        <span className="mx-work-text">Thought</span>
        <span className="mx-spacer" />
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
      </button>
      {open && (
        <div className="mx-work-steps mx-thought-text">
          {texts.map((text, index) => <p key={index}>{text}</p>)}
        </div>
      )}
    </div>
  )
}
