/** What the host says when the window has no line to the service: while it is still dialling, and while the service is away. */
const NOT_CONNECTED = 'not connected to the daemon'

/** Whether a failure's only reason is that the window has no line to the service. */
export function saysNotConnected(message: string): boolean {
  return message.includes(NOT_CONNECTED)
}
