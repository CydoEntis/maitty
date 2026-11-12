import { useEffect, useState } from 'react'
import { agoWords, exitWords, type ChatProblem } from '../problem'

const TICK_MS = 5000

/** A banner that rises out of the message box when the chat has a problem: the agent stopped, its machine is out of reach, or its context is nearly full. */
export function ProblemBanner({ problem }: { problem: ChatProblem }) {
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
        <button type="button" className="mx-answer" onClick={problem.onViewLog}>View log</button>
        {problem.onRestart !== null && <button type="button" className="mx-answer is-y" onClick={problem.onRestart}>Restart</button>}
      </section>
    )
  }
  if (problem.kind === 'offline') {
    const ago = agoWords(problem.since, now)
    return (
      <section className="mx-problem is-warn" role="status" aria-label="Machine offline">
        <span className="mx-problem-tile" aria-hidden="true">!</span>
        <span className="mx-problem-text"><strong>{problem.machine} is offline</strong><small>{ago === null ? 'Lost the connection.' : `Lost the connection ${ago} ago.`} Trying again. The agent keeps running on {problem.machine}.</small></span>
        <button type="button" className="mx-answer is-y" onClick={problem.onRetry}>Retry now</button>
      </section>
    )
  }
  return (
    <section className="mx-problem is-warn" aria-label="Context almost full">
      <span className="mx-problem-tile" aria-hidden="true">!</span>
      <span className="mx-problem-text"><strong>Context is almost full · {problem.percent}%</strong><small>{problem.agent} will start forgetting the earliest messages soon.</small></span>
      <button type="button" className="mx-answer is-y" onClick={problem.onCompact}>Compact</button>
    </section>
  )
}
