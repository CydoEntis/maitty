import { useId, useState } from 'react'
import { Icon } from '@/features/workspace/components/glyphs'

/**
 * The agent's reasoning, folded under one quiet line labelled Thought. The text is model output, so it is drawn as plain
 * text in paragraphs (never as Markdown or HTML) and goes nowhere else.
 */
export function ThoughtRow({ texts }: { texts: readonly string[] }) {
  const [open, setOpen] = useState(false)
  const panel = useId()
  return (
    <div className="mx-thought">
      <button type="button" className="mx-thought-toggle" aria-expanded={open} aria-controls={panel} aria-label={texts.length > 1 ? `Thought, ${String(texts.length)} parts` : undefined} title={open ? 'Hide the thought' : 'Show the thought'} onClick={() => { setOpen(!open) }}>
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
        Thought
      </button>
      {open && (
        <div id={panel} className="mx-thought-text">
          {texts.map((text, index) => <p key={index}>{text}</p>)}
        </div>
      )}
    </div>
  )
}
