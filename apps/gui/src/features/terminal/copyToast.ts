import { LOCAL_MACHINE } from '@/api/machines'

/** One toast for a copy from a pane, however it was made, so a selection a program also copies says so once. */
export const COPY_TOAST_ID = 'terminal-copy'
export const COPY_TOAST_MS = 1500

/** What the toast says when a program in a pane copied text, and when the clipboard refused it. */
export function copySaid(machine: number, machineName: string, length: number): { done: string; failed: string } {
  if (machine === LOCAL_MACHINE) return { done: 'Copied', failed: 'Could not copy what the program sent' }
  return { done: `${machineName} copied ${String(length)} characters to your clipboard`, failed: `Could not copy what ${machineName} sent` }
}
