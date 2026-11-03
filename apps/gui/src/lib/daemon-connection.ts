/** The daemon and GUI protocol numbers when the daemon refused this GUI for speaking another one. */
export function daemonProtocolMismatch(
  error: unknown,
): { daemonProtocol: number; guiProtocol: number } | null {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : ''
  const versionMismatch =
    /protocol v(\d+), (?:this one v|while this GUI needs v)(\d+)/.exec(message)
  if (!versionMismatch) return null
  const daemonProtocol = Number(versionMismatch[1])
  const guiProtocol = Number(versionMismatch[2])
  return Number.isSafeInteger(daemonProtocol) &&
    Number.isSafeInteger(guiProtocol)
    ? { daemonProtocol, guiProtocol }
    : null
}

/** A connection failure in words a user can act on. */
export function daemonConnectionError(error: unknown): string {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : ''
  const mismatch = daemonProtocolMismatch(error)
  if (mismatch) {
    return `The Maitty service uses protocol v${String(mismatch.daemonProtocol)}, but this GUI needs v${String(mismatch.guiProtocol)}. This GUI will leave the service running while it owns sessions.`
  }
  return message
    ? `Could not connect to the local Maitty service: ${message}`
    : 'Could not connect to the local Maitty service. Check that Maitty is installed, then retry.'
}
