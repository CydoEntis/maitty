import { Copy, Quote } from 'lucide-react'
import type { PickedText } from '../selectionActions'

type Props = {
  picked: PickedText
  onCopy: () => void
  /** Puts the text in the message box as a quote; absent where there is no box to put it in. */
  onQuote?: (() => void) | undefined
}

/**
 * What can be done with text selected in the chat, floating where the mouse let go of it: copy it, or quote it
 * into the message box to reply to. A press on it keeps the selection, so the text it acts on stays marked.
 */
export function SelectionBar({ picked, onCopy, onQuote }: Props) {
  return (
    <div className="mx-select-bar" role="group" aria-label="Selected text" style={{ left: picked.x, top: picked.y }} onMouseDown={(event) => { event.preventDefault() }}>
      <button type="button" className="mx-select-action" onClick={onCopy}><Copy size={13} aria-hidden="true" />Copy</button>
      {onQuote !== undefined && <button type="button" className="mx-select-action" onClick={onQuote}><Quote size={13} aria-hidden="true" />Quote</button>}
    </div>
  )
}
