import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"
import { Slot } from "radix-ui"

/*
 * The design's Button (`Button/kind=…/size=…` in the chat handoff). Diverges from upstream shadcn: the variants are the
 * design's kinds, drawn from the role tokens in `tokens.css`, and the corners come from `--r-control`.
 *   primary  the theme's accent: the one main action on a surface
 *   needs    the needs-you colour: the main answer to something waiting on the person, and nothing else
 *   soft     a raised neutral: every other action beside a primary or needs one
 *   ghost    text only until hovered: quiet actions in a header or toolbar
 *   danger   a red word on a soft red tint: an action that ends or removes something
 *   link     an inline text link
 */
const buttonVariants = cva(
  "group/button inline-flex shrink-0 items-center justify-center gap-2 rounded-(--r-control) border whitespace-nowrap font-(family-name:--ui) outline-none select-none transition-[filter,background-color,color] duration-(--t-fast) ease-(--ease) focus-visible:shadow-[0_0_0_2px_var(--bg),0_0_0_4px_var(--focus)] disabled:cursor-not-allowed disabled:opacity-42 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-3.5",
  {
    variants: {
      variant: {
        primary: "border-(--accent) bg-(--accent) font-semibold text-(--on-accent) hover:enabled:brightness-108 active:enabled:bg-[color-mix(in_oklab,var(--accent)_80%,var(--bg))]",
        needs: "border-(--needs) bg-(--needs) font-semibold text-(--on-needs) hover:enabled:brightness-108 active:enabled:bg-[color-mix(in_oklab,var(--needs)_80%,var(--bg))]",
        soft: "border-(--line2) bg-(--raise) font-semibold text-(--fg) hover:enabled:bg-[color-mix(in_oklab,var(--raise)_88%,var(--fg))] active:enabled:bg-[color-mix(in_oklab,var(--raise)_80%,var(--bg))] aria-expanded:bg-(--sel)",
        ghost: "border-transparent bg-transparent font-medium text-(--fg2) hover:enabled:bg-(--hover) hover:enabled:text-(--fg) active:enabled:bg-(--sel) aria-expanded:bg-(--sel)",
        danger: "border-[color-mix(in_oklab,var(--err)_45%,transparent)] bg-[color-mix(in_oklab,var(--err)_10%,transparent)] font-semibold text-(--err) hover:enabled:bg-[color-mix(in_oklab,var(--err)_18%,transparent)]",
        link: "h-auto border-transparent px-0 font-medium text-(--link) underline-offset-2 hover:underline",
      },
      size: {
        sm: "h-6 px-2 text-xs",
        md: "h-[30px] px-3 text-[12.5px]",
        icon: "size-7 border-transparent px-0 text-(--muted) hover:enabled:text-(--fg) [&_svg:not([class*='size-'])]:size-[15px]",
      },
    },
    defaultVariants: {
      variant: "primary",
      size: "md",
    },
  }
)

/** The key that presses a button while its card is shown, drawn after the label. */
/** The names `aria-keyshortcuts` takes for the keys a keycap draws as a symbol. */
const KEY_NAME: Readonly<Record<string, string>> = { "⏎": "Enter", "↑": "ArrowUp", "↓": "ArrowDown", "Esc": "Escape" }

function KeyHint({ children }: { children: React.ReactNode }) {
  // The keycap is for the eye; the button's name stays its words, and `aria-keyshortcuts` names the key.
  return <kbd aria-hidden="true" className="rounded-(--r-badge) border border-current px-[5px] py-px font-(family-name:--mono) text-[10.5px] font-medium opacity-70">{children}</kbd>
}

function Button({
  className,
  variant = "primary",
  size = "md",
  asChild = false,
  keyHint,
  children,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean
    /** The key that also presses it, shown as a small keycap. */
    keyHint?: string | undefined
  }) {
  const Comp = asChild ? Slot.Root : "button"

  return (
    <Comp
      data-slot="button"
      data-variant={variant}
      data-size={size}
      className={cn(buttonVariants({ variant, size, className }))}
      aria-keyshortcuts={keyHint === undefined ? undefined : KEY_NAME[keyHint] ?? keyHint}
      {...props}
    >
      {asChild || keyHint === undefined ? children : <>{children}<KeyHint>{keyHint}</KeyHint></>}
    </Comp>
  )
}

/** A square button with only an icon: the label is its tooltip and its accessible name. */
function IconButton({ label, ...props }: Omit<React.ComponentProps<"button">, "aria-label" | "title"> & { label: string }) {
  return <Button type="button" variant="ghost" size="icon" aria-label={label} title={label} {...props} />
}

export { Button, IconButton, KeyHint, buttonVariants }
