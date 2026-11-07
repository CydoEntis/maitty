const THOUSAND = 1000
const MILLION = THOUSAND * THOUSAND
const ONE_DECIMAL_BELOW = 10
const TENTH_STEPS = 10

/** `812`, `12.4k`, `1.3M`: a token count in the room a pill has. */
export function formatTokens(tokens: number): string {
  if (tokens >= MILLION) return `${(tokens / MILLION).toFixed(1)}M`
  if (tokens >= THOUSAND) {
    const tenths = Math.round((tokens / THOUSAND) * TENTH_STEPS) / TENTH_STEPS
    return `${tenths < ONE_DECIMAL_BELOW ? tenths.toFixed(1) : String(Math.round(tenths))}k`
  }
  return String(tokens)
}
