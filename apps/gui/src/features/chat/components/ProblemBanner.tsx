import { Button } from '@/components/ui/button'
import { useEffect, useState } from 'react'
import { agoWords, exitWords, type ChatProblem } from '../problem'

const TICK_MS = 5000

/** A banner that rises out of the message box when the chat has a problem: the agent stopped, its machine is out of reach, or its context is nearly full. */
export function ProblemBanner({ problem, summarize }: { problem: ChatProblem; /** Writes the summary a new chat starts from. */ summarize?: () => string }) {
  const [now, setNow] = useState(() => Date.now())
  const timed = problem.kind === 'offline'
  useEffect(() => {
    if (!timed) return undefined
    const timer = window.setInterval(() => { setNow(Date.now()) }, TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [timed])
  if (problem.kind === 'crashed') {
    return (
      <section className="mx-problem is-danger" aria-label="Agent stopped">
        <span className="mx-problem-tile" aria-hidden="true">×</span>
        <span className="mx-problem-text"><strong>{problem.agent} stopped unexpectedly</strong><small>{exitWords(problem.code)} Your branch and changes are kept.</small></span>
        <Button size="sm" variant="soft" onClick={problem.onViewLog}>View log</Button>
        {problem.onRestart !== null && <Button size="sm" onClick={problem.onRestart}>Restart</Button>}
      </section>
    )
  }
  if (problem.kind === 'offline') {
    const ago = agoWords(problem.since, now)
    return (
      <section className="mx-problem is-warn" role="status" aria-label="Machine offline">
        <span className="mx-problem-tile" aria-hidden="true">!</span>
        <span className="mx-problem-text"><strong>{problem.machine} is offline</strong><small>{ago === null ? 'Lost the connection.' : `Lost the connection ${ago} ago.`} Trying again. The agent keeps running on {problem.machine}.</small></span>
        <Button size="sm" variant="soft" onClick={problem.onRetry}>Retry now</Button>
      </section>
    )
  }
  return (
    <section className="mx-problem is-warn" aria-label="Context almost full">
      <span className="mx-problem-tile" aria-hidden="true">!</span>
      <span className="mx-problem-text"><strong>Context is almost full · {problem.percent}%</strong><small>{problem.agent} will start forgetting the earliest messages soon.</small></span>
      {summarize !== undefined && <Button size="sm" variant="soft" onClick={() => { problem.onNewWithSummary(summarize()) }}>New chat with summary</Button>}
      <Button size="sm" onClick={problem.onCompact}>Compact</Button>
    </section>
  )
}
