import { Check, FolderOpen, LoaderCircle, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import type { SessionPermissions } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { PERMISSION_LABELS } from '@/data/conversationPermissions'
import { AgentAvatar } from './AgentAvatar'

export function NewConversationDialog({
  open,
  project,
  onProjectChange,
  provider,
  onProviderChange,
  kind,
  onKindChange,
  permissions,
  onPermissionsChange,
  claudeAvailable,
  desktop,
  onChooseFolder,
  isChoosingFolder,
  onClose,
  onCreate,
  disabled,
  isCreating,
  recentProjects,
}: {
  open: boolean
  project: string
  onProjectChange: (value: string) => void
  provider: 'Codex' | 'Claude'
  onProviderChange: (provider: 'Codex' | 'Claude') => void
  kind: 'chat' | 'terminal'
  onKindChange: (kind: 'chat' | 'terminal') => void
  permissions: SessionPermissions
  onPermissionsChange: (permissions: SessionPermissions) => void
  claudeAvailable: boolean
  desktop: boolean
  onChooseFolder: () => void
  isChoosingFolder: boolean
  onClose: () => void
  onCreate: () => void
  disabled: boolean
  isCreating: boolean
  recentProjects: Conversation[]
}) {
  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="new-conversation-dialog">
        <DialogHeader>
          <DialogTitle>New conversation</DialogTitle>
          <DialogDescription>
            Choose a project and agent. The agent will use its saved provider
            configuration.
          </DialogDescription>
        </DialogHeader>
        <div className="dialog-field">
          <label htmlFor="project-folder">Project folder</label>
          <div className="project-folder-input">
            <input
              id="project-folder"
              value={project}
              onChange={(event) => {
                onProjectChange(event.target.value)
              }}
              placeholder="Home folder unless you choose one"
              autoFocus
            />
            <Button
              type="button"
              variant="soft"
              onClick={onChooseFolder}
              disabled={!desktop || disabled || isChoosingFolder}
              aria-label="Browse for project folder"
            >
              <FolderOpen size={14} />
              {isChoosingFolder ? 'Opening…' : 'Browse'}
            </Button>
          </div>
        </div>
        {recentProjects.length > 0 && (
          <div
            className="dialog-recent"
            role="group"
            aria-label="Recent projects"
          >
            <span className="dialog-caps">Recent</span>
            {recentProjects.map((item) => (
              <button
                key={item.folder}
                type="button"
                className={`dialog-recent-row ${project === item.folder ? 'is-selected' : ''}`}
                onClick={() => {
                  onProjectChange(item.folder)
                }}
              >
                <FolderOpen size={14} aria-hidden="true" />
                <strong>{item.project}</strong>
                <code>{item.folder}</code>
              </button>
            ))}
          </div>
        )}
        <div
          className="dialog-provider-options"
          role="group"
          aria-label="Agent"
        >
          {(['Codex', 'Claude'] as const).map((choice) => (
            <button
              key={choice}
              type="button"
              className={`dialog-provider ${provider === choice ? 'is-selected' : ''}`}
              disabled={choice === 'Claude' && !claudeAvailable}
              aria-pressed={provider === choice}
              onClick={() => {
                onProviderChange(choice)
              }}
            >
              <AgentAvatar provider={choice} status="idle" />
              <div>
                <strong>{choice}</strong>
                <span>
                  {choice === 'Claude'
                    ? claudeAvailable
                      ? 'Claude Code CLI and saved sign-in.'
                      : 'Claude Code CLI was not found.'
                    : 'Codex CLI and saved sign-in.'}
                </span>
              </div>
              {provider === choice && <Check size={15} />}
            </button>
          ))}
        </div>
        {desktop && (
          <div
            className="dialog-provider-options"
            role="group"
            aria-label="Session type"
          >
            {(
              [
                ['chat', 'Chat', 'Messages and tool output in this window.'],
                [
                  'terminal',
                  'Terminal',
                  'The agent in a terminal, also open in the terminal app.',
                ],
              ] as const
            ).map(([choice, name, note]) => (
              <button
                key={choice}
                type="button"
                className={`dialog-provider ${kind === choice ? 'is-selected' : ''}`}
                aria-pressed={kind === choice}
                onClick={() => {
                  onKindChange(choice)
                }}
              >
                <div>
                  <strong>{name}</strong>
                  <span>{note}</span>
                </div>
                {kind === choice && <Check size={15} />}
              </button>
            ))}
          </div>
        )}
        {kind === 'chat' && (
          <div
            className="dialog-permissions"
            role="group"
            aria-label="Permissions"
          >
            <span className="dialog-caps">Permissions</span>
            {(
              [
                ['read-only', 'Reads files and asks before anything else.'],
                [
                  'workspace-write',
                  'Edits files in the project and asks before the rest.',
                ],
                [
                  'full-access',
                  'Never asks and is not sandboxed. Use with care.',
                ],
              ] as const
            ).map(([value, help]) => (
              <button
                key={value}
                type="button"
                className={`dialog-permission ${permissions === value ? 'is-selected' : ''} ${value === 'full-access' ? 'is-danger' : ''}`}
                aria-pressed={permissions === value}
                onClick={() => {
                  onPermissionsChange(value)
                }}
              >
                <strong>{PERMISSION_LABELS[value]}</strong>
                <span>{help}</span>
              </button>
            ))}
          </div>
        )}
        <DialogFooter>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={onCreate} disabled={disabled}>
            {isCreating ? (
              <LoaderCircle size={14} className="animate-spin" />
            ) : (
              <Plus size={14} />
            )}
            {isCreating
              ? 'Starting session…'
              : kind === 'terminal'
                ? 'Start terminal session'
                : 'Start conversation'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
