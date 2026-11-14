import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

/*
 * The design's Card (`Card · shell, head, body, foot` in the chat handoff): a solid card with a hairline and no shadow.
 * Every card in the feed is one of these; buttons belong in its head or foot. `tone` is the edge: `needs` for something
 * waiting on the person, `err` for something that failed. A card that is itself clickable (`interactive`) shows its
 * edge in `--line2` on hover and the focus ring outside it.
 */
const cardVariants = cva(
  "flex min-w-0 flex-col rounded-(--r-panel) border bg-(--card) text-(--fg)",
  {
    variants: {
      tone: {
        none: "border-(--line)",
        needs: "border-(--needs)",
        err: "border-[color-mix(in_oklab,var(--err)_55%,var(--line))]",
      },
      interactive: {
        true: "cursor-pointer outline-none transition-[border-color] duration-(--t-fast) hover:border-(--line2) focus-visible:shadow-[0_0_0_2px_var(--bg),0_0_0_4px_var(--focus)]",
        false: "",
      },
    },
    defaultVariants: { tone: "none", interactive: false },
  }
)

function Card({ className, tone, interactive, ...props }: React.ComponentProps<"div"> & VariantProps<typeof cardVariants>) {
  return <div data-slot="card" className={cn(cardVariants({ tone, interactive }), className)} {...props} />
}

/** The head strip: a mark and title at the left, its buttons at the right. */
function CardHead({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-head" className={cn("flex min-h-9 min-w-0 items-center gap-2 border-b border-(--line) py-0 pr-1.5 pl-3 text-[12.5px] font-semibold last:border-b-0", className)} {...props} />
}

function CardBody({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-body" className={cn("min-w-0 px-3 py-2.5", className)} {...props} />
}

/** The foot: the card's answers, the main one first. */
function CardFoot({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="card-foot" className={cn("flex min-h-10 flex-wrap items-center gap-1.5 border-t border-(--line) py-1.5 pr-2 pl-3", className)} {...props} />
}

export { Card, CardHead, CardBody, CardFoot, cardVariants }
