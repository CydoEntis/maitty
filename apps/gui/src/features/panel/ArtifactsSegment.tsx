import { ArrowLeft, Braces, FileText, Image as ImageIcon, ScrollText } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { loadTargetFile, type ConversationTarget } from '@/api/daemon'
import { Button } from '@/components/ui/button'
import { Markdown } from '@/features/chat/components/Markdown'
import { errorMessage } from '@/lib/error-message'
import { canPreview, isMarkdown, type Artifact, type ArtifactKind } from './artifacts'

const TILE: Record<ArtifactKind, { icon: typeof FileText; tone: string }> = {
  document: { icon: FileText, tone: 'bg-[color-mix(in_srgb,var(--accent)_16%,transparent)] text-[var(--accent)]' },
  report: { icon: ScrollText, tone: 'bg-[color-mix(in_srgb,var(--ok)_16%,transparent)] text-[var(--ok)]' },
  data: { icon: Braces, tone: 'bg-[color-mix(in_srgb,var(--warn)_16%,transparent)] text-[var(--warn)]' },
  image: { icon: ImageIcon, tone: 'bg-[color-mix(in_srgb,var(--ansi-mag)_16%,transparent)] text-[var(--ansi-mag)]' },
}

type ViewProps = {
  artifact: Artifact
  target: ConversationTarget
  /** The path the daemon reads, relative to the session's folder. */
  readPath: string
  onBack: () => void
  onOpenInFiles: (file: string) => void
}

/** An artifact opened in the panel: text is read through the daemon and shown as text (markdown drawn safely). */
function ArtifactView({ artifact, target, readPath, onBack, onOpenInFiles }: ViewProps) {
  const previewable = canPreview(artifact.kind)
  const file = useQuery({
    queryKey: ['session-file', target.conversation, target.generation, readPath],
    queryFn: () => loadTargetFile(target, readPath),
    enabled: previewable,
    retry: false,
  })
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Button type="button" variant="ghost" size="icon" className="size-7 shrink-0 text-dim hover:text-fg" aria-label="Back to artifacts" onClick={onBack}>
          <ArrowLeft className="size-4" aria-hidden="true" />
        </Button>
        <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg" title={readPath}>{artifact.title}</span>
      </div>
      {!previewable ? (
        <div className="rounded-lg border border-line2 bg-raise p-3 text-[12.5px] text-dim">
          <p className="m-0">A picture cannot be shown here: the daemon sends project files as text.</p>
          <Button type="button" size="sm" variant="outline" className="mt-2" onClick={() => { onOpenInFiles(artifact.file) }}>Open in the Files drawer</Button>
        </div>
      ) : file.isPending ? (
        <p className="m-0 text-[12.5px] text-dim">Reading the file…</p>
      ) : file.isError ? (
        <p role="alert" className="m-0 text-[12.5px] text-[var(--danger)]">{errorMessage(file.error, 'Could not read the file')}</p>
      ) : isMarkdown(readPath) ? (
        <Markdown text={file.data} className="text-[13px] leading-[1.55] text-fg2" />
      ) : (
        <pre className="m-0 max-h-[60vh] overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[11.5px] leading-[1.55] whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{file.data}</pre>
      )}
    </div>
  )
}

type Props = {
  artifacts: readonly Artifact[]
  target: ConversationTarget
  readPathOf: (file: string) => string
  /** The artifact the segment shows open, and how it is chosen. */
  open: string | null
  onOpen: (file: string | null) => void
  onOpenInFiles: (file: string) => void
}

/** Things the agent produced (v18 C.2): cards with a tinted tile, a title and a subtitle. */
export function ArtifactsSegment({ artifacts, target, readPathOf, open, onOpen, onOpenInFiles }: Props) {
  const opened = artifacts.find((artifact) => artifact.file === open)
  if (opened !== undefined) {
    return <ArtifactView artifact={opened} target={target} readPath={readPathOf(opened.file)} onBack={() => { onOpen(null) }} onOpenInFiles={onOpenInFiles} />
  }
  if (artifacts.length === 0) {
    return <p className="m-0 px-1 py-4 text-center text-[12.5px] text-dim">The agent has not created a document, report or picture in this session.</p>
  }
  return (
    <ul className="m-0 flex list-none flex-col gap-1.5 p-0" aria-label="Artifacts">
      {artifacts.map((artifact) => {
        const { icon: Glyph, tone } = TILE[artifact.kind]
        return (
          <li key={artifact.file}>
            <button type="button" className="flex w-full items-center gap-2.5 rounded-[10px] border border-line2 bg-raise p-2 text-left hover:bg-hover" onClick={() => { onOpen(artifact.file) }}>
              <span className={`flex size-8 shrink-0 items-center justify-center rounded-lg ${tone}`} aria-hidden="true"><Glyph className="size-4" /></span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13px] text-fg">{artifact.title}</span>
                <span className="block truncate text-[11.5px] text-dim">{artifact.subtitle}</span>
              </span>
            </button>
          </li>
        )
      })}
    </ul>
  )
}
