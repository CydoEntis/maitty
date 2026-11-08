import * as React from "react"
import { cn } from "@/lib/utils"

/**
 * The results of a search box whose focus stays in the box: the arrows move `selected`, Enter picks, the pointer
 * hovers. It is one listbox of options and nothing more; keeping the keys is the caller's, since they live on the
 * input. (A menu or select that takes focus itself is Radix's, in dropdown-menu.tsx and select.tsx.)
 */
function ResultsList({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="results-list" role="listbox" className={className} {...props} />
}

function ResultsOption({ className, selected, ...props }: React.ComponentProps<"div"> & { selected: boolean }) {
  return <div data-slot="results-option" role="option" aria-selected={selected} className={cn(className)} {...props} />
}

export { ResultsList, ResultsOption }
