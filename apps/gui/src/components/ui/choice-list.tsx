import * as React from "react"
import { ToggleGroup as ToggleGroupPrimitive } from "radix-ui"

/**
 * A list of rows where one is current and picking a row acts at once (a model, a folder, a theme). Radix's
 * toggle group gives the radio roles, Tab as one stop, and the arrows, Home and End moving focus without
 * picking; Enter or Space on a row clicks it. Unlike a radio group, moving focus never picks, so a list that
 * closes or previews on pick is safe to arrow through. Rows may sit in any wrappers (sections) inside it.
 */
function ChoiceList({ current, className, children, ...props }: Omit<React.ComponentProps<typeof ToggleGroupPrimitive.Root>, "type" | "value" | "defaultValue" | "onValueChange"> & { current: string | null }) {
  return (
    <ToggleGroupPrimitive.Root data-slot="choice-list" type="single" value={current ?? ""} orientation="vertical" loop className={className} {...props}>
      {children}
    </ToggleGroupPrimitive.Root>
  )
}

/** One row. `onPick` runs on a click, Enter or Space, whether or not the row is the current one. */
function ChoiceItem({ onPick, ...props }: Omit<React.ComponentProps<typeof ToggleGroupPrimitive.Item>, "onClick"> & { onPick: () => void; onClick?: never }) {
  return <ToggleGroupPrimitive.Item data-slot="choice-item" onClick={() => { onPick() }} {...props} />
}

export { ChoiceItem, ChoiceList }
