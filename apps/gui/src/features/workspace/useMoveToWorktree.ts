import { useCallback, useEffect, useRef } from 'react'
import { addWorktree, folderBranches } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'
import { typeMessage } from '@/features/chat/useChatComposer'
import { EXIT_POLL_MS, EXIT_WAIT_MS, moveRefusal, resumeLine, worktreeBranch } from './model/moveToWorktree'
import { isShell } from './model/sidebar'
import type { SessionView } from './model/session'

const IS_WINDOWS = /windows/i.test(typeof navigator === 'undefined' ? '' : navigator.userAgent)

const pause = (ms: number) => new Promise<void>((resolve) => { window.setTimeout(resolve, ms) })

/**
 * Moves a chat to a new worktree: makes the worktree, has the agent exit, then resumes the same
 * conversation from the new folder in the same pane. The pane and its daemon session stay; only the
 * agent restarts, so it is offered only while the agent is idle. Uncommitted edits stay in the old folder.
 */
export function useMoveToWorktree({ sessionMap, sendTo }: {
  sessionMap: Map<string, SessionView>
  sendTo: (target: SessionView, data: string) => Promise<void>
}) {
  const latest = useRef(sessionMap)
  useEffect(() => { latest.current = sessionMap }, [sessionMap])

  return useCallback(async (session: SessionView) => {
    const refusal = moveRefusal(session)
    if (refusal !== null) { toast.error(refusal); return }
    const agent = session.process === 'codex' ? 'codex' : 'claude'
    const id = session.agentSession ?? ''
    try {
      const known = await folderBranches(session.cwd)
      const taken = [...(known?.branches.map((b) => b.name) ?? []), ...(known?.worktrees.flatMap((w) => (w.branch === null ? [] : [w.branch])) ?? [])]
      const folder = await addWorktree(session.cwd, worktreeBranch(taken))
      const line = resumeLine(agent, id, folder, IS_WINDOWS)
      if (line === null) throw new Error('The new worktree folder has a name that cannot be typed safely. It was made, but the chat was not moved.')
      await typeMessage((text) => sendTo(session, text), agent, '/exit')
      const deadline = Date.now() + EXIT_WAIT_MS
      for (;;) {
        const now = latest.current.get(session.id)
        if (now !== undefined && isShell(now)) break
        if (Date.now() > deadline) throw new Error(`The worktree was made at ${folder}, but the agent did not exit. Resume it there yourself.`)
        await pause(EXIT_POLL_MS)
      }
      await sendTo(session, `${line}\r`)
      toast.success('Moved to a new worktree. Edits not yet committed stayed in the old folder.')
    } catch (error: unknown) {
      toast.error(errorMessage(error, 'Could not move the chat to a worktree'))
    }
  }, [sendTo])
}
