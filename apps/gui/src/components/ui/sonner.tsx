import { Toaster as Sonner, type ToasterProps } from 'sonner'

type MaittyToasterProps = {
  position: NonNullable<ToasterProps['position']>
  theme: NonNullable<ToasterProps['theme']>
}

const TOAST_OFFSETS = { top: 48, right: 16, bottom: 36, left: 16 }

export function Toaster({ position, theme }: MaittyToasterProps) {
  return (
    <Sonner
      className="maitty-toaster"
      closeButton
      offset={TOAST_OFFSETS}
      mobileOffset={TOAST_OFFSETS}
      position={position}
      theme={theme}
      toastOptions={{ closeButtonAriaLabel: 'Dismiss notification' }}
    />
  )
}
