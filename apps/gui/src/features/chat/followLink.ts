import { isDesktop, openLink } from '@/api/daemon'
import { copyText } from '@/lib/clipboard'
import { errorMessage } from '@/lib/error-message'
import { toast } from '@/lib/toast'

function copyLink(href: string): void {
  copyText(href).then(
    () => { toast.message('Link copied') },
    () => { toast.error('Could not copy the link') },
  )
}

/**
 * Opens a web address in the person's own browser, through the host, which opens only http and https
 * addresses. Outside the desktop app, where there is no host to ask, it copies the address instead.
 */
export function followLink(href: string): void {
  if (!isDesktop()) {
    copyLink(href)
    return
  }
  openLink(href).then(
    () => { toast.message('Opened in your browser', { action: { label: 'Copy link', onClick: () => { copyLink(href) } } }) },
    (error: unknown) => { toast.error(errorMessage(error, 'Could not open the link'), { action: { label: 'Copy link', onClick: () => { copyLink(href) } } }) },
  )
}
