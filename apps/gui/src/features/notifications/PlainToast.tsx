import { toast as sonner } from 'sonner'
import { StatusMark, type StatusMarkState } from '@/components/StatusMark'
import { Spinner } from '@/features/workspace/components/glyphs'
import type { ToastOptions } from '@/lib/toast'
import { NotificationCard, type NotificationTone } from './NotificationCard'

export type ToastKind = 'message' | 'success' | 'error' | 'warning' | 'loading'

const LOOK: Record<Exclude<ToastKind, 'loading'>, { tone: NotificationTone; mark: StatusMarkState }> = {
  message: { tone: 'info', mark: 'idle' },
  success: { tone: 'done', mark: 'done' },
  error: { tone: 'error', mark: 'recovery' },
  warning: { tone: 'needs', mark: 'needs-you' },
}

/** How long a toast stays when it is not told: sonner's own default. */
export const DEFAULT_TOAST_MS = 4000

/** A plain toast drawn as the notification card, the one design every toast shares. */
export function PlainToast({ id, kind, title, options }: { id: string | number; kind: ToastKind; title: string; options: ToastOptions }) {
  const { action, description, closeButton, dismissible } = options
  const duration = options.duration ?? (kind === 'loading' ? Infinity : DEFAULT_TOAST_MS)
  const look = kind === 'loading' ? { tone: 'info' as const } : LOOK[kind]
  const icon = kind === 'loading' ? <Spinner /> : <StatusMark status={LOOK[kind].mark} />
  let kept = false
  return (
    <NotificationCard
      tone={look.tone}
      icon={<span className="mx-notice-mark">{icon}</span>}
      title={title}
      {...(description === undefined || description === null ? {} : { body: description })}
      actions={action === undefined ? [] : [{ label: action.label, primary: true, onClick: () => { action.onClick({ preventDefault: () => { kept = true } }) }, keepOpen: () => kept }]}
      time="just now"
      closable={closeButton !== false && dismissible !== false}
      onClose={() => { sonner.dismiss(id) }}
      {...(Number.isFinite(duration) ? { countdownMs: duration } : {})}
    />
  )
}

