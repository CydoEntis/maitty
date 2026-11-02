import { Toaster as Sonner, type ToasterProps } from 'sonner'

type MaittyToasterProps = {
  position: NonNullable<ToasterProps['position']>
  theme: NonNullable<ToasterProps['theme']>
}

const TOAST_OFFSETS = { top: 16, right: 16, bottom: 16, left: 16 }
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
