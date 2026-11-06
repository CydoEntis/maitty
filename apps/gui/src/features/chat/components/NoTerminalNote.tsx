/**
 * Where a chat session's terminal would be. The agent of a chat session speaks to the daemon over
 * pipes, so it has no terminal to show (ADR 0003); the chat above covers this, and this says why
 * for anyone who reads the pane without it (a screen reader, or the instant before the chat draws).
 */
export function NoTerminalNote() {
  return (
    <p className="m-0 p-4 text-[13px] text-dim">
      This is a chat session. Its agent has no terminal. To use the agent&rsquo;s own interface, start a terminal session instead.
    </p>
  )
}
