import * as React from "react"
import { Popover as PopoverPrimitive } from "radix-ui"
import { cn } from "@/lib/utils"

/** A panel anchored to a button. Radix gives it focus handling, Esc, outside click and flipping where there is no room. */
function Popover(props: React.ComponentProps<typeof PopoverPrimitive.Root>) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />
}

function PopoverTrigger(props: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />
}

function PopoverAnchor(props: React.ComponentProps<typeof PopoverPrimitive.Anchor>) {
  return <PopoverPrimitive.Anchor data-slot="popover-anchor" {...props} />
}

function PopoverContent({ className, align = "start", sideOffset = 6, container, ...props }: React.ComponentProps<typeof PopoverPrimitive.Content> & { container?: HTMLElement | null }) {
  return (
    <PopoverPrimitive.Portal container={container}>
      <PopoverPrimitive.Content data-slot="popover-content" align={align} sideOffset={sideOffset} className={cn("mx-pop z-50 rounded-xl p-2 text-fg outline-hidden", className)} {...props} />
    </PopoverPrimitive.Portal>
  )
}

export { Popover, PopoverAnchor, PopoverContent, PopoverTrigger }
