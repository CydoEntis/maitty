const ADJECTIVES = ['bold', 'brave', 'bright', 'calm', 'clever', 'fresh', 'keen', 'lucky', 'quiet', 'sly', 'steady', 'swift', 'tall', 'warm', 'wise', 'young'] as const
const ANIMALS = ['badger', 'crane', 'elk', 'finch', 'hare', 'heron', 'kite', 'lynx', 'marten', 'otter', 'owl', 'raven', 'robin', 'stoat', 'wren', 'yak'] as const
/** After this many names already in use, a number is added instead of trying again. */
const TRIES = 8

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
  let suffix = 2
  while (taken.has(`${name}-${String(suffix)}`)) suffix += 1
  return `${name}-${String(suffix)}`
}
