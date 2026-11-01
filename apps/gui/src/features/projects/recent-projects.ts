import type { Conversation } from '@/data/preview'

export type RecentProject = { name: string; folder: string; sessions: number }

export function recentProjectsFrom(
  conversations: readonly Conversation[],
): RecentProject[] {
  const byFolder = new Map<string, RecentProject>()
  for (const conversation of conversations) {
    const existing = byFolder.get(conversation.folder)
    if (existing) existing.sessions += 1
    else
      byFolder.set(conversation.folder, {
        name: conversation.project,
        folder: conversation.folder,
        sessions: 1,
      })
  }
  return Array.from(byFolder.values())
}
