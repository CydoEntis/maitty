import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { forgetAllowedCommand, loadAllowedCommands, type AllowedCommand } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'

const QUERY_KEY = ['allowed-commands']
const MS_PER_SECOND = 1000

/** The folder's own name: enough to tell projects apart in a short list; the full path is the tooltip. */
function projectName(path: string): string {
  const parts = path.split(/[\\/]/).filter((part) => part !== '')
  return parts[parts.length - 1] ?? path
}

/**
 * The commands you approved with "Always in this project" in a chat. Each can be taken back: the background
 * service stops approving it at once, with no restart. Nothing else is ever remembered (ADR 0005).
 */
export function ApprovedCommands() {
  const client = useQueryClient()
  const list = useQuery({ queryKey: QUERY_KEY, queryFn: loadAllowedCommands, retry: false })
  const [failure, setFailure] = useState<string | null>(null)
  const forget = (item: AllowedCommand) => {
    setFailure(null)
    forgetAllowedCommand(item)
      .then(() => client.invalidateQueries({ queryKey: QUERY_KEY }))
      .catch((error: unknown) => { setFailure(errorMessage(error, 'Could not take that back')) })
  }
  return (
    <section className="mx-approved" aria-label="Approved commands">
      <h3 className="mx-menu-caps">Commands approved for a project</h3>
      <p className="mx-inspect-note">Forgetting one takes effect at once. Another Maitty service running on this computer (a separate socket) only drops it when it next starts.</p>
      {list.isError && <p className="mx-inspect-note">{errorMessage(list.error, 'Could not read them')}</p>}
      {list.data?.length === 0 && <p className="mx-inspect-note">None. A chat's command offers “Always in this project”; what you choose appears here.</p>}
      <ul>
        {(list.data ?? []).map((item) => (
          <li key={`${item.project}\n${item.command_sha256}`} className="mx-approved-row">
            <code title={item.command}>{item.command}</code>
            <small title={item.project}>{projectName(item.project)} · {new Date(item.at * MS_PER_SECOND).toLocaleDateString()}</small>
            <button type="button" className="mx-text-button" onClick={() => { forget(item) }}>Forget</button>
          </li>
        ))}
      </ul>
      {failure !== null && <p className="mx-commit-error" role="alert">{failure}</p>}
    </section>
  )
}
