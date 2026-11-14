import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import {
  ChevronDown,
  ChevronRight,
  CircleAlert,
  FileText,
  Folder,
  FolderOpen,
  LoaderCircle,
} from 'lucide-react'
import {
  loadConversationDiff,
  loadConversationFile,
  loadConversationFiles,
} from '@/api/daemon'
import { HighlightedCode } from '@/components/HighlightedCode'
import { Button } from '@/components/ui/button'
import type { Conversation } from '@/data/preview'

const INDENT_PX = 14
const ROW_PADDING_PX = 10

function joinPath(directory: string, name: string): string {
  return directory ? `${directory}/${name}` : name
}

/**
 * The project's files: a tree that stays on screen, and the selected file's
 * contents below it. Read-only; edits belong in your editor.
 */
export function FilesView({
  conversation,
  desktop,
}: {
  conversation: Conversation
  desktop: boolean
}) {
  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set())
  const diffQuery = useQuery({
    queryKey: ['conversation-diff', conversation.id, conversation.generation],
    queryFn: () => loadConversationDiff(conversation),
    enabled: desktop && conversation.isGitRepository !== false,
    retry: false,
    retryOnMount: false,
    staleTime: 1500,
  })
  const changed = new Set(diffQuery.data?.files.map((file) => file.path) ?? [])

  if (!desktop) {
    return (
      <div className="panel-empty">
        <FolderOpen size={18} />
        <h2>Files need the desktop app</h2>
      </div>
    )
  }

  function toggle(path: string) {
    setExpanded((current) => {
      const next = new Set(current)
      if (next.has(path)) next.delete(path)
      else next.add(path)
      return next
    })
  }

  return (
    <div className="files-browser">
      <div className="files-tree" role="tree" aria-label="Project files">
        <DirectoryNode
          conversation={conversation}
          directory=""
          depth={0}
          expanded={expanded}
          changed={changed}
          selectedPath={selectedPath}
          onToggle={toggle}
          onSelect={setSelectedPath}
        />
      </div>
      <div className="files-path">
        {selectedPath ?? 'Select a file to see its contents'}
      </div>
      {selectedPath ? (
        <FilePreview conversation={conversation} path={selectedPath} />
      ) : (
        <div className="files-preview-empty" />
      )}
      <p className="files-readonly-note">
        Read-only preview. Files can’t be changed here.
      </p>
    </div>
  )
}

function DirectoryNode({
  conversation,
  directory,
  depth,
  expanded,
  changed,
  selectedPath,
  onToggle,
  onSelect,
}: {
  conversation: Conversation
  directory: string
  depth: number
  expanded: ReadonlySet<string>
  changed: ReadonlySet<string>
  selectedPath: string | null
  onToggle: (path: string) => void
  onSelect: (path: string) => void
}) {
  const listing = useQuery({
    queryKey: [
      'conversation-files',
      conversation.id,
      conversation.generation,
      directory,
    ],
    queryFn: () => loadConversationFiles(conversation, directory || null),
    retry: false,
  })

  if (listing.isLoading) {
    return (
      <div className="files-status" role="status">
        <LoaderCircle size={13} />
        Loading…
      </div>
    )
  }
  if (listing.isError) {
    return (
      <div className="files-status is-error" role="alert">
        <CircleAlert size={13} />
        Couldn’t load {directory || 'project files'}.
        <Button
          variant="soft"
          size="sm"
          onClick={() => void listing.refetch()}
        >
          Retry
        </Button>
      </div>
    )
  }

  const entries = [...(listing.data ?? [])].sort(
    (left, right) =>
      Number(right.is_directory) - Number(left.is_directory) ||
      left.name.localeCompare(right.name),
  )
  if (entries.length === 0 && depth === 0) {
    return (
      <div className="files-status">
        <FolderOpen size={13} />
        This folder is empty
      </div>
    )
  }

  return (
    <>
      {entries.map((entry) => {
        const path = joinPath(directory, entry.name)
        const indent = {
          paddingLeft: `${String(depth * INDENT_PX + ROW_PADDING_PX)}px`,
        }
        if (entry.is_directory) {
          const open = expanded.has(path)
          return (
            <div key={path} role="group">
              <button
                type="button"
                role="treeitem"
                aria-expanded={open}
                aria-label={`${open ? 'Collapse' : 'Expand'} folder ${entry.name}`}
                className="files-row is-directory"
                style={indent}
                onClick={() => {
                  onToggle(path)
                }}
              >
                {open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
                {open ? <FolderOpen size={14} /> : <Folder size={14} />}
                <span>{entry.name}</span>
              </button>
              {open && (
                <DirectoryNode
                  conversation={conversation}
                  directory={path}
                  depth={depth + 1}
                  expanded={expanded}
                  changed={changed}
                  selectedPath={selectedPath}
                  onToggle={onToggle}
                  onSelect={onSelect}
                />
              )}
            </div>
          )
        }
        return (
          <button
            key={path}
            type="button"
            role="treeitem"
            aria-selected={selectedPath === path}
            aria-label={`Preview file ${entry.name}`}
            className={`files-row ${selectedPath === path ? 'is-selected' : ''}`}
            style={indent}
            onClick={() => {
              onSelect(path)
            }}
          >
            <span className="files-row-spacer" aria-hidden="true" />
            <FileText size={14} />
            <span>{entry.name}</span>
            {changed.has(path) && (
              <b className="files-changed" title="Changed in this session">
                M
              </b>
            )}
          </button>
        )
      })}
    </>
  )
}

function FilePreview({
  conversation,
  path,
}: {
  conversation: Conversation
  path: string
}) {
  const file = useQuery({
    queryKey: [
      'conversation-file',
      conversation.id,
      conversation.generation,
      path,
    ],
    queryFn: () => loadConversationFile(conversation, path),
    retry: false,
  })

  return (
    <section className="files-preview" aria-label={`Preview ${path}`}>
      {file.isLoading ? (
        <p className="files-status" role="status">
          Loading preview…
        </p>
      ) : file.isError ? (
        <div className="files-status is-error" role="alert">
          <CircleAlert size={13} />
          Couldn’t load this file preview.
          <Button
            variant="soft"
            size="sm"
            onClick={() => void file.refetch()}
          >
            Retry
          </Button>
        </div>
      ) : (
        <pre>
          <HighlightedCode code={file.data ?? ''} path={path} />
        </pre>
      )}
    </section>
  )
}
