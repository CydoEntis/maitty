/** Where the sessions that started with Claude allowed to use Bypass permissions are remembered, so the mode dropdown can say which can. */
const KEY = 'maitty.gui.yoloSessions.v1'

function read(): string[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]')
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : []
  } catch {
    return []
  }
}

/** The most sessions kept; older ones are long closed. */
const KEEP = 200

export function rememberYoloSession(id: string): void {
  try {
    localStorage.setItem(KEY, JSON.stringify([...read().filter((known) => known !== id), id].slice(-KEEP)))
  } catch {
    // Without storage the dropdown simply keeps Bypass off for this session, which is the safe side.
  }
}

/** The session started with its agent's yolo flag, so Bypass permissions can be picked in it. */
export function startedWithYolo(id: string | undefined): boolean {
  return id !== undefined && read().includes(id)
}
