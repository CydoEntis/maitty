import type { ComponentProps } from 'react'
import { Popover } from 'radix-ui'

/**
 * The popover every picker in the window opens: portalled into the window root, where the theme's
 * variables live, and drawn as a float of the current material (material.css). Use it instead of
 * styling a Radix popover by hand, so a menu looks the same in Glass and in Flat wherever it is.
 */
export function PopupContent({ className = '', children, ...props }: ComponentProps<typeof Popover.Content>) {
  return (
    <Popover.Portal container={document.querySelector<HTMLElement>('.mx-app')}>
      <Popover.Content sideOffset={8} {...props} className={`mx-pop z-50 rounded-[14px] p-2 ${className}`}>
        {children}
      </Popover.Content>
    </Popover.Portal>
  )
}
