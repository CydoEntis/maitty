import { Spinner } from '@/features/workspace/components/glyphs'

/**
 * The mark before "Claude is reading…" in a chat (`WorkingLine`): the working spinner the person picked in Settings, in
 * the working colour. The line names the agent, so the mark needs no tile.
 */
export function WorkingMark({ agent }: { agent: string }) {
  return <span className="mx-working-mark" aria-hidden="true" data-agent={agent}><Spinner /></span>
}
