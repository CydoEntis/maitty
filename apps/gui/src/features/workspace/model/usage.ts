import type { TerminalDetails } from '@/data/preview'

const THOUSAND = 1000
const MILLION = THOUSAND * THOUSAND
const ONE_DECIMAL_BELOW = 10
const TENTH_STEPS = 10
const COST_DECIMALS = 2

/** `812`, `12.4k`, `1.3M`: a token count in the room a pill has. */
export function formatTokens(tokens: number): string {
  if (tokens >= MILLION) return `${(tokens / MILLION).toFixed(1)}M`
  if (tokens >= THOUSAND) {
    const tenths = Math.round((tokens / THOUSAND) * TENTH_STEPS) / TENTH_STEPS
    return `${tenths < ONE_DECIMAL_BELOW ? tenths.toFixed(1) : String(Math.round(tenths))}k`
  }
  return String(tokens)
}

/**
 * What the provider says a session has used, as the words a pill shows. Empty when it said
 * nothing: the pill is then not drawn, because a made-up number would read as a fact.
 */
export function usageParts(usage: TerminalDetails['usage']): string[] {
  const parts: string[] = []
  if (usage.tokens !== null) parts.push(`${formatTokens(usage.tokens)} tokens`)
  if (usage.context !== null) parts.push(`${String(Math.round(usage.context))}% context`)
  if (usage.cost !== null) parts.push(`$${usage.cost.toFixed(COST_DECIMALS)}`)
  return parts
}
