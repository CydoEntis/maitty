import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { PanelRightClose } from 'lucide-react'
import { Tabs } from 'radix-ui'
import { loadTargetDiff } from '@/api/daemon'
import { Button } from '@/components/ui/button'
import { hasPendingApproval, type ChatFeed } from '@/features/chat/chatFeed'
import { formatComment } from '@/features/chat/commentMessage'
import { AgentsCard } from '@/features/chat/components/AgentsCard'
import { newQueuedMessage, updateComposer } from '@/features/chat/composerStore'
import { useChatComposer } from '@/features/chat/useChatComposer'
import { useChatFeed } from '@/features/chat/useChatFeed'
import { relativePath } from '@/features/chat/stepInfo'
import { useTipWithKeys } from '@/features/shortcuts/platform'
import { AppIcon } from '@/features/workspace/components/glyphs'
import type { ChatAgent, ChatAvailability, ChatLink } from '@/features/workspace/model/chatView'
import { ArtifactsSegment } from './ArtifactsSegment'
import { artifactsOf } from './artifacts'
import { ChangesSegment } from './ChangesSegment'
import { ChecklistCard } from './ChecklistCard'
import { addComment, useComments } from './commentStore'
import { DiffView } from './DiffView'
import { FilesSegment } from './FilesSegment'
import { checklistOf, currentItemSince, emptyReason, helperAgentsOf, touchedFilesOf } from './panelModel'
import { consumeDiffRequest, useDiffRequest } from './panelCommands'

/** The docked panel is 320px wide, 520px while a diff is open (v18 C.2). */
export const PANEL_WIDTH_PX = 320
export const PANEL_WIDE_WIDTH_PX = 520

/** How often the open panel asks for the worktree's changes. */
export const DIFF_REFRESH_MS = 4000

type Props = {
  mode: 'docked' | 'overlay'
  /** The session's name and what runs in it. */
  title: string
  process: string
  availability: ChatAvailability
  /** The repository's root, so an edit's absolute path finds its file in the diff. */
  root: string | null
  /** The session's own folder, which the daemon reads project files from. */
  folder: string | null
  branch: string | null
  /** The daemon is connected, so a comment can be delivered. */
  online: boolean
  /** The chat is showing this session, so its composer delivers queued messages; otherwise the panel does. */
  chatOpen: boolean
  width: number
  onWide: (wide: boolean) => void
  /** Shows a file in the Inspect drawer's Files view. */
  onOpenInFiles: (file: string) => void
  onClose: () => void
}

function Header({ title, process, agent, onClose }: { title: string; process: string; agent: ChatAgent | null; onClose: () => void }) {
  const tip = useTipWithKeys()
  return (
    <header className="flex h-[46px] shrink-0 items-center gap-2.5 border-b border-line2 px-3">
      {agent !== null && <AppIcon process={agent} size={24} />}
      <div className="min-w-0 flex-1">
        <h2 className="m-0 truncate text-[13px] font-semibold text-fg">{title}</h2>
        <p className="m-0 truncate font-mono text-[11px] text-dim">{process}</p>
      </div>
      <Button type="button" variant="ghost" size="icon" className="size-8 text-dim hover:text-fg" aria-label={tip('Hide session panel', 'session-panel')} onClick={onClose}>
        <PanelRightClose className="size-4" aria-hidden="true" />
      </Button>
    </header>
  )
}

function Nothing({ children }: { children: ReactNode }) {
  return (
    <div className="px-1 py-6 text-center">
      <p className="m-0 text-[13px] font-semibold text-fg2">Nothing to track yet</p>
      <p className="mt-1.5 mb-0 text-[12.5px] leading-relaxed text-dim">{children}</p>
    </div>
  )
}

/**
 * Delivers the messages queued for the conversation while the chat is not open. The chat's own
 * composer does it when it is; exactly one of them runs, so a comment is sent once.
 */
function QueueDelivery({ link, feed, online, refresh }: { link: ChatLink; feed: ChatFeed; online: boolean; refresh: () => void }) {
  useChatComposer({ link, feed, online, refresh })
  return null
}

const SEGMENT = 'h-7 flex-1 rounded-md px-2 text-[12px] text-dim data-[state=active]:bg-hover data-[state=active]:text-fg hover:text-fg'

