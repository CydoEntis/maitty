export const CONVERSATION_DIFF_REFRESH_INTERVAL_MS = 5000

export function conversationDiffRefreshInterval(
  desktop: boolean,
  status: 'error' | 'pending' | 'success',
): number | false {
  return desktop && status === 'success'
    ? CONVERSATION_DIFF_REFRESH_INTERVAL_MS
    : false
}
