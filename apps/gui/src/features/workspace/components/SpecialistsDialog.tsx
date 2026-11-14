import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useEffect, useRef, useState } from 'react'
import { readSpecialists, type SpecialistFile, type SpecialistList } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'

type Loaded = { state: 'loading' } | { state: 'ready'; list: SpecialistList } | { state: 'failed'; why: string }

const TOOL_WORDS: Record<SpecialistFile['tool'], string> = { claude: 'Claude Code', codex: 'Codex', unknown: 'Other' }
const SCOPE_WORDS: Record<SpecialistFile['scope'], string> = { project: 'This project', home: 'Your home folder', unknown: 'Elsewhere' }
const SCOPES: readonly SpecialistFile['scope'][] = ['project', 'home', 'unknown']

/**
 * The specialist agents a session's project and its user's home define, for Claude Code and Codex,
 * read-only (ADR 0010). Maitty never makes or edits these files; `/agents` in Claude Code and the
 * files themselves are where that is done. Opening one shows the file as it is.
 */
export function SpecialistsDialog({ title, term, onClose, load = readSpecialists }: { title: string; term: number; onClose: () => void; load?: (term: number) => Promise<SpecialistList> }) {
  const [loaded, setLoaded] = useState<Loaded>({ state: 'loading' })
  const [open, setOpen] = useState<SpecialistFile | null>(null)
  const dialog = useRef<HTMLDivElement>(null)

  useEffect(() => {
    let live = true
    load(term).then(
      (list) => { if (live) setLoaded({ state: 'ready', list }) },
      (error: unknown) => { if (live) setLoaded({ state: 'failed', why: errorMessage(error, 'Could not list the agent files') }) },
    )
    return () => { live = false }
  }, [term, load])

  useEffect(() => {
    dialog.current?.focus()
    // The right-click menu that opened this is still closing, and its focus trap takes focus back as
    // it goes; claim it again once it has finished.
    const timer = window.setTimeout(() => {
      if (!dialog.current?.contains(document.activeElement)) dialog.current?.focus()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])

  const heading = open === null ? `Specialist agents · ${title}` : open.name
  return (
    <Dialog open onOpenChange={(next) => { if (!next) onClose() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
        <div
          ref={dialog}
          tabIndex={-1}
          aria-label={heading}
          className="mx-rename mx-specialists"
          onKeyDown={(event) => {
            // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
            if (event.key === 'Escape') event.stopPropagation()
          }}
        >
          <DialogTitle asChild><h2>{heading}</h2></DialogTitle>
          {open === null ? <Listing loaded={loaded} onOpen={setOpen} /> : <FileView file={open} />}
          <div className="mx-rename-actions">
            {open !== null && <button type="button" className="mx-text-button" onClick={() => { setOpen(null) }}>Back</button>}
            <button type="button" className="mx-text-button is-primary" onClick={onClose}>Close</button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function Listing({ loaded, onOpen }: { loaded: Loaded; onOpen: (file: SpecialistFile) => void }) {
  if (loaded.state === 'loading') return <p role="status">Looking for agent files…</p>
  if (loaded.state === 'failed') return <p role="alert">{loaded.why}</p>
  const { list } = loaded
  if (list.files.length === 0) {
    return <p>No agent files in <code>.claude/agents</code> or <code>.codex/agents</code> here or in your home folder.</p>
  }
  return (
    <div className="mx-specialists-groups">
      {SCOPES.map((scope) => {
        const files = list.files.filter((file) => file.scope === scope)
        if (files.length === 0) return null
        return (
          <section key={scope} aria-label={SCOPE_WORDS[scope]}>
            <h3>{SCOPE_WORDS[scope]}</h3>
            <ul>
              {files.map((file) => (
                <li key={file.path}>
                  <button type="button" className="mx-specialist-row" onClick={() => { onOpen(file) }}>
                    <span className="mx-specialist-name">{file.name}</span>
                    <span className="mx-specialist-tool">{TOOL_WORDS[file.tool]}</span>
                    {file.description !== null && <span className="mx-specialist-description">{file.description}</span>}
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )
      })}
      {list.truncated && <p className="mx-sub-note">Only the first {list.files.length} files are listed.</p>}
    </div>
  )
}

function FileView({ file }: { file: SpecialistFile }) {
  return (
    <div className="mx-specialist-file">
      <p className="mx-sub-note">{TOOL_WORDS[file.tool]} · <code>{file.path}</code></p>
      <pre aria-label={`${file.name} file`}>{file.text}</pre>
      {file.text_cut && <p className="mx-sub-note">The file is longer; only its start is shown.</p>}
    </div>
  )
}