/** What the session's own events say: its checklist, helper agents and worktree changes. Mounted only for a session with a chat link. */
function AgentBody({ link, agent, root, folder, branch, online, chatOpen, onWide, onOpenInFiles }: { link: ChatLink; agent: ChatAgent; root: string | null; folder: string | null; branch: string | null; online: boolean; chatOpen: boolean; onWide: (wide: boolean) => void; onOpenInFiles: (file: string) => void }) {
  const { feed, events, refresh } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const working = feed.status === 'Working'
  const { checklist, unreadable } = useMemo(() => checklistOf(events), [events])
  const helpers = useMemo(() => helperAgentsOf(events), [events])
  const since = useMemo(() => (checklist === null ? null : currentItemSince(events, checklist)), [events, checklist])
  const comments = useComments(link.conversation)
  const touched = useMemo(() => touchedFilesOf(events), [events])
  // The agent names files by absolute path; the panel shows and asks the daemon for them relative to the project.
  const showPath = useCallback((file: string) => relativePath(file.replace(/\\/g, '/'), root), [root])
  const readPathOf = useCallback((file: string) => relativePath(file.replace(/\\/g, '/'), folder ?? root), [folder, root])
  const artifacts = useMemo(() => artifactsOf(touched, showPath), [touched, showPath])

  const [segment, setSegment] = useState('changes')
  const [diffFile, setDiffFile] = useState<string | null>(null)
  const [artifactFile, setArtifactFile] = useState<string | null>(null)

  const diff = useQuery({
    queryKey: ['session-diff', link.conversation, link.generation],
    queryFn: () => loadTargetDiff(link),
    refetchInterval: DIFF_REFRESH_MS,
    refetchIntervalInBackground: false,
  })
  // The diff is read again as soon as the agent edits a file, not only on the timer.
  const editCount = useMemo(() => events.filter((event) => event.kind === 'edit').length, [events])
  const refetchDiff = diff.refetch
  const seenEdits = useRef(editCount)
  useEffect(() => {
    if (seenEdits.current === editCount) return
    seenEdits.current = editCount
    void refetchDiff()
  }, [editCount, refetchDiff])

  // An edit block's header asks to see its file here, opening the panel first if it was hidden. It is
  // answered during render once the changes are read (the same way a new generation resets the feed).
  const request = useDiffRequest()
  const [answered, setAnswered] = useState(0)
  const [missing, setMissing] = useState<string | null>(null)
  if (request !== null && request.nonce !== answered && diff.data !== undefined) {
    setAnswered(request.nonce)
    setSegment('changes')
    const wanted = request.file.replace(/\\/g, '/')
    const relative = relativePath(wanted, root)
    const found = diff.data.files.find((file) => file.path === relative || wanted.endsWith(`/${file.path}`))
    setMissing(found === undefined ? relative : null)
    setDiffFile(found?.path ?? null)
  }
  useEffect(() => {
    if (request !== null && request.nonce === answered) consumeDiffRequest(request.nonce)
  }, [request, answered])

  const openFile = diff.data?.files.find((file) => file.path === diffFile)
  const diffOpen = openFile !== undefined
  useEffect(() => {
    onWide(diffOpen)
    return () => { onWide(false) }
  }, [diffOpen, onWide])

  const deliveryNote = hasPendingApproval(feed)
    ? `${agent} is waiting for an approval. This comment is queued and sent after you answer it.`
    : !online ? 'The daemon is offline. This comment is queued until it is back.'
      : working ? `${agent} is working. This comment is queued and sent when it finishes this turn.` : null

  const sendComment = useCallback((file: string, comment: { line: number; code: string; text: string }) => {
    const entry = { file, ...comment }
    const message = formatComment(entry)
    // The composer's own queue is the one route to the agent, so a comment is never sent while an approval waits.
    updateComposer(link.conversation, (state) => ({ ...state, queue: [...state.queue, newQueuedMessage(message)] }))
    addComment(link.conversation, entry, message)
  }, [link.conversation])

  const delivery = !chatOpen && <QueueDelivery link={link} feed={feed} online={online} refresh={refresh} />

  if (!feed.loaded) return <p className="m-0 px-1 py-6 text-center text-[12.5px] text-dim">Reading the conversation…</p>
  if (openFile !== undefined) {
    return (
      <>
        {delivery}
        <DiffView
          file={openFile}
          agent={agent}
          comments={comments.filter((comment) => comment.file === openFile.path)}
          deliveryNote={deliveryNote}
          onBack={() => { setDiffFile(null) }}
          onComment={(comment) => { sendComment(openFile.path, comment) }}
        />
      </>
    )
  }
  const changeCount = diff.data?.total_files
  return (
    <>
      {delivery}
      {checklist !== null && <ChecklistCard checklist={checklist} working={working} currentSince={since} />}
      {checklist === null && unreadable && <p className="m-0 rounded-[10px] border border-line2 bg-raise px-3 py-2 text-[12px] text-dim">The agent made a checklist, but the conversation history did not keep its items.</p>}
      {helpers.length > 0 && <AgentsCard agents={helpers} panel />}
      {checklist === null && !unreadable && helpers.length === 0 && (
        <Nothing>The agent has made no checklist and started no helper agents in this session.</Nothing>
      )}
      <Tabs.Root value={segment} onValueChange={setSegment} className="flex flex-col gap-2">
        <Tabs.List aria-label="Session panel sections" className="flex gap-1 rounded-lg border border-line2 bg-raise p-0.5">
          <Tabs.Trigger value="changes" className={SEGMENT}>Changes{changeCount === undefined ? '' : ` · ${String(changeCount)}`}</Tabs.Trigger>
          <Tabs.Trigger value="files" className={SEGMENT}>Files · {touched.length}</Tabs.Trigger>
          <Tabs.Trigger value="artifacts" className={SEGMENT}>Artifacts · {artifacts.length}</Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="changes" className="outline-none">
          {missing !== null && <p role="status" className="m-0 mb-2 rounded-lg border border-line2 bg-raise px-2.5 py-1.5 text-[12px] text-dim">{missing} has no changes in the worktree now.</p>}
          <ChangesSegment query={diff} branch={branch} comments={comments} onOpen={(path) => { setMissing(null); setDiffFile(path) }} />
        </Tabs.Content>
        <Tabs.Content value="files" className="outline-none">
          <FilesSegment
            files={touched}
            displayPath={showPath}
            hasDiff={(path) => diff.data?.files.some((file) => file.path === path) === true}
            onOpenDiff={(file) => { setSegment('changes'); setDiffFile(showPath(file)) }}
            onOpenInFiles={onOpenInFiles}
          />
        </Tabs.Content>
        <Tabs.Content value="artifacts" className="outline-none">
          <ArtifactsSegment artifacts={artifacts} target={link} readPathOf={readPathOf} open={artifactFile} onOpen={setArtifactFile} onOpenInFiles={onOpenInFiles} />
        </Tabs.Content>
      </Tabs.Root>
    </>
  )
}

