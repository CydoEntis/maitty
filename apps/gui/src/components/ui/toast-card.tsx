import * as React from "react"
import { cn } from "@/lib/utils"

/**
 * The design's Toast: one line on the card colour with the pop shadow, a mark at the left and its actions at the
 * right. It is announced politely; where it sits and when it goes is the caller's.
 */
function ToastCard({ mark, actions, className, children }: { mark?: React.ReactNode; actions?: React.ReactNode; className?: string; children: React.ReactNode }) {
  return (
    <div role="status" data-slot="toast" className={cn("flex min-h-10 items-center gap-2.5 rounded-(--r-panel) border border-(--line2) bg-(--card) py-1.5 pr-2 pl-3 text-[13px] text-(--fg) shadow-(--sh-pop)", className)}>
      {mark}
      <span className="min-w-0 flex-1">{children}</span>
      {actions}
    </div>
  )
}

export { ToastCard }
