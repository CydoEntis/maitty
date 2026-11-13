import { useCallback, useEffect, useMemo, useState } from 'react'
import { toast } from '@/lib/toast'
import { errorMessage } from '@/lib/error-message'
import { useLatest } from '@/lib/useLatest'
import { repoSnapshots, restoreCount, restoreSnapshot } from '@/features/inspect/api'
import { snapshotBefore } from '@/features/inspect/turnRange'
import type { ChatEvent } from './chatEvents'
import { peekComposer, updateComposer } from './composerStore'
import { confirmBody, restoreNote, turnSpanOf, type EditFromHere, type RewindPlace } from './editFromHere'
import { messageIntoDraft } from './useChatComposer'

/** A restore waiting for the person's yes: the message to put back, the snapshot, and the confirm's words. */
export type RestoreAsk = { readonly text: string; readonly commit: string; readonly body: string; readonly confirmLabel: string }

type Input = {
  /** Null where the chat offers no Edit from here at all. */
  place: RewindPlace | null
  conversation: number
  events: readonly ChatEvent[]
  agent: string
  /** Why nothing can be restored right now (the agent works, the daemon is away), or null. */
  busy: string | null
}

const refused = (why: string) => { toast.error(`Can't edit from here: ${why}`) }

/**
 * Edit from here on a message the person sent: the checkout goes back to the snapshot taken before that message's turn,
 * once the person confirmed how many files change, and the message goes back in the box with a note for the agent.
 */
export function useEditFromHere({ place, conversation, events, agent, busy }: Input) {
  const [ask, setAsk] = useState<RestoreAsk | null>(null)
  const eventsNow = useLatest(() => events)
  const term = place !== null && 'term' in place ? place.term : null

  const pick = useCallback((message: { readonly text: string; readonly turnId: string }) => {
    if (term === null) return
    const turn = turnSpanOf(eventsNow(), message.turnId)
    if (turn === null) {
      refused("this message's turn has no times from the daemon to find its snapshot by")
      return
    }
    const prepare = async () => {
      const before = snapshotBefore(await repoSnapshots(term), turn)
      if (before === undefined) {
        refused('no snapshot from before this message is kept')
        return
      }
      // Counted now, against the files as they are: the confirm never offers a restore it cannot size.
      const count = await restoreCount(term, before.commit)
      const hasDraft = peekComposer(conversation).draft.trim() !== ''
      setAsk({ text: message.text, commit: before.commit, body: confirmBody(count, agent, hasDraft), confirmLabel: count === 0 ? 'Edit from here' : `Restore ${count === 1 ? '1 file' : `${String(count)} files`}` })
    }
    prepare().catch((failure: unknown) => { refused(errorMessage(failure, "couldn't count the files a restore would change")) })
  }, [term, eventsNow, conversation, agent])

  const busyNow = useLatest(() => busy)
  const confirm = useCallback(() => {
    if (ask === null || term === null) return
    setAsk(null)
    const why = busyNow()
    if (why !== null) {
      refused(why)
      return
    }
    restoreSnapshot(term, ask.commit).then(
      (changed) => {
        updateComposer(conversation, (state) => ({ ...messageIntoDraft(ask.text)(state), note: restoreNote(ask.text) }))
        toast.success(changed === 0 ? 'The files already matched; the message is back in the box' : `Restored ${changed === 1 ? '1 file' : `${String(changed)} files`}; the message is back in the box`)
      },
      (failure: unknown) => { toast.error(errorMessage(failure, 'Could not restore the files')) },
    )
  }, [ask, term, busyNow, conversation])
  const cancel = useCallback(() => { setAsk(null) }, [])

  // Leaving the chat drops the note: it belongs to the message the person was about to send here.
  useEffect(() => () => { updateComposer(conversation, (state) => (state.note === null ? state : { ...state, note: null })) }, [conversation])

  const offered = place !== null
  const disabled = place === null ? null : 'refusal' in place ? place.refusal : busy
  const action = useMemo<EditFromHere | null>(() => (offered ? { disabled, pick } : null), [offered, disabled, pick])
  return { action, ask, confirm, cancel }
}
