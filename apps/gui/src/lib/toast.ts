import { createElement, type ReactNode } from 'react'
import { toast as sonner } from 'sonner'
import { DEFAULT_TOAST_MS, PlainToast, type ToastKind } from '@/features/notifications/PlainToast'

export type ToastAction = {
  label: string
  /** `event.preventDefault()` keeps the toast on screen after the click, as it does for a progress toast's Cancel. */
  onClick: (event: { preventDefault: () => void }) => void
}

export type ToastOptions = {
  /** A toast with this id is replaced in place. */
  id?: string | number | undefined
  /** Under the title: a line, or a small picture such as a progress bar. */
  description?: ReactNode | undefined
  /** Milliseconds; `Infinity` stays until it is replaced or dismissed. */
  duration?: number | undefined
  dismissible?: boolean | undefined
  closeButton?: boolean | undefined
  action?: ToastAction | undefined
}

function show(kind: ToastKind, title: string, options: ToastOptions = {}): string | number {
  const duration = options.duration ?? (kind === 'loading' ? Infinity : DEFAULT_TOAST_MS)
  return sonner.custom((id) => createElement(PlainToast, { id, kind, title, options }), { ...(options.id === undefined ? {} : { id: options.id }), duration })
}

/**
 * The app's toast: the same calls as sonner's (`toast.error(text, { description })`, `toast.success`, `toast.message`,
 * `toast.loading`, a bare `toast(text, { action })`), but each one is drawn with the notification card, the design every
 * toast of the app shares: a session that needs you, a finished chat, an Undo, a failure.
 */
export const toast = Object.assign(
  (title: string, options?: ToastOptions) => show('message', title, options),
  {
    message: (title: string, options?: ToastOptions) => show('message', title, options),
    success: (title: string, options?: ToastOptions) => show('success', title, options),
    error: (title: string, options?: ToastOptions) => show('error', title, options),
    warning: (title: string, options?: ToastOptions) => show('warning', title, options),
    loading: (title: string, options?: ToastOptions) => show('loading', title, options),
    custom: sonner.custom,
    dismiss: sonner.dismiss,
  },
)
