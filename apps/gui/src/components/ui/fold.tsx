import * as React from "react"
import { cn } from "@/lib/utils"

/** The design's Fold: a full-width row at the foot of a card that shows or hides the rest of it. */
function Fold({ open, onToggle, more, less = "Hide", className }: {
  open: boolean
  onToggle: () => void
  /** What showing adds, as the closed row says it: "Show 12 more lines". */
  more: React.ReactNode
  less?: React.ReactNode
  className?: string
}) {
  return (
    <button
      type="button"
      data-slot="fold"
      aria-expanded={open}
      onClick={onToggle}
      className={cn("flex h-[30px] w-full items-center gap-[7px] border-t border-(--line) px-3 text-left text-xs font-medium text-(--muted) outline-none hover:bg-(--hover) hover:text-(--fg) focus-visible:shadow-[inset_0_0_0_2px_var(--focus)]", className)}
    >
      <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" className={cn("transition-transform duration-(--t-fast)", open && "rotate-90")}>
        <path d="m9 6 6 6-6 6" />
      </svg>
      {open ? less : more}
    </button>
  )
}

export { Fold }
