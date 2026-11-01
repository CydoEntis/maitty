import { useState, type KeyboardEvent } from 'react'
import { useQuery } from '@tanstack/react-query'
import { FolderOpen, FolderPlus } from 'lucide-react'
import { suggestProjectFolders } from '@/api/daemon'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import type { RecentProject } from '@/features/projects/recent-projects'

const PATH_PREFIXES = ['~', '/', '.']

function looksLikePath(value: string): boolean {
  // A Windows drive path ("C:\\dev" or "C:/dev") is also a path.
  return (
    PATH_PREFIXES.some((prefix) => value.startsWith(prefix)) ||
    /^[A-Za-z]:[\\/]/.test(value)
  )
}

function folderName(folder: string): string {
  const parts = folder.split(/[\\/]/).filter(Boolean)
  return parts[parts.length - 1] ?? folder
}

/**
 * Pick a folder by typing a path (folders that continue it are suggested as you
 * type), choosing a recent project, or using the native picker.
 */
export function OpenProjectDialog({
  open,
  recents,
  canBrowse,
  browsing,
  onClose,
  onOpen,
  onBrowse,
}: {
  open: boolean
  recents: RecentProject[]
  canBrowse: boolean
  browsing: boolean
  onClose: () => void
  onOpen: (folder: string) => void
  onBrowse: () => void
}) {
  const [query, setQuery] = useState('')
  const [highlight, setHighlight] = useState(0)
  const trimmed = query.trim()
  const typedPath = looksLikePath(trimmed)
  const suggestionQuery = useQuery({
    queryKey: ['folder-suggestions', trimmed],
    queryFn: () => suggestProjectFolders(trimmed),
    enabled: open && typedPath && canBrowse,
    staleTime: 2000,
  })
  const suggestions = typedPath
    ? (suggestionQuery.data ?? []).filter((path) => path !== trimmed)
    : []
  const matches = typedPath
    ? []
    : recents.filter((recent) =>
        `${recent.name} ${recent.folder}`
          .toLowerCase()
          .includes(trimmed.toLowerCase()),
      )
  // The typed path is row 0 and opens as-is; each suggestion after it completes it.
  const rowCount = typedPath ? 1 + suggestions.length : matches.length

  function complete(path: string) {
    setQuery(path)
    setHighlight(0)
  }

  function activate(index: number) {
    if (typedPath) {
      if (index === 0) onOpen(trimmed)
      else {
        const suggestion = suggestions[index - 1]
        if (suggestion) complete(suggestion)
      }
    } else {
      const match = matches[index]
      if (match) onOpen(match.folder)
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (rowCount === 0) return
      const step = event.key === 'ArrowDown' ? 1 : -1
      setHighlight((current) => (current + step + rowCount) % rowCount)
    } else if (event.key === 'Tab' && typedPath && suggestions.length > 0) {
      event.preventDefault()
      const suggestion = suggestions[Math.max(highlight - 1, 0)]
      if (suggestion) complete(suggestion)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      activate(highlight)
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(isOpen) => {
        if (!isOpen) onClose()
      }}
    >
      <DialogContent className="open-project-dialog">
        <DialogHeader className="sr-only">
          <DialogTitle>Open a project</DialogTitle>
          <DialogDescription>
            Type a folder path or search your recent projects.
          </DialogDescription>
        </DialogHeader>
        <input
          className="open-project-input"
          aria-label="Folder path, or search recent projects"
          placeholder="Folder path, or search recent projects"
          value={query}
          autoFocus
          onChange={(event) => {
            setQuery(event.target.value)
            setHighlight(0)
          }}
          onKeyDown={handleKeyDown}
        />
        <div className="open-project-list" role="listbox" aria-label="Projects">
          {typedPath ? (
            <>
              <button
                type="button"
                role="option"
                aria-selected={highlight === 0}
                className={`open-project-row ${highlight === 0 ? 'is-highlighted' : ''}`}
                onClick={() => {
                  activate(0)
                }}
              >
                <FolderPlus size={16} aria-hidden="true" />
                <span>
                  <strong>Open {folderName(trimmed)}</strong>
                  <code>{trimmed}</code>
                </span>
                <small>Enter</small>
              </button>
              {suggestions.length > 0 && (
                <span className="dialog-caps">Folders here</span>
              )}
              {suggestions.map((path, index) => (
                <button
                  key={path}
                  type="button"
                  role="option"
                  aria-selected={highlight === index + 1}
                  className={`open-project-row ${highlight === index + 1 ? 'is-highlighted' : ''}`}
                  onClick={() => {
                    activate(index + 1)
                  }}
                >
                  <FolderOpen size={16} aria-hidden="true" />
                  <span>
                    <strong>{folderName(path)}</strong>
                    <code>{path}</code>
                  </span>
                  <small>Tab</small>
                </button>
              ))}
            </>
          ) : (
            <>
              <span className="dialog-caps">Recent</span>
              {matches.length === 0 && (
                <p className="open-project-empty">
                  {recents.length === 0
                    ? 'No recent projects yet. Type a folder path above.'
                    : 'No recent project matches. Type a folder path to open a new one.'}
                </p>
              )}
              {matches.map((recent, index) => (
                <button
                  key={recent.folder}
                  type="button"
                  role="option"
                  aria-selected={index === highlight}
                  className={`open-project-row ${index === highlight ? 'is-highlighted' : ''}`}
                  onClick={() => {
                    activate(index)
                  }}
                >
                  <FolderOpen size={16} aria-hidden="true" />
                  <span>
                    <strong>{recent.name}</strong>
                    <code>{recent.folder}</code>
                  </span>
                  <small>
                    {recent.sessions === 1
                      ? '1 session'
                      : `${String(recent.sessions)} sessions`}
                  </small>
                </button>
              ))}
            </>
          )}
        </div>
        <div className="open-project-footer">
          <span>Type a path, then press Tab to complete a folder.</span>
          <Button
            variant="outline"
            size="sm"
            onClick={onBrowse}
            disabled={!canBrowse || browsing}
          >
            {browsing ? 'Opening…' : 'Browse…'}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
