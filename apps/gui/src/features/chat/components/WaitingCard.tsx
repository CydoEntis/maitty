import type { KeyboardEvent, ReactNode } from 'react'
import { cardVariants } from '@/components/ui/card'
import { StatusMark } from '@/components/ui/status-mark'
import { cn } from '@/lib/utils'

type Props = {
  /** The card's accessible name. */
  label: string
  title: ReactNode
  /** Quiet words at the right of the head: the agent's name, "1 of 3", "Plan mode". */
  aside?: ReactNode
  children?: ReactNode
  /** The body has no padding of its own, for a well (a command, a menu) that sits flush in the card. */
  flush?: boolean
  /** The answers, the main one first. */
  foot?: ReactNode
  /** What Enter does while the card itself has the keys: its main answer. Absent where there is none. */
  onEnter?: (() => void) | undefined
}

/**
 * Something the agent is waiting on the person for, rising out of the top of the message box (`WaitingCard`): an
 * amber-edged card open at the bottom, a head with the needs mark and what is asked, the detail, and the answers in
 * its foot. Only one shows at a time; the chat keeps the rest in order behind it.
 */
export function WaitingCard({ label, title, aside, children, flush = false, foot, onEnter }: Props) {
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    // Only on the card itself: a focused button already presses with Enter, and a field's Enter is its own.
    if (onEnter === undefined || event.key !== 'Enter' || event.target !== event.currentTarget) return
    event.preventDefault()
    onEnter()
  }
  return (
    <section
      aria-label={label}
      tabIndex={onEnter === undefined ? undefined : 0}
      aria-keyshortcuts={onEnter === undefined ? undefined : 'Enter'}
      className={cn(cardVariants({ tone: 'needs' }), 'mx-waiting')}
      onKeyDown={onKeyDown}
    >
      <div className="mx-waiting-head">
        <StatusMark state="needs" />
        <span className="mx-waiting-title">{title}</span>
        <span className="mx-spacer" />
        {aside !== undefined && <span className="mx-waiting-aside">{aside}</span>}
      </div>
      {children !== undefined && children !== null && children !== false && <div className={flush ? 'mx-waiting-body is-flush' : 'mx-waiting-body'}>{children}</div>}
      {foot !== undefined && foot !== null && foot !== false && <div className="mx-waiting-foot">{foot}</div>}
    </section>
  )
}
