import type { MouseEvent } from 'react'
import type { Conversation, SessionStatus } from '@/data/preview'

export const OTHER_FOLDERS = 'Other folders'

/** The sidebar group a terminal session belongs to. */
export function sessionGroup(conversation: Conversation): string {
  return conversation.repo ?? OTHER_FOLDERS
}

export function focusMainContent(event: MouseEvent<HTMLAnchorElement>) {
  event.preventDefault()
  document.getElementById('conversation-main')?.focus()
}

export const PREVIEW_RESPONSE_DELAY_MS = 1500

export const PREVIEW_APPROVAL_DELAY_MS = 1700

export const PERMISSION_JSON_INDENT_SPACES = 2

export const SESSION_REFRESH_INTERVAL_MS = 1500

export const HISTORY_REFRESH_INTERVAL_MS = 2500

export const APPROVAL_COMPACT_HEIGHT_PX = 760

export const COMPACT_SIDEBAR_MAX_WIDTH_PX = 1099

export const RECENT_PROJECT_LIMIT = 5

export const ACTIVITY_PREVIEW_LINE_COUNT = 3

export const CODE_FENCE_MARKER = '```'

export type PanelTab = 'details' | 'diff' | 'files'

export type SendFailure = { kind: 'not-delivered' | 'unknown'; operation: number }

export const statusText: Record<SessionStatus, string> = {
  working: 'Working',
  'needs-you': 'Needs you',
  done: 'Done',
  idle: 'Idle',
  recovery: 'Recovery needed',
  unknown: 'Disconnected',
}
