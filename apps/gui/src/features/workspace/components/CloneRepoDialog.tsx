import { useState } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { errorMessage } from '@/lib/error-message'
import { GitHubRepoPicker, type GitHubRepoSource } from './GitHubRepoPicker'

/** Longest address and path the host accepts. */
const MOST_ADDRESS_CHARS = 2048
const MOST_PATH_CHARS = 4096

type Props = {
  /** Where the clone lands, as the title says it: "this computer" or a machine's name. */
  where: string
  /** The folder the destination starts as. */
  startIn: string
  /** A folder picker for that machine, whose answer is null when it is cancelled. */
  chooseFolder: (() => Promise<string | null>) | null
  /** Clones `address` into a new folder inside `parent` and gives that folder; rejects with the machine's reason. */
  clone: (address: string, parent: string) => Promise<string>
  /** That machine's GitHub repositories to pick from; without it the dialog takes a pasted address only. */
  github?: GitHubRepoSource | undefined
  onCloned: (folder: string, parent: string) => void
  onCancel: () => void
}

/**
 * Brings a repository onto a machine without leaving the window: its address, and the folder the
 * clone goes inside. The machine's daemon runs git, so the address is judged and the sign-in is
 * found there; what it says when it will not clone is shown as it said it. Picking one of the
 * machine's GitHub repositories fills in its address.
 */
export function CloneRepoDialog({ where, startIn, chooseFolder, clone, github, onCloned, onCancel }: Props) {
  const [address, setAddress] = useState('')
  const [parent, setParent] = useState(startIn)
  const [cloning, setCloning] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const ready = address.trim() !== '' && parent.trim() !== '' && !cloning

  const run = () => {
    if (!ready) return
    const into = parent.trim()
    setCloning(true)
    setFailure(null)
    clone(address.trim(), into).then(
      (folder) => { onCloned(folder, into) },
      (error: unknown) => {
        setFailure(errorMessage(error, 'The repository could not be cloned'))
        setCloning(false)
      },
    )
  }
  const choose = () => {
    if (chooseFolder === null) return
    void chooseFolder().then((chosen) => { if (chosen !== null) setParent(chosen) })
  }

  return (
    // A clone that is running cannot be taken back, so the dialog stays until it has ended.
    <Dialog open onOpenChange={(isOpen) => { if (!isOpen && !cloning) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
        <form
          aria-label={`Clone a repository to ${where}`}
          className="mx-rename mx-getfile"
          onSubmit={(event) => { event.preventDefault(); run() }}
          onKeyDown={(event) => {
            // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
            if (event.key === 'Escape') event.stopPropagation()
          }}
        >
          <DialogTitle asChild><h2>Clone a repository to {where}</h2></DialogTitle>
          <input
            autoFocus
            aria-label="Repository address"
            placeholder="https://github.com/owner/name.git or git@github.com:owner/name.git"
            value={address}
            maxLength={MOST_ADDRESS_CHARS}
            spellCheck={false}
            disabled={cloning}
            onChange={(event) => { setAddress(event.target.value) }}
          />
          {github !== undefined && <GitHubRepoPicker where={where} source={github} disabled={cloning} onPick={(repo) => { setAddress(repo.cloneUrl) }} />}
          <p>It goes in a new folder, named after the repository, inside:</p>
          <div className="mx-clone-into">
            <input aria-label={`Folder on ${where} to clone into`} value={parent} maxLength={MOST_PATH_CHARS} spellCheck={false} disabled={cloning} onChange={(event) => { setParent(event.target.value) }} />
            {chooseFolder !== null && <button type="button" className="mx-text-button" disabled={cloning} onClick={choose}>Choose…</button>}
          </div>
          {cloning && <p role="status">Cloning… A large repository can take a few minutes.</p>}
          {failure !== null && <p role="alert">{failure}</p>}
          <div className="mx-rename-actions">
            <button type="button" className="mx-text-button" disabled={cloning} onClick={onCancel}>Cancel</button>
            <button type="submit" className="mx-text-button is-primary" disabled={!ready}>{cloning ? 'Cloning…' : 'Clone'}</button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  )
}
