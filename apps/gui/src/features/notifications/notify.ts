import { createElement } from 'react'
import { toast } from 'sonner'
import { NoticeToast, type NoticeProps } from './NoticeToast'

/** Show a notification card. Sticky ones stay until answered or dismissed. */
export function showNotice(
  notice: NoticeProps,
  options: { sticky?: boolean } = {},
): string | number {
  const duration = options.sticky ? Infinity : notice.countdownMs
  return toast.custom(
    (id) =>
      createElement(NoticeToast, {
        ...notice,
        onDismiss: () => {
          toast.dismiss(id)
        },
      }),
    duration === undefined ? {} : { duration },
  )
}
