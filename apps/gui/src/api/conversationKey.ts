import { LOCAL_MACHINE } from './machines'

/**
 * A conversation's number is only unique on its machine, but the chat code keeps its feeds, drafts
 * and queues by that one number. A conversation of another machine is therefore named by a key that
 * holds the machine too: the machine's number above 32 bits, the conversation's below. This computer
 * is machine 0, so a local conversation's key is its own number, unchanged. The API layer splits a
 * key back before it asks the host, so nothing else has to know.
 */
const MACHINE_STRIDE = 4_294_967_296 // 2 to the 32nd
/** The most machines a window numbers: the host's numbers are 16 bits. */
const MOST_MACHINE = 65_535

/** The key for a conversation of `machine`, or null when its number is too large to keep apart from other machines'. */
export function conversationKey(machine: number, id: number): number | null {
  if (!Number.isSafeInteger(id) || id < 0 || id >= MACHINE_STRIDE) return null
  if (!Number.isInteger(machine) || machine < 0 || machine > MOST_MACHINE) return null
  return machine * MACHINE_STRIDE + id
}

/** The machine and the conversation number on it that `key` names. */
export function splitConversationKey(key: number): { machine: number; id: number } {
  return { machine: Math.floor(key / MACHINE_STRIDE), id: key % MACHINE_STRIDE }
}

/** What a host command needs to reach a conversation's machine: nothing for this computer. */
export function machineArgOfKey(key: number): { machine?: number } {
  const { machine } = splitConversationKey(key)
  return machine === LOCAL_MACHINE ? {} : { machine }
}
