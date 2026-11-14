import * as React from "react"
import { cn } from "@/lib/utils"

type Tone = "working" | "done" | "err" | "needs"
const PERCENT = 100
const FILL: Record<Tone, string> = { working: "bg-(--working)", done: "bg-(--done)", err: "bg-(--err)", needs: "bg-(--needs)" }

/** The design's Progress: a 4px bar on the selected colour, filled in a status colour. */
function Progress({ value, max, tone = "working", label, className }: { value: number; max: number; tone?: Tone; label: string; className?: string }) {
  const share = max > 0 ? Math.min(1, Math.max(0, value / max)) : 0
  return (
    <span role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={max} aria-valuenow={value} data-slot="progress" className={cn("block h-1 bg-(--sel)", className)}>
      <span className={cn("block h-full transition-[width] duration-(--t-base) ease-(--ease)", FILL[tone])} style={{ width: `${String(share * PERCENT)}%` }} />
    </span>
  )
}

/** A quiet count in mono: "2 of 5", "+3". */
function Counter({ className, ...props }: React.ComponentProps<"span">) {
  return <span data-slot="counter" className={cn("font-(family-name:--mono) text-[11px] font-medium text-(--dim) tabular-nums", className)} {...props} />
}

/** A hairline between groups. */
function Divider({ className }: { className?: string }) {
  return <span role="separator" data-slot="divider" className={cn("block h-px w-full bg-(--line)", className)} />
}

export { Progress, Counter, Divider }
