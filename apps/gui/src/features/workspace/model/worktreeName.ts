const ADJECTIVES = ['bold', 'brave', 'bright', 'calm', 'clever', 'fresh', 'keen', 'lucky', 'quiet', 'sly', 'steady', 'swift', 'tall', 'warm', 'wise', 'young'] as const
const ANIMALS = ['badger', 'crane', 'elk', 'finch', 'hare', 'heron', 'kite', 'lynx', 'marten', 'otter', 'owl', 'raven', 'robin', 'stoat', 'wren', 'yak'] as const
/** After this many names already in use, a number is added instead of trying again. */
const TRIES = 8
/** The longest a branch named after a message is, before any -2 it takes. */
const SLUG_MAX = 40

/** `name`, or the first of `name-2`, `name-3`, … that is free. */
function freeName(name: string, taken: ReadonlySet<string>): string {
  if (!taken.has(name)) return name
  let suffix = 2
  while (taken.has(`${name}-${String(suffix)}`)) suffix += 1
  return `${name}-${String(suffix)}`
}

/**
 * A name for a branch nobody named: two short words, like the terminal app's worktrees. "New
 * worktree" on the new-chat page needs a branch at once, because one branch cannot be checked out
 * in two folders. Never one of `taken`.
 */
export function suggestBranchName(taken: ReadonlySet<string>, random: () => number = Math.random): string {
  const pick = (words: readonly string[]) => words[Math.min(words.length - 1, Math.floor(random() * words.length))] ?? words[0] ?? ''
  let name = ''
  for (let attempt = 0; attempt < TRIES; attempt += 1) {
    name = `${pick(ADJECTIVES)}-${pick(ANIMALS)}`
    if (!taken.has(name)) return name
  }
  return freeName(name, taken)
}

/**
 * The first words of a message as a branch name, or null when it has none: lowercase ASCII letters
 * and digits joined by `-`, at most 40 characters cut between words. Accented letters become their
 * plain ones; anything else (punctuation, `/`, `!`, other scripts) only separates words, so the name
 * can never hold `..`, `.lock` or a leading `-` that git refuses.
 */
export function branchSlug(message: string): string | null {
  const words = message.normalize('NFKD').replace(/\p{M}/gu, '').toLowerCase().split(/[^a-z0-9]+/).filter(Boolean)
  let slug = ''
  for (const word of words) {
    const next = slug === '' ? word : `${slug}-${word}`
    if (next.length > SLUG_MAX) break
    slug = next
  }
  if (slug === '' && words[0] !== undefined) slug = words[0].slice(0, SLUG_MAX)
  return slug === '' ? null : slug
}

/**
 * A free branch named after the message (`fix-login`, then `fix-login-2`, …), or null when the
 * message has no usable words. A branch `x/…` counts as `x` in use, since git cannot hold both.
 */
export function branchNameFromMessage(message: string, taken: ReadonlySet<string>): string | null {
  const slug = branchSlug(message)
  if (slug === null) return null
  const blocked = new Set(taken)
  for (const name of taken) {
    const folder = name.indexOf('/')
    if (folder > 0) blocked.add(name.slice(0, folder))
  }
  return freeName(slug, blocked)
}
