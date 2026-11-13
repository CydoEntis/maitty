import { ageOf, type SavedRead } from '../model/newChatHost'

type Props = {
  read: SavedRead
  folderName: string
  /** Own copy or a new worktree is chosen: a resumed conversation leaves it out for this start. */
  leavesCopy: boolean
  /** The message box has text, which goes to the conversation picked. */
  withMessage: boolean
  disabled: boolean
  now: number
  onPick: (id: string) => void
}

/** The new chat page's Resume list: the conversations the agent saved for the folder, newest first, each a click from resuming. */
export function ResumeList({ read, folderName, leavesCopy, withMessage, disabled, now, onPick }: Props) {
  if (read.kind === 'failed') return <p className="mx-nc-quiet">Could not read the saved conversations.</p>
  if (read.kind === 'older') return <p className="mx-nc-quiet">Restart the Maitty service to see saved conversations.</p>
  if (read.kind !== 'listed' || read.list.length === 0) return null
  return (
    <div className="mx-nc-resume">
      <h3 className="mx-nc-heading">Resume</h3>
      {leavesCopy && <p className="mx-nc-quiet">{`A resumed conversation continues in ${folderName}, the folder it was saved in, not in a copy of its own.`}</p>}
      {withMessage && <p className="mx-nc-quiet">Your message is sent to the conversation you pick.</p>}
      <ul className="mx-nc-tasks" aria-label="Resume">
        {read.list.map((saved) => (
          <li key={saved.id}>
            <button type="button" disabled={disabled} title={saved.first_message} onClick={() => { onPick(saved.id) }}>
              <span aria-hidden="true">↻</span>
              <span className="mx-nc-resume-text">{saved.first_message}</span>
              <span className="mx-chip-count">{ageOf(saved.last_active, now)}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  )
}
