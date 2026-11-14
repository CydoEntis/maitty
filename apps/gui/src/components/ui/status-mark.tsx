import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

/**
 * The states a step, run or helper can be in. Each has its own shape, so none reads by colour alone:
 * running a spinner, done ✓, failed ×, stopped ■, waiting ·, needs you ! on the needs square.
 */
export type MarkState = "running" | "done" | "failed" | "stopped" | "waiting" | "needs"

const WORD: Record<MarkState, string> = { running: "running", done: "done", failed: "failed", stopped: "stopped", waiting: "waiting", needs: "needs you" }
const WORD_COLOUR: Record<MarkState, string> = { running: "text-(--working)", done: "text-(--done)", failed: "text-(--err)", stopped: "text-(--muted)", waiting: "text-(--muted)", needs: "text-(--needs)" }

/** The 14px mark alone. Its word is its accessible name, so a screen reader hears the state. */
function Mark({ state, spinner, className }: { state: MarkState; spinner?: ReactNode; className?: string | undefined }) {
  const box = "flex size-3.5 flex-none items-center justify-center"
  return (
    <span role="img" aria-label={WORD[state]} data-slot="status-mark" data-state={state} className={cn("inline-flex", className)}>
      {state === "running" && (
        <span aria-hidden="true" className={cn(box, "text-(--working)")}>
          {spinner ?? <span className="box-border size-2.5 animate-spin rounded-[1px] border-[1.5px] border-(--line2) border-t-(--working) border-r-(--working) [animation-duration:var(--t-spin)]" />}
        </span>
      )}
      {state === "done" && <span aria-hidden="true" className={cn(box, "font-(family-name:--mono) text-xs leading-none font-bold text-(--done)")}>✓</span>}
      {state === "failed" && <span aria-hidden="true" className={cn(box, "font-(family-name:--mono) text-xs leading-none font-bold text-(--err)")}>×</span>}
      {state === "stopped" && <span aria-hidden="true" className={box}><span className="size-[7px] bg-(--muted)" /></span>}
      {state === "waiting" && <span aria-hidden="true" className={cn(box, "font-(family-name:--mono) text-base leading-none font-bold text-(--dim)")}>·</span>}
      {state === "needs" && <span aria-hidden="true" className={cn(box, "rounded-(--r-badge) bg-(--needs) font-(family-name:--mono) text-[10px] leading-none font-extrabold text-(--on-needs)")}>!</span>}
    </span>
  )
}

/**
 * The design's StatusMark: the mark, and with `word` the state's word beside it in its colour. A string `word` replaces
 * the default word ("3 of 5", "waiting for you"). `spinner` replaces the running square with the spinner the person
 * picked in Settings.
 */
function StatusMark({ state, word = false, spinner, className }: { state: MarkState; word?: boolean | string; spinner?: ReactNode; className?: string | undefined }) {
  if (word === false) return <Mark state={state} spinner={spinner} className={className} />
  return (
    <span className={cn("inline-flex items-center gap-1.5", className)}>
      <Mark state={state} spinner={spinner} />
      <span className={cn("text-xs font-semibold", WORD_COLOUR[state])}>{word === true ? WORD[state] : word}</span>
    </span>
  )
}

export { StatusMark, WORD as MARK_WORD }
