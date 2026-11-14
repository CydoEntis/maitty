import type { KeyboardEvent } from 'react'
import { Plus, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { DiffStat } from '@/components/DiffStat'
import { useReviewStat } from '@/features/review/useReviewStat'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import type { Conversation } from '@/data/preview'
import { type PanelTab, statusText } from './shared'

export function ConversationTabStrip({
  tabs,
  selectedId,
  onSelect,
  onClose,
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
        size="icon"
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
            <ContextMenuContent>
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
              variant={!showTerminal ? 'soft' : 'ghost'}
              size="sm"
              disabled={!showTerminal}
              onClick={onToggleTerminal}
              aria-pressed={!showTerminal}
              aria-label={showTerminal ? 'Return to chat' : 'Chat view'}
            >
              Chat
            </Button>
            <Button
              variant={showTerminal ? 'soft' : 'ghost'}
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
