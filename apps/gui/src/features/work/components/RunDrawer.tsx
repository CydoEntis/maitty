import { useState } from 'react'
import { Popover } from 'radix-ui'
import { Button, IconButton } from '@/components/ui/button'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { PopupContent } from '@/components/ui/popup'
import { Icon } from '@/features/workspace/components/glyphs'
import { subagentWord, type SubagentView } from '@/features/workspace/model/subagents'
import { STEP_WORDS, namesHelper, type WorkflowView } from '@/features/workspace/model/workflows'
import { followLink } from '@/features/chat/followLink'
import { WorkStepMark } from './WorkStepMark'

const SECOND_MS = 1000

/** Only a web address is offered as the ticket's link; the host refuses anything else anyway. */
const WEB_ADDRESS = /^https?:\/\//i

/**
 * What a step's helper label says, read-only: the label as the workflow sent it, and where it names one of the
 * session's subagents, what that subagent was asked, what it is doing or reported, and a way to its chat.
 */
function HelperCard({ label, helper, onOpenHelper }: { label: string; helper: SubagentView | undefined; onOpenHelper: (id: string) => void }) {
  // Read once: the card is a still picture of the helper, opened and read, not a clock.
  const [now] = useState(() => Math.floor(Date.now() / SECOND_MS))
  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <button type="button" className="mx-board-helper" aria-label={`Helper ${label}`}>{label}</button>
      </Popover.Trigger>
      <PopupContent side="bottom" align="end" className="mx-board-helper-card" aria-label={`Helper ${label}`}>
        <span className="mx-board-helper-label">{label}</span>
        {helper === undefined ? (
          <p className="mx-board-helper-quiet">No subagent of this session goes by this name.</p>
        ) : (
          <>
            <span className="mx-board-helper-name">{helper.name}<span className="mx-board-helper-kind">{helper.kind}</span><span className="mx-board-helper-word">{subagentWord(helper, now)}</span></span>
            {helper.task !== null && <><span className="mx-board-helper-head">Asked to</span><p>{helper.task}</p></>}
            {(helper.result ?? helper.latest) !== null && <><span className="mx-board-helper-head">{helper.result === null ? 'Doing' : 'Reported'}</span><p>{helper.result ?? helper.latest}</p></>}
            <div><Button variant="soft" size="sm" onClick={() => { onOpenHelper(helper.id) }}>Open its chat</Button></div>
          </>
        )}
      </PopupContent>
    </Popover.Root>
  )
}

type Props = {
  /** The run shown, or null while the drawer is closed. */
  run: WorkflowView | null
  helpers: readonly SubagentView[]
  onOpenHelper: (id: string) => void
  onClose: () => void
  /** The board it covers: the drawer and its dim mount there, so they cover the pane and not the window. */
  container: HTMLElement | null
}

/**
 * One run, in a drawer at the board's right over the dimmed board (ADR 0010): where it is, its ticket, and every step
 * with its mark, name, word and note, as the workflow reported them. Escape, the close button or a click outside
 * closes it.
 */
export function RunDrawer({ run, helpers, onOpenHelper, onClose, container }: Props) {
  const link = run?.link !== null && run?.link !== undefined && WEB_ADDRESS.test(run.link) ? run.link : null
  return (
    <Dialog open={run !== null} onOpenChange={(open) => { if (!open) onClose() }}>
      <DialogContent unstyled container={container} overlayClassName="mx-board-dim" className="mx-board-drawer" aria-label={run === null ? undefined : `Run ${run.label}`} aria-describedby={undefined}>
        {run !== null && (
          <>
            <header className="mx-board-drawer-head">
              <nav className="mx-board-crumbs" aria-label="Where">
                <span>Work</span><Icon name="chevron" /><strong>{run.label}</strong>
              </nav>
              <span className="mx-spacer" />
              {link !== null && <Button variant="ghost" size="sm" onClick={() => { followLink(link) }}>Open ticket</Button>}
              <IconButton label="Close run" onClick={onClose}><Icon name="close" /></IconButton>
            </header>
            {run.title !== null && <h3 className="mx-board-drawer-title">{run.title}</h3>}
            <ol className="mx-board-steps" aria-label="Steps">
              {run.steps.map((step) => {
                const label = step.helper
                const helper = label === null ? undefined : helpers.find((child) => namesHelper(label, child))
                return (
                  <li key={step.name} className={`mx-board-step is-${step.state}`} aria-current={step === run.current ? 'step' : undefined}>
                    <WorkStepMark state={step.state} />
                    <span className="mx-board-step-text">
                      <span className="mx-board-step-line">
                        <span className="mx-board-step-name">{step.name}</span>
                        <span className="mx-board-step-word">{STEP_WORDS[step.state]}</span>
                      </span>
                      {step.note !== null && <span className="mx-board-step-note">{step.note}</span>}
                    </span>
                    {label !== null && <HelperCard label={label} helper={helper} onOpenHelper={onOpenHelper} />}
                  </li>
                )
              })}
            </ol>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
