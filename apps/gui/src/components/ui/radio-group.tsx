import * as React from "react"
import { RadioGroup as RadioGroupPrimitive } from "radix-ui"
import { cn } from "@/lib/utils"

/** One choice out of several. Radix gives it the arrow keys, Tab as one stop, and the radio roles; what it looks like is the className. */
function RadioGroup({ className, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Root>) {
  return <RadioGroupPrimitive.Root data-slot="radio-group" className={className} {...props} />
}

function RadioGroupItem({ className, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Item>) {
  return <RadioGroupPrimitive.Item data-slot="radio-group-item" className={className} {...props} />
}

/** The segmented control: a row of buttons, one picked, on the window's track and selection tokens. */
function Segmented({ className, tall = false, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Root> & { tall?: boolean }) {
  return <RadioGroupPrimitive.Root data-slot="segmented" orientation="horizontal" className={cn("mx-segmented", tall && "is-tall", className)} {...props} />
}

function SegmentedItem({ className, ...props }: React.ComponentProps<typeof RadioGroupPrimitive.Item>) {
  return <RadioGroupPrimitive.Item data-slot="segmented-item" className={className} {...props} />
}

export { RadioGroup, RadioGroupItem, Segmented, SegmentedItem }
