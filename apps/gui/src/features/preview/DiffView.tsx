import { useCallback, useEffect, useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ArrowUpRight, ChevronDown, CircleAlert, FileCode2, GitBranch, LoaderCircle } from 'lucide-react'
import { toast } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { HighlightedLine } from '@/components/HighlightedLine'
import { loadConversationDiff } from '@/api/daemon'
import { type Conversation, previewDiff } from '@/data/preview'
import { conversationDiffRefreshInterval } from '@/data/conversationDiffRefresh'

function isPatchHeader(line: string): boolean {
  return (
    line.startsWith('@@') || line.startsWith('+++') || line.startsWith('---')
  )
}

export function DiffView({
  conversation,
  desktop,
}: {
  conversation: Conversation
  desktop: boolean
}) {
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
  const diffQuery = useQuery({
    queryKey: ['conversation-diff', conversation.id, conversation.generation],
    queryFn: () => loadConversationDiff(conversation),
    enabled: desktop && conversation.isGitRepository !== false,
    retry: false,
    // A failed diff waits for the user's Retry instead of refetching on every mount.
    retryOnMount: false,
    staleTime: 1500,
    refetchInterval: (query) =>
      conversation.status === 'working'
        ? conversationDiffRefreshInterval(desktop, query.state.status)
        : false,
  })
  const files = useMemo(
    () =>
      desktop
        ? (diffQuery.data?.files ?? [])
        : previewDiff.map((file) => ({
            path: file.file,
            additions: file.added,
            deletions: file.removed,
            patch: file.lines.join('\n'),
            binary: false,
          })),
    [desktop, diffQuery.data?.files],
  )
  const selectedFileIndex = Math.max(
    0,
    files.findIndex((file) => file.path === selectedFilePath),
  )
  const selectedFile = files[selectedFileIndex]
  const selectRelativeFile = useCallback(
    (direction: -1 | 1) => {
      const currentIndex = Math.max(
        0,
        files.findIndex((file) => file.path === selectedFilePath),
      )
      const nextIndex = Math.max(
        0,
        Math.min(files.length - 1, currentIndex + direction),
      )
      const nextFile = files[nextIndex]
      if (nextFile) setSelectedFilePath(nextFile.path)
    },
    [files, selectedFilePath],
  )
  useEffect(() => {
    const onKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || event.shiftKey)
        return
      if (event.key !== '[' && event.key !== ']') return
      if (files.length <= 1) return
      const target = event.target instanceof HTMLElement ? event.target : null
      const isTextEntry =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        Boolean(target?.isContentEditable)
      if (isTextEntry || target?.closest('[role="dialog"], .terminal-screen'))
        return
      event.preventDefault()
      selectRelativeFile(event.key === ']' ? 1 : -1)
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [files.length, selectRelativeFile])

  if (desktop && conversation.isGitRepository === false)
    return (
      <div className="panel-empty">
        <GitBranch size={18} />
        <h2>Changes need Git</h2>
        <p>
          This folder isn’t a Git repository. Initialize Git here to view file
          changes.
        </p>
      </div>
    )
  if (desktop && diffQuery.isLoading)
    return (
      <div className="panel-empty" role="status">
        <LoaderCircle size={18} />
        <h2>Loading changes</h2>
      </div>
    )
  if (desktop && diffQuery.isError)
    return (
      <div className="panel-empty">
        <CircleAlert size={18} />
        <h2>Couldn?t load changes</h2>
        <p>
          Check that the project folder exists and is a Git repository, then
          retry.
        </p>
        <Button
          variant="soft"
          size="sm"
          onClick={() => {
            void diffQuery.refetch()
          }}
        >
          Retry
        </Button>
      </div>
    )
  if (files.length === 0)
    return (
      <div className="panel-empty">
        <FileCode2 size={18} />
        <h2>No changes yet</h2>
        <p>Changes made in this conversation?s project will appear here.</p>
      </div>
    )

  const additions = desktop
    ? (diffQuery.data?.additions ?? 0)
    : files.reduce((total, file) => total + file.additions, 0)
  const deletions = desktop
    ? (diffQuery.data?.deletions ?? 0)
    : files.reduce((total, file) => total + file.deletions, 0)
  const totalFiles = desktop
    ? (diffQuery.data?.total_files ?? files.length)
    : files.length
  if (!selectedFile)
    return (
      <div className="panel-empty">
        <FileCode2 size={18} />
        <h2>No changes yet</h2>
        <p>Changes made in this conversation?s project will appear here.</p>
      </div>
    )
  return (
    <div className="diff-view">
      <div className="diff-summary">
        <strong>
          {totalFiles} {totalFiles === 1 ? 'file' : 'files'} changed
        </strong>
        <span>
          <i>+{additions}</i>{' '}
          <b>
            {'?'}
            {deletions}
          </b>
        </span>
      </div>
      {desktop && diffQuery.data?.truncated && (
        <p className="diff-truncated" role="status">
          Showing a partial diff. Some large or unsupported files were omitted.
        </p>
      )}
      <div className="diff-file-navigation">
        <span role="status" aria-live="polite" aria-atomic="true">
          File {selectedFileIndex + 1} of {files.length}: {selectedFile.path}
        </span>
        <div>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Previous changed file"
            onClick={() => {
              selectRelativeFile(-1)
            }}
            disabled={selectedFileIndex === 0}
          >
            <ChevronDown className="diff-previous-icon" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Next changed file"
            onClick={() => {
              selectRelativeFile(1)
            }}
            disabled={selectedFileIndex === files.length - 1}
          >
            <ChevronDown className="diff-next-icon" />
          </Button>
        </div>
      </div>
      <div className="diff-file-list" role="group" aria-label="Changed files">
        {files.map((file, index) => (
          <button
            className={`diff-file-choice ${index === selectedFileIndex ? 'is-selected' : ''}`}
            type="button"
            key={file.path}
            aria-pressed={index === selectedFileIndex}
            aria-label={`Select changed file ${file.path}`}
            onClick={() => {
              setSelectedFilePath(file.path)
            }}
          >
            <FileCode2 size={13} />
            <span>{file.path}</span>
            <small>
              <i>+{file.additions}</i>{' '}
              <b>
                {'?'}
                {file.deletions}
              </b>
            </small>
          </button>
        ))}
      </div>
      <section className="diff-file">
        <div className="diff-file-head">
          <FileCode2 size={13} />
          <span>
            {selectedFile.path.split('/').at(-1) ?? selectedFile.path}
          </span>
          <small>
            <i>+{selectedFile.additions}</i>{' '}
            <b>
              {'?'}
              {selectedFile.deletions}
            </b>
          </small>
        </div>
        <div className="diff-file-path">{selectedFile.path}</div>
        {selectedFile.binary ? (
          <p className="diff-binary">Binary file changed</p>
        ) : (
          <pre>
            {selectedFile.patch.split('\n').map((line, index) => (
              <span
                className={
                  line.startsWith('+')
                    ? 'diff-added'
                    : line.startsWith('-')
                      ? 'diff-removed'
                      : 'diff-context'
                }
                key={`${selectedFile.path}:${String(index)}`}
              >
                {line.length > 0 && !isPatchHeader(line) ? (
                  <>
                    {line.slice(0, 1)}
                    <HighlightedLine
                      line={line.slice(1)}
                      path={selectedFile.path}
                    />
                  </>
                ) : (
                  line
                )}
              </span>
            ))}
          </pre>
        )}
      </section>
      {!desktop && (
        <Button
          variant="soft"
          className="open-editor-button"
          onClick={() =>
            toast.message('Open in editor is not connected in preview')
          }
        >
          <ArrowUpRight size={14} />
          Open in editor
        </Button>
      )}
    </div>
  )
}
