/** The bytes of a host reply sent raw (`tauri::ipc::Response`), which arrive as an ArrayBuffer. */
export function bytesOf(reply: unknown): Uint8Array {
  if (reply instanceof ArrayBuffer) return new Uint8Array(reply)
  if (reply instanceof Uint8Array) return reply
  throw new Error('The desktop host did not answer with bytes')
}
