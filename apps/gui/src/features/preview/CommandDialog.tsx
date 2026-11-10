import { type KeyboardEvent, type ReactNode, useState } from 'react'
import { Command, FileCode2, Keyboard, MessageSquarePlus, PanelRight, Search, Settings2, TerminalSquare } from 'lucide-react'
import { StatusMark } from '@/components/StatusMark'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import type { Conversation } from '@/data/preview'
import { type PanelTab, statusText } from './shared'

export function CommandDialog({
  open,
  conversations,
  onClose,
  onSelect,
  onNew,
  onSettings,
  onKeyboard,
  onPanel,
  primaryModifier,
  onSelectInTerminal,
}: {
  open: boolean
  conversations: Conversation[]
  onClose: () => void
  onSelect: (id: string) => void
  onNew: () => void
  onSettings: () => void
  onKeyboard: () => void
  onPanel: (tab: PanelTab | 'terminal') => void
  primaryModifier: '⌘' | 'Ctrl'
  onSelectInTerminal: (id: string) => void
}) {
  const [query, setQuery] = useState('')
  const matchingConversations = conversations
    .filter((conversation) =>
      `${conversation.title} ${conversation.project} ${conversation.status}`
        .toLowerCase()
        .includes(query.trim().toLowerCase()),
    )
    .sort(
      (left, right) =>
        Number(right.status === 'needs-you' || right.status === 'recovery') -
        Number(left.status === 'needs-you' || left.status === 'recovery'),
    )
  const actionQuery = query.trim().toLowerCase()
  const actions = [
    {
      icon: <MessageSquarePlus />,
      title: 'New conversation',
      searchText: 'new conversation',
      onClick: onNew,
    },
    {
      icon: <FileCode2 />,
      title: 'Show changes',
      searchText: 'changes diff',
      onClick: () => {
        onPanel('diff')
      },
    },
    {
      icon: <PanelRight />,
      title: 'Show conversation details',
      searchText: 'details context',
      onClick: () => {
        onPanel('details')
      },
    },
    {
      icon: <TerminalSquare />,
      title: 'Show native terminal',
      searchText: 'terminal',
      onClick: () => {
        onPanel('terminal')
      },
    },
    {
      icon: <Settings2 />,
      title: 'Open settings',
      searchText: 'settings theme appearance notifications daemon providers',
      onClick: onSettings,
    },
    {
      icon: <Keyboard />,
      title: 'Keyboard shortcuts',
      searchText: 'keyboard shortcuts help keys',
      onClick: onKeyboard,
    },
  ].filter((action) => !actionQuery || action.searchText.includes(actionQuery))

  function moveFocus(event: KeyboardEvent<HTMLDivElement>) {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      const focusedItem = document.activeElement?.closest<HTMLButtonElement>(
        '.command-item[data-session-id]',
      )
      if (!focusedItem) return
      event.preventDefault()
      onSelectInTerminal(focusedItem.dataset.sessionId ?? '')
      return
    }
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
    const buttons = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>('.command-item'),
    )
    if (buttons.length === 0) return
    event.preventDefault()
    const currentIndex = buttons.findIndex(
      (button) => button === document.activeElement,
    )
    const step = event.key === 'ArrowDown' ? 1 : -1
    const nextIndex =
      currentIndex === -1
        ? step === 1
          ? 0
          : buttons.length - 1
        : (currentIndex + step + buttons.length) % buttons.length
    buttons[nextIndex]?.focus()
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="command-dialog" onKeyDown={moveFocus}>
        <DialogHeader>
          <DialogTitle className="sr-only">Commands</DialogTitle>
          <DialogDescription className="sr-only">
            Choose an action
          </DialogDescription>
        </DialogHeader>
        <div className="command-search">
          <Search size={16} />
          <input
            autoFocus
            aria-label="Search actions and sessions"
            placeholder="Search threads and actions"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value)
            }}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="command-list">
          {matchingConversations.length > 0 && (
            <>
              <span className="command-group-title">Threads</span>
              {matchingConversations.map((conversation) => (
                <CommandItem
                  key={conversation.id}
                  sessionId={conversation.id}
                  icon={<StatusMark status={conversation.status} />}
                  title={conversation.title}
                  meta={`${conversation.project} · ${statusText[conversation.status]}`}
                  onClick={() => {
                    onSelect(conversation.id)
                  }}
                />
              ))}
            </>
          )}
          {actions.length > 0 && (
            <>
              <span className="command-group-title">Actions</span>
              {actions.map((action) => (
                <CommandItem
                  key={action.title}
                  icon={action.icon}
                  title={action.title}
                  onClick={() => {
                    action.onClick()
                    onClose()
                  }}
                />
              ))}
            </>
          )}
          {matchingConversations.length === 0 && actions.length === 0 && (
            <p className="command-empty">No matching threads or actions.</p>
          )}
        </div>
        <div className="command-footer">
          <Command size={13} />
          Navigate <kbd>↑</kbd>
          <kbd>↓</kbd>
          <span>Choose</span>
          <kbd>↵</kbd>
          <span>Terminal</span>
          <kbd>{primaryModifier}+↵</kbd>
          <span>Close</span>
          <kbd>Esc</kbd>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function CommandItem({
  icon,
  title,
  shortcut,
  meta,
  sessionId,
  onClick,
}: {
  icon: ReactNode
  title: string
  shortcut?: string
  meta?: string
  sessionId?: string
  onClick: () => void
}) {
  return (
    <button
      className="command-item"
      type="button"
      data-session-id={sessionId}
      onClick={onClick}
    >
      {icon}
      <span>{title}</span>
      {meta && <small>{meta}</small>}
      {shortcut && <kbd>{shortcut}</kbd>}
    </button>
  )
}
