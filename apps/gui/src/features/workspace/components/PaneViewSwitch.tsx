import { MessageSquare, SquareTerminal } from 'lucide-react'
import type { PaneView } from '../model/chatView'
import { useTipWithKeys } from '@/features/shortcuts/platform'

type Props = {
  view: PaneView
  onChange: (view: PaneView) => void
}

/**
 * The Terminal | Chat switch of a pane (v18 B.3). Choosing a view changes what the pane shows and
 * sends nothing to the daemon: both views read the same session. The thumb moves with a transform
 * only, and Reduce motion turns the move off at the window root.
 */
export function PaneViewSwitch({ view, onChange }: Props) {
  const tip = useTipWithKeys()
  const choices: ReadonlyArray<{ value: PaneView; label: string; icon: typeof MessageSquare }> = [
    { value: 'terminal', label: tip('Terminal view', 'chat-toggle'), icon: SquareTerminal },
    { value: 'chat', label: tip('Chat view', 'chat-toggle'), icon: MessageSquare },
  ]
  return (
    <div
      role="radiogroup"
      aria-label="Pane view"
      className="relative mr-1 grid h-7 grid-cols-2 items-center rounded-lg border border-line2 bg-raise p-0.5"
      onMouseDown={(event) => { event.stopPropagation() }}
    >
      <span
        aria-hidden="true"
        className={`pointer-events-none absolute inset-y-0.5 left-0.5 w-[calc(50%-2px)] rounded-md bg-hover transition-transform duration-[320ms] ease-[cubic-bezier(.2,.8,.2,1)] ${view === 'chat' ? 'translate-x-full' : ''}`}
      />
      {choices.map(({ value, label, icon: ChoiceIcon }) => (
        <button
          key={value}
          type="button"
          role="radio"
          aria-checked={view === value}
          aria-label={label}
          className={`relative z-10 flex h-6 w-8 items-center justify-center rounded-md transition-colors ${view === value ? 'text-fg' : 'text-dim hover:text-fg'}`}
          onClick={() => { if (view !== value) onChange(value) }}
        >
          <ChoiceIcon className={`size-3.5 transition-transform ${view === value ? 'scale-[1.08]' : ''}`} aria-hidden="true" />
        </button>
      ))}
    </div>
  )
}
