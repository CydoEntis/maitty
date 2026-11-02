import { useQuery } from '@tanstack/react-query'
import { isDesktop, loadConversationDiff } from '@/api/daemon'
import { previewDiff, type Conversation } from '@/data/preview'

export type ReviewStat = { files: number; additions: number; deletions: number }

const PREVIEW_STAT: ReviewStat = previewDiff.reduce<ReviewStat>(
  (total, file) => ({
    files: total.files + 1,
    additions: total.additions + file.added,
    deletions: total.deletions + file.removed,
  }),
  { files: 0, additions: 0, deletions: 0 },
)

/** How much a session changed, shared with the Changes tab through one cached query. */
export function useReviewStat(conversation: Conversation): ReviewStat | null {
  const desktop = isDesktop()
  const query = useQuery({
    queryKey: ['conversation-diff', conversation.id, conversation.generation],
    queryFn: () => loadConversationDiff(conversation),
    enabled:
      desktop &&
      conversation.isGitRepository !== false &&
      conversation.generation !== undefined,
    retry: false,
    retryOnMount: false,
    staleTime: 1500,
  })
  if (!desktop) return conversation.hasDiff ? PREVIEW_STAT : null
  if (!query.data || query.data.total_files === 0) return null
  return {
    files: query.data.total_files,
    additions: query.data.additions,
    deletions: query.data.deletions,
  }
}
