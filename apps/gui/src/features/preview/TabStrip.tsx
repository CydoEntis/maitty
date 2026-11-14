import { useState, type KeyboardEvent } from 'react'
import { Pencil, Plus, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { DiffStat } from '@/components/DiffStat'
import { useReviewStat } from '@/features/review/useReviewStat'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import type { Conversation } from '@/data/preview'
import { InlineName } from '@/features/workspace/components/InlineName'
import { MAX_SESSION_NAME_LENGTH } from '@/features/workspace/model/session'
import { type PanelTab, statusText } from './shared'

export function ConversationTabStrip({
  tabs,
  selectedId,
  onSelect,
  onClose,
  onRename,
  onNewTab,
  conversation,
  panel,
  showTerminal,
  onPanel,
  onToggleTerminal,
  showViewSwitch,
  threadsButton,
  onOpenThreads,
  attentionCount,
}: {
  tabs: Conversation[]
  selectedId: string
  onSelect: (id: string) => void
  onClose: (id: string) => void
  onRename: (id: string, title: string) => void
  onNewTab: () => void
  conversation: Conversation
  panel: PanelTab | null
  showTerminal: boolean
  onPanel: (tab: PanelTab | null) => void
  onToggleTerminal: () => void
  showViewSwitch: boolean
  threadsButton: boolean
  onOpenThreads: () => void
  attentionCount: number
}) {
  const reviewStat = useReviewStat(conversation)
  const [renamingId, setRenamingId] = useState<string | null>(null)

  function handleTabKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return
    const index = tabs.findIndex((tab) => tab.id === selectedId)
    const next = tabs[index + (event.key === 'ArrowRight' ? 1 : -1)]
    if (!next) return
    event.preventDefault()
    onSelect(next.id)
    requestAnimationFrame(() => {
      document
        .querySelector<HTMLElement>('.conversation-tab.is-active [role="tab"]')
        ?.focus()
    })
  }

  return (
    <div className="tab-strip">
      {threadsButton && (
        <Button variant="ghost" size="sm" onClick={onOpenThreads}>
          Threads
          {attentionCount > 0 && (
            <span className="threads-attention">
              <StatusMark status="needs-you" />
              {attentionCount}
            </span>
          )}
        </Button>
      )}
      <Button
        variant="ghost"
        size="icon-sm"
        aria-label="New tab"
        title="New conversation in a new tab (Ctrl+N)"
        onClick={onNewTab}
      >
        <Plus size={14} />
      </Button>
      <div
        className="conversation-tabs"
        role="tablist"
        aria-label="Open conversations"
        onKeyDown={handleTabKeyDown}
      >
        {tabs.map((tab) => (
          <ContextMenu key={tab.id}>
            <ContextMenuTrigger asChild>
              <div
                className={`conversation-tab ${tab.id === selectedId ? 'is-active' : ''} ${tab.status === 'working' && tab.id === selectedId ? 'is-shimmering' : ''}`}
                onAuxClick={(event) => {
                  if (event.button === 1 && tabs.length > 1) onClose(tab.id)
                }}
              >
                {renamingId === tab.id ? (
                  <div className="conversation-tab-rename">
                    <StatusMark status={tab.status} />
                    <InlineName
                      value={tab.title}
                      placeholder={tab.title}
                      label="Tab name"
                      maxLength={MAX_SESSION_NAME_LENGTH}
                      onCommit={(name) => {
                        setRenamingId(null)
                        // An empty name keeps the title: a conversation always has one.
                        if (name !== '' && name !== tab.title) onRename(tab.id, name)
                      }}
                      onCancel={() => {
                        setRenamingId(null)
                      }}
                    />
                  </div>
                ) : (
                <button
                  type="button"
                  role="tab"
                  aria-selected={tab.id === selectedId}
                  aria-controls="conversation-main"
                  tabIndex={tab.id === selectedId ? 0 : -1}
                  title={`${tab.title} · ${statusText[tab.status]}`}
                  onClick={() => {
                    onSelect(tab.id)
                  }}
                >
                  <StatusMark status={tab.status} />
                  <span>{tab.title}</span>
                  {tab.id === selectedId && (
                    <span
                      className="sr-only"
                      role="status"
                      aria-label={`Current status: ${statusText[tab.status]}`}
                    />
                  )}
                </button>
                )}
                <Tooltip>
                  <TooltipTrigger asChild>
                    <button
                      className="conversation-tab-close"
                      type="button"
                      aria-label={`Close ${tab.title} tab`}
                      disabled={tabs.length <= 1}
                      onClick={() => {
                        onClose(tab.id)
                      }}
                    >
                      <X size={12} />
                    </button>
                  </TooltipTrigger>
                  <TooltipContent>
                    Close tab · the session keeps running in maittyd
                  </TooltipContent>
                </Tooltip>
              </div>
            </ContextMenuTrigger>
            <ContextMenuContent
              onCloseAutoFocus={(event) => {
                // The name field has taken focus; handing it back to the tab would end the rename at once.
                if (renamingId !== null) event.preventDefault()
              }}
            >
              <ContextMenuItem
                onSelect={() => {
                  setRenamingId(tab.id)
                }}
              >
                <Pencil size={14} />
                Rename
              </ContextMenuItem>
              <ContextMenuItem
                disabled={tabs.length <= 1}
                onSelect={() => {
                  onClose(tab.id)
                }}
              >
                <X size={14} />
                Close tab
              </ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        ))}
      </div>
      <div className="header-toolbar">
        {showViewSwitch && (
          <div
            className="view-switch"
            role="group"
            aria-label="Conversation view"
          >
            <Button
              variant={!showTerminal ? 'secondary' : 'ghost'}
              size="sm"
              disabled={!showTerminal}
              onClick={onToggleTerminal}
              aria-pressed={!showTerminal}
              aria-label={showTerminal ? 'Return to chat' : 'Chat view'}
            >
              Chat
            </Button>
            <Button
              variant={showTerminal ? 'secondary' : 'ghost'}
              size="sm"
              disabled={showTerminal}
              onClick={onToggleTerminal}
              aria-pressed={showTerminal}
              aria-label="Show terminal"
            >
              Terminal
            </Button>
          </div>
        )}
        {conversation.kind !== 'terminal' && (
          <Button
            className="review-button"
            variant="ghost"
            size="sm"
            title="Changes, files and context (Ctrl+Shift+D)"
            aria-pressed={panel !== null}
            onClick={() => {
              onPanel(panel ? null : 'diff')
            }}
          >
            Review
            {reviewStat && (
              <DiffStat
                additions={reviewStat.additions}
                deletions={reviewStat.deletions}
              />
            )}
          </Button>
        )}
      </div>
    </div>
  )
}
