const STORAGE_KEY = 'maitty.gui.machine-nicknames.v1'

/** Names given to machines in the sidebar, by the machine's real name in lower case. */
export type MachineNicknames = Record<string, string>

function isNicknames(value: unknown): value is MachineNicknames {
  return typeof value === 'object' && value !== null && !Array.isArray(value) && Object.values(value).every((item) => typeof item === 'string')
}

/** What the window shows for each machine now; kept in step with what is read and written, so any text can name a machine by the name its owner gave it. */
let shown: MachineNicknames = {}

/** The name to show for a machine: the one given to it, else its own. */
export function machineLabel(name: string): string
export function machineLabel(name: string | null): string | null
export function machineLabel(name: string | null): string | null {
  if (name === null) return null
  return shown[name.toLowerCase()] ?? name
}

export function readMachineNicknames(): MachineNicknames {
  try {
    const stored = localStorage.getItem(STORAGE_KEY)
    const value: unknown = stored === null ? {} : JSON.parse(stored)
    shown = isNicknames(value) ? value : {}
    return shown
  } catch (error) {
    console.warn('Ignoring invalid Maitty machine names', error)
    return {}
  }
}

/** `nicknames` with `machine` named `nickname`; an empty name takes the nickname away. */
export function withNickname(nicknames: MachineNicknames, machine: string, nickname: string): MachineNicknames {
  const key = machine.toLowerCase()
  const rest = Object.fromEntries(Object.entries(nicknames).filter(([other]) => other !== key))
  const trimmed = nickname.trim()
  return trimmed === '' ? rest : { ...rest, [key]: trimmed }
}

export function writeMachineNicknames(nicknames: MachineNicknames): void {
  shown = nicknames
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(nicknames))
  } catch (error) {
    console.warn('Could not keep the machine names', error)
  }
}
