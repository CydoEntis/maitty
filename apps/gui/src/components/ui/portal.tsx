import type { ReactNode } from 'react'
import { Portal } from 'radix-ui'

/**
 * Draws its children in the window root, outside whatever clips or offsets the place they are written in.
 * For something that only shows (a card beside a row); anything that opens and takes focus is a Radix
 * popover, menu or dialog, which portal themselves.
 */
export function WindowPortal({ container, children }: { container: HTMLElement | null; children: ReactNode }) {
  return <Portal.Root container={container ?? document.querySelector<HTMLElement>('.mx-app')}>{children}</Portal.Root>
}
