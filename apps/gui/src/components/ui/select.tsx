import * as React from "react"
import { Select as SelectPrimitive } from "radix-ui"
import { CheckIcon, ChevronDownIcon } from "lucide-react"
import { cn } from "@/lib/utils"

/** The dropdown that picks one value. Radix gives it the listbox roles, the arrow keys, Home and End, typing to jump and Esc. */
function Select(props: React.ComponentProps<typeof SelectPrimitive.Root>) {
  return <SelectPrimitive.Root data-slot="select" {...props} />
}

function SelectGroup(props: React.ComponentProps<typeof SelectPrimitive.Group>) {
  return <SelectPrimitive.Group data-slot="select-group" {...props} />
}

function SelectValue(props: React.ComponentProps<typeof SelectPrimitive.Value>) {
  return <SelectPrimitive.Value data-slot="select-value" {...props} />
}

/** The button that shows the choice; its content is the caller's (a picture, a name and a line) and the chevron is added. */
function SelectTrigger({ className, children, ...props }: React.ComponentProps<typeof SelectPrimitive.Trigger>) {
  return (
    <SelectPrimitive.Trigger data-slot="select-trigger" className={cn("mx-picker-button", className)} {...props}>
      {children}
      <SelectPrimitive.Icon asChild>
        <ChevronDownIcon className="mx-picker-chevron size-4" aria-hidden="true" />
      </SelectPrimitive.Icon>
    </SelectPrimitive.Trigger>
  )
}

function SelectContent({ className, children, container, position = "popper", ...props }: React.ComponentProps<typeof SelectPrimitive.Content> & { container?: HTMLElement | null }) {
  return (
    <SelectPrimitive.Portal container={container}>
      <SelectPrimitive.Content
        data-slot="select-content"
        position={position}
        sideOffset={6}
        className={cn("mx-pop z-50 max-h-(--radix-select-content-available-height) min-w-(--radix-select-trigger-width) overflow-hidden rounded-xl p-1.5 text-fg", className)}
        {...props}
      >
        <SelectPrimitive.Viewport className="max-h-[min(340px,var(--radix-select-content-available-height))] overflow-y-auto">{children}</SelectPrimitive.Viewport>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  )
}

function SelectLabel({ className, ...props }: React.ComponentProps<typeof SelectPrimitive.Label>) {
  return <SelectPrimitive.Label data-slot="select-label" className={cn("px-1.5 pt-1.5 pb-1 text-[10.5px] font-semibold tracking-wider text-dim uppercase", className)} {...props} />
}

function SelectSeparator({ className, ...props }: React.ComponentProps<typeof SelectPrimitive.Separator>) {
  return <SelectPrimitive.Separator data-slot="select-separator" className={cn("my-1 h-px bg-pane-line", className)} {...props} />
}

/** One choice. `children` is what the list shows; `text` is what the button shows and what typing a name matches. */
function SelectItem({ className, children, text, ...props }: React.ComponentProps<typeof SelectPrimitive.Item> & { text?: string }) {
  return (
    <SelectPrimitive.Item
      data-slot="select-item"
      className={cn("relative flex min-h-10 cursor-default items-center gap-2.5 rounded-lg py-1.5 pr-8 pl-1.5 text-[12.5px] outline-hidden select-none focus:bg-hover data-[state=checked]:bg-accent-soft", className)}
      {...props}
    >
      <SelectPrimitive.ItemText className="sr-only">{text ?? ""}</SelectPrimitive.ItemText>
      {children}
      <SelectPrimitive.ItemIndicator className="absolute right-2 text-brand">
        <CheckIcon className="size-4" aria-hidden="true" />
      </SelectPrimitive.ItemIndicator>
    </SelectPrimitive.Item>
  )
}

export { Select, SelectContent, SelectGroup, SelectItem, SelectLabel, SelectSeparator, SelectTrigger, SelectValue }