/**
 * The session panel (v18 C.2) for the focused session: its agent's checklist and helper agents, read
 * from the daemon's conversation events, and the agent's worktree changes with line comments. A
 * session with no agent or no event stream says why there is nothing to show. Docked beside the
 * panes on a wide window, an overlay on a compact one.
 */
export function SessionPanel({ mode, title, process, availability, root, folder, branch, online, chatOpen, width, onWide, onOpenInFiles, onClose }: Props) {
  const agent = availability.kind === 'ready' ? availability.agent : null
  const body = (
    <aside
      aria-label="Session panel"
      className={`mx-card flex min-h-0 max-w-full flex-col overflow-hidden ${mode === 'docked' ? 'm-[var(--edge)] shrink-0' : 'h-full'}`}
      style={{ width }}
    >
      <Header title={title} process={process} agent={agent} onClose={onClose} />
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
        {availability.kind === 'ready'
          ? <AgentBody key={availability.link.conversation} link={availability.link} agent={availability.agent} root={root} folder={folder} branch={branch} online={online} chatOpen={chatOpen} onWide={onWide} onOpenInFiles={onOpenInFiles} />
          : <Nothing>{emptyReason(availability)}</Nothing>}
      </div>
    </aside>
  )
  if (mode === 'docked') return body
  // Compact: over a 40% scrim, from the window's edge (v18 C.2). A press on the scrim closes it; Esc is the window's ladder.
  return (
    <div className="mx-drawer-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <div className="absolute top-10 right-2 bottom-2 max-w-[calc(100%-16px)] [.mx-app[data-side=right]_&]:right-auto [.mx-app[data-side=right]_&]:left-2">{body}</div>
    </div>
  )
}
