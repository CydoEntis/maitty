import { displayName, truncatePrompt, type SessionView } from '@/features/workspace/model/session'
import type { SessionNoticeProps } from './SessionNotice'

/** Finished and failed cards go by themselves, with a thin line showing how long is left; one that needs you waits. */
export const SELF_DISMISS_MS = 7000

const LABEL = { needs: 'Needs you', done: 'Finished', error: 'Error' } as const

type Project = { name: string } | undefined
type Handlers = {
  open: () => void
  /** Types "yes, once" into the session's own prompt; null when there is nothing to allow. */
  allow: (() => void) | null
}

/** The in-window card for a session that just needed you, finished or failed. The card may show what it is asking: it is in your own window, unlike a system notification. */
export function sessionNoticeFor(session: SessionView, project: Project, handlers: Handlers): SessionNoticeProps | null {
  const kind = session.status
  if (kind !== 'needs' && kind !== 'done' && kind !== 'error') return null
  const where = `${session.process} in ${displayName(session)}`
  const body =
    kind === 'needs'
      ? session.asking === '' ? `${where} needs you` : `${where}: ${truncatePrompt(session.asking)}`
      : kind === 'done' ? `${where} finished` : `${where} exited with an error`
  const allow = kind === 'needs' && session.asking !== '' ? handlers.allow : null
  return {
    kind,
    title: `${LABEL[kind]} · ${project?.name ?? displayName(session)}`,
    body,
    actions: [...(allow ? [{ label: 'Allow once', primary: true, onClick: allow }] : []), { label: 'Open', onClick: handlers.open }],
    ...(kind === 'needs' ? {} : { countdownMs: SELF_DISMISS_MS }),
  }
}
