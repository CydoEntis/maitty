import { useState } from 'react'
import { Check, FolderPlus, PanelLeft, Pencil, PenLine, Pin, Plus, Search, Settings2, Square, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { StatusMark } from '@/components/StatusMark'
import { Elapsed } from '@/components/Elapsed'
import { ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger } from '@/components/ui/context-menu'
import { useWorkingSince } from '@/lib/working-since'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import type { Conversation } from '@/data/preview'
import { OTHER_FOLDERS, statusText } from './shared'

type SidebarProps = {
  conversations: Conversation[]
  selectedId: string
  filter: string
  pinnedIds: string[]
  compact: boolean
  desktopCollapsed: boolean
  disconnected: boolean
  drawerOpen: boolean
  onFilter: (value: string) => void
  onNew: () => void
  onSelect: (id: string) => void
  onTogglePin: (id: string) => void
  onArchive: (id: string, archived: boolean) => void
  onRename: (id: string, title: string) => void
  onEnd: (id: string) => void
  primaryModifier: '⌘' | 'Ctrl'
  starting: { key: string; label: string; startedAt: number }[]
  onOpenProject: () => void
  onNewInProject: (folder: string) => void
  daemonLabel: string
  sessionCount: number
  previewMode: boolean
  onOpenSettings: () => void
  onToggleDrawer: () => void
  onToggleCollapse: () => void
}

export function ConversationSidebar({
  conversations,
  selectedId,
  filter,
  pinnedIds,
  compact,
  desktopCollapsed,
  disconnected,
  drawerOpen,
  onFilter,
  onNew,
  onSelect,
  onTogglePin,
  onArchive,
  onRename,
  onEnd,
  onToggleDrawer,
  onToggleCollapse,
  primaryModifier,
  starting,
  onOpenProject,
  onNewInProject,
  daemonLabel,
  sessionCount,
  previewMode,
  onOpenSettings,
}: SidebarProps) {
  const projects = Array.from(
    new Set(conversations.map((conversation) => conversation.project)),
  ).sort((left, right) => {
    // Sessions outside any repository sit under Other folders, last.
    if (left === OTHER_FOLDERS) return 1
    if (right === OTHER_FOLDERS) return -1
    return left.localeCompare(right)
  })
  const rowActions = {
    pinnedIds,
    selectedId,
    onSelect,
    onTogglePin,
    onArchive,
    onRename,
    onEnd,
  }

  return (
    <aside
      className={`session-sidebar ${compact ? 'is-compact' : ''} ${desktopCollapsed ? 'is-desktop-collapsed' : ''} ${drawerOpen ? 'is-drawer-open' : ''}`}
      aria-label="Conversations"
    >
      <div className="sidebar-heading">
        <Button
          className="sidebar-rail-toggle"
          variant="ghost"
          size="icon"
          aria-label={
            drawerOpen
              ? 'Close conversation sidebar'
              : 'Open conversation sidebar'
          }
          onClick={onToggleDrawer}
        >
          <PanelLeft />
        </Button>
        <Button
          className="sidebar-collapse-toggle"
          variant="ghost"
          size="icon"
          aria-label={
            desktopCollapsed
              ? 'Expand conversation sidebar'
              : 'Collapse conversation sidebar'
          }
          onClick={onToggleCollapse}
        >
          <PanelLeft />
        </Button>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="sidebar-open-project"
              variant="ghost"
              size="icon"
              aria-label="New session"
              onClick={onOpenProject}
              disabled={disconnected}
            >
              <FolderPlus />
            </Button>
          </TooltipTrigger>
          <TooltipContent>New session · {primaryModifier} O</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              className="sidebar-new"
              variant="ghost"
              size="icon"
              aria-label="New conversation"
              onClick={onNew}
              disabled={disconnected}
            >
              <PenLine />
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            New conversation · {primaryModifier} N
          </TooltipContent>
        </Tooltip>
      </div>
      <label className="session-search">
        <Search size={14} aria-hidden="true" />
        <input
          aria-label="Search conversations"
          placeholder="Search sessions"
          value={filter}
          onChange={(event) => {
            onFilter(event.target.value)
          }}
        />
      </label>
      {starting.map((item) => (
        <div className="starting-row" role="status" key={item.key}>
          <StatusMark status="working" />
          <span>{item.label}</span>
          <Elapsed since={item.startedAt} />
        </div>
      ))}
      <ScrollArea className="session-list-scroll">
        {projects.map((project) => (
          <ProjectSection
            key={project}
            project={project}
            conversations={conversations.filter(
              (conversation) => conversation.project === project,
            )}
            disconnected={disconnected}
            onNewInProject={onNewInProject}
            {...rowActions}
          />
        ))}
        {conversations.length === 0 && (
          <div className="no-sessions" role="status">
            {filter.trim()
              ? `No sessions match “${filter}”`
              : 'No sessions yet. Start one in a folder to get started.'}
          </div>
        )}
      </ScrollArea>
      <div className="sidebar-bottom">
        <div
          className="sidebar-connection"
          role="status"
          aria-live="polite"
          aria-label={daemonLabel}
        >
          <span
            className={`connection-dot ${disconnected ? 'is-offline' : 'is-connected'}`}
          />
          <span>
            {previewMode
              ? 'Local preview · sample data'
              : disconnected
                ? 'Reconnecting to maittyd…'
                : `maittyd · ${String(sessionCount)} ${sessionCount === 1 ? 'session' : 'sessions'}`}
          </span>
        </div>
        <Button
          className="sidebar-settings"
          variant="ghost"
          size="sm"
          aria-label="Open settings"
          onClick={onOpenSettings}
        >
          <Settings2 />
          <span>Settings</span>
        </Button>
      </div>
    </aside>
  )
}

type SessionRowActions = {
  pinnedIds: string[]
  selectedId: string
  onSelect: (id: string) => void
  onTogglePin: (id: string) => void
  onArchive: (id: string, archived: boolean) => void
  onRename: (id: string, title: string) => void
  onEnd: (id: string) => void
}

function ProjectSection({
  project,
  conversations,
  disconnected,
  onNewInProject,
  ...actions
}: {
  project: string
  conversations: Conversation[]
  disconnected: boolean
  onNewInProject: (folder: string) => void
} & SessionRowActions) {
  const needsYou = conversations.filter(
    (conversation) => conversation.status === 'needs-you',
  ).length
  const active = [...conversations]
    .sort((left, right) => {
      const attention = (conversation: Conversation) =>
        conversation.status === 'needs-you' ||
        conversation.status === 'recovery'
          ? 0
          : 1
      const attentionOrder = attention(left) - attention(right)
      if (attentionOrder !== 0) return attentionOrder
      const pinOrder =
        Number(actions.pinnedIds.includes(right.id)) -
        Number(actions.pinnedIds.includes(left.id))
      if (pinOrder !== 0) return pinOrder
      return 0
    })

  return (
    <section className="project-section" aria-label={`${project} project`}>
      <ContextMenu>
        <ContextMenuTrigger asChild>
          <div className="project-heading">
            <span>{project}</span>
            {needsYou > 0 && (
              <span className="project-needs">{needsYou} needs you</span>
            )}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  className="project-new"
                  variant="ghost"
                  size="icon"
                  aria-label={`New session in ${project}`}
                  disabled={disconnected}
                  onClick={() => {
                    onNewInProject(conversations[0]?.folder ?? '')
                  }}
                >
                  <Plus />
                </Button>
              </TooltipTrigger>
              <TooltipContent>New session in {project}</TooltipContent>
            </Tooltip>
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <ContextMenuItem
            disabled={disconnected}
            onSelect={() => {
              onNewInProject(conversations[0]?.folder ?? '')
            }}
          >
            <Plus size={14} />
            New session in {project}
          </ContextMenuItem>
        </ContextMenuContent>
      </ContextMenu>

      <div className="session-rows">
        {active.map((conversation) => (
          <SessionRow
            key={conversation.id}
            conversation={conversation}
            selected={conversation.id === actions.selectedId}
            isPinned={actions.pinnedIds.includes(conversation.id)}
            {...actions}
          />
        ))}
      </div>
      {disconnected && (
        <span className="project-offline">Last known statuses</span>
      )}
    </section>
  )
}

function RowMeta({ conversation }: { conversation: Conversation }) {
  const since = useWorkingSince(conversation.id)
  if (conversation.status === 'working' && since !== undefined)
    return <Elapsed since={since} />
  if (conversation.status === 'needs-you')
    return <span className="meta-needs">Needs you</span>
  if (conversation.status === 'recovery')
    return <span className="meta-recovery">Recovery</span>
  if (conversation.status === 'unknown') return <>—</>
  return <>{conversation.updated}</>
}

function rowSubline(conversation: Conversation): string {
  switch (conversation.status) {
    case 'needs-you':
      return 'Waiting for your reply'
    case 'recovery':
      return `${conversation.provider} exited mid-turn`
    case 'working':
      return 'Working…'
    case 'unknown':
      return conversation.lastKnownStatus
        ? `Last seen ${statusText[conversation.lastKnownStatus].toLowerCase()}`
        : 'Last seen —'
    case 'done':
    case 'idle':
      return `${conversation.project} · ${conversation.branch}`
  }
}

function SessionRow({
  conversation,
  selected,
  isPinned,
  isArchived = false,
  onSelect,
  onTogglePin,
  onArchive,
  onRename,
  onEnd,
}: {
  conversation: Conversation
  selected: boolean
  isPinned: boolean
  isArchived?: boolean
} & Pick<
  SessionRowActions,
  'onSelect' | 'onTogglePin' | 'onArchive' | 'onRename' | 'onEnd'
>) {
  const lastKnownStatus = conversation.lastKnownStatus
  const lastKnownText =
    conversation.status === 'unknown' && lastKnownStatus
      ? `, last known ${statusText[lastKnownStatus]}`
      : ''
  const [isRenaming, setIsRenaming] = useState(false)
  const [renameDraft, setRenameDraft] = useState(conversation.title)

  function beginRename() {
    setRenameDraft(conversation.title)
    setIsRenaming(true)
  }

  function saveRename() {
    onRename(conversation.id, renameDraft)
    setIsRenaming(false)
  }

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div className={`session-row-shell ${selected ? 'is-selected' : ''}`}>
          {isRenaming ? (
            <div className="session-row-rename">
              <input
                autoFocus
                aria-label={`Rename ${conversation.title}`}
                value={renameDraft}
                onChange={(event) => {
                  setRenameDraft(event.target.value)
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') {
                    event.preventDefault()
                    saveRename()
                  } else if (event.key === 'Escape') setIsRenaming(false)
                }}
              />
              <Button
                variant="ghost"
                size="icon"
                aria-label="Save conversation name"
                onClick={saveRename}
              >
                <Check />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                aria-label="Cancel rename"
                onClick={() => {
                  setIsRenaming(false)
                }}
              >
                <X />
              </Button>
            </div>
          ) : (
            <button
              className={`session-row status-${conversation.status} ${selected ? 'is-selected' : ''} ${conversation.status === 'needs-you' ? 'is-attention' : ''} ${isArchived ? 'is-archived' : ''}`}
              type="button"
              onClick={() => {
                onSelect(conversation.id)
              }}
              onKeyDown={(event) => {
                if (event.key === 'F2') {
                  event.preventDefault()
                  beginRename()
                }
              }}
              aria-label={`${conversation.title}, ${statusText[conversation.status]}${lastKnownText}, ${conversation.project}`}
              aria-current={selected ? 'page' : undefined}
            >
              <StatusMark status={conversation.status} className="row-mark" />
              <span className="session-row-copy">
                <span className="session-row-top">
                  <span className="session-row-title">
                    {conversation.title}
                  </span>
                  <span className="session-updated">
                    <RowMeta conversation={conversation} />
                  </span>
                </span>
                <span className="session-row-bottom">
                  <span>{rowSubline(conversation)}</span>
                </span>
              </span>
            </button>
          )}
        </div>
      </ContextMenuTrigger>
      {!isRenaming && (
        <ContextMenuContent>
          <ContextMenuItem
            onSelect={() => {
              onTogglePin(conversation.id)
            }}
          >
            <Pin size={14} />
            {isPinned ? 'Unpin conversation' : 'Pin conversation'}
          </ContextMenuItem>
          <ContextMenuItem onSelect={beginRename}>
            <Pencil size={14} />
            Rename
          </ContextMenuItem>
          <ContextMenuItem
            onSelect={() => {
              onArchive(conversation.id, true)
            }}
          >
            <X size={14} />
            Close session
          </ContextMenuItem>
          <ContextMenuItem
            variant="destructive"
            disabled={conversation.execution !== 'Structured'}
            onSelect={() => {
              onEnd(conversation.id)
            }}
          >
            <Square size={14} />
            End session
          </ContextMenuItem>
        </ContextMenuContent>
      )}
    </ContextMenu>
  )
}
