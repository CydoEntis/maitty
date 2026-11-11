/**
 * The sessions the person opened as a plain terminal. An agent typed into one later is still their
 * terminal, so its pane does not turn into the chat by itself; the view switch still shows the chat.
 * Kept in this window's storage so it holds after a restart, as the sessions themselves do.
 */
const KEY = 'maitty.gui.shellStarts.v1'

/** The most sessions kept; older ones are long closed. */
const KEEP = 200

function read(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]')
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}

export function rememberShellStart(id: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify([...read().filter((known) => known !== id), id].slice(-KEEP)))
  } catch {
    // Without storage the pane follows the agent's default view, as it did before this was kept.
  }
}

export function startedAsShell(id: string): boolean {
  return read().includes(id)
}
