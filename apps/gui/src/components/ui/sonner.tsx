import { Toaster as Sonner, type ToasterProps } from 'sonner'

type MaittyToasterProps = {
  position: NonNullable<ToasterProps['position']>
  theme: NonNullable<ToasterProps['theme']>
}

// The design puts a toast 96px below the top edge and 36px in from the side, clear of the title bar's controls.
const TOAST_OFFSETS = { top: 96, right: 36, bottom: 36, left: 36 }
// The newest toast is in front; older ones collapse behind it and fan out on hover.
const VISIBLE_TOASTS = 3

export function Toaster({ position, theme }: MaittyToasterProps) {
  return (
    <Sonner
      className="maitty-toaster"
      closeButton
      visibleToasts={VISIBLE_TOASTS}
      offset={TOAST_OFFSETS}
      mobileOffset={TOAST_OFFSETS}
      position={position}
      theme={theme}
      toastOptions={{ closeButtonAriaLabel: 'Dismiss notification' }}
    />
  )
}
