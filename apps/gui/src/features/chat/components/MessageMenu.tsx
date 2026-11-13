import { useId } from 'react'
import { MoreHorizontal } from 'lucide-react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import type { MessageEvent } from '../chatEvents'
import type { EditFromHere } from '../editFromHere'

/** The actions on a message the person sent: Edit from here, greyed with its reason when it cannot be used now. */
export function MessageMenu({ message, editFromHere }: { message: MessageEvent; editFromHere: EditFromHere }) {
  const { disabled, pick } = editFromHere
  const why = useId()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button type="button" className="mx-msg-menu" aria-label="Message actions" title="Message actions">
          <MoreHorizontal className="size-[15px]" aria-hidden="true" />
        </button>
      </DropdownMenuTrigger>
      {/* Mounted in the window root, as the permission menu is, so it reads the theme's colours. */}
      <DropdownMenuContent align="end" container={document.querySelector<HTMLElement>('.mx-app')} className="mx-msg-menu-list text-fg">
        <DropdownMenuItem disabled={disabled !== null} {...(disabled === null ? {} : { 'aria-describedby': why })} onSelect={() => { pick(message) }}>Edit from here</DropdownMenuItem>
        {disabled !== null && <p id={why} className="mx-msg-menu-why">{disabled}</p>}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
