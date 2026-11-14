import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { forgetAllowedCommand, loadAllowedCommands, type AllowedCommand } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { Card } from './DesktopSettings'

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
    <Card title="Commands approved for a project" foot="Forgetting one takes effect at once. Another Maitty service running on this computer (a separate socket) only drops it when it next starts.">
      {list.isError && <p className="mx-scard-note">{errorMessage(list.error, 'Could not read them')}</p>}
      {list.data?.length === 0 && <p className="mx-scard-note">None. A chat's command offers “Always in this project”; what you choose appears here.</p>}
      {(list.data ?? []).map((item) => (
        <div key={`${item.project}\n${item.command_sha256}`} className="mx-scard-row mx-approved-row">
          <div className="mx-scard-text">
            <code title={item.command}>{item.command}</code>
            <span title={item.project}>{projectName(item.project)} · {new Date(item.at * MS_PER_SECOND).toLocaleDateString()}</span>
          </div>
          <button type="button" className="mx-text-button" aria-label={`Forget ${item.command}`} onClick={() => { forget(item) }}>Forget</button>
        </div>
      ))}
      {failure !== null && <p className="mx-commit-error" role="alert">{failure}</p>}
    </Card>
  )
}
