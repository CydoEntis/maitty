import * as React from "react"
import { Dialog as DialogPrimitive } from "radix-ui"
import { cn } from "@/lib/utils"

/**
 * A modal panel that stays where it is rendered (no portal), so the window's own tokens and positioning
 * context apply: Settings in the middle, the sessions drawer and the inbox at an edge. Radix gives it the
 * dialog role, the focus trap, focus returning to what had it, Esc and the click outside. What it looks like
 * is the two class names: the scrim behind and the panel itself.
 */
function Panel({
  label,
  onClose,
  scrimClassName,
  className,
  children,
  onEscapeKeyDown,
  ...props
}: Omit<React.ComponentProps<typeof DialogPrimitive.Content>, "onCloseAutoFocus"> & {
  /** The panel's name, read by assistive technology and by the tests. */
  label: string
  onClose: () => void
  scrimClassName?: string
}) {
  // The panel is unmounted by its owner to close it, so Radix has no trigger to give focus back to: the element
  // that had focus when it opened gets it again when it goes.
  const [opener] = React.useState(() => (document.activeElement instanceof HTMLElement ? document.activeElement : null))
  React.useEffect(() => () => { if (opener?.isConnected === true) opener.focus() }, [opener])
  return (
    <DialogPrimitive.Root open onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogPrimitive.Overlay data-slot="panel-scrim" className={scrimClassName}>
        <DialogPrimitive.Content
          data-slot="panel"
          aria-describedby={undefined}
          className={cn("outline-none", className)}
          onEscapeKeyDown={onEscapeKeyDown}
          {...props}
        >
          <DialogPrimitive.Title className="sr-only">{label}</DialogPrimitive.Title>
          {children}
        </DialogPrimitive.Content>
      </DialogPrimitive.Overlay>
    </DialogPrimitive.Root>
  )
}

export { Panel }
