export type PathLink = { start: number; end: number; path: string; line?: number }

// A run of path characters, optionally followed by :line (and :column, which is ignored).
const CANDIDATE = /(?:[A-Za-z]:[\\/])?[\w@+~.\\/-]+(?::(\d+))?(?::\d+)?/g
const EXTENSION = /\.[A-Za-z][A-Za-z0-9]{0,7}$/
// What a URL's path part follows: `https:` right before the `//`.
const AFTER_SCHEME = /[A-Za-z][A-Za-z0-9+.-]*:$/
const MIN_STEM = 2
/** The endings a bare name must have to be read as a site and not a file: the common ones, so `main.rs` or `notes.md` never is. */
const SITE_ENDINGS = 'com|org|net|io|dev|ai|app|co|gov|edu'
const BARE_DOMAIN = new RegExp(`^(?:[A-Za-z0-9-]+\\.)+(?:${SITE_ENDINGS})$`, 'i')
// An address with its scheme, or a bare site name with an optional path, not part of an email or a longer path.
const WEB = new RegExp(`https?://[^\\s<>"'\`]+|(?<![\\w@./-])(?:[A-Za-z0-9-]+\\.)+(?:${SITE_ENDINGS})(?:/[^\\s<>"'\`]*)?(?![\\w-])`, 'gi')

export type WebLink = { start: number; end: number; url: string }

/**
 * Web addresses in one line of terminal output: `https://example.com/a?b=1`, and a bare site name such as
 * `app.hubspot.com`, which is opened as `https://`. Punctuation that ends the sentence is left out.
 */
export function findWebLinks(text: string): WebLink[] {
  const links: WebLink[] = []
  for (const match of text.matchAll(WEB)) {
    const shown = match[0].replace(/[.,;:!?)\]]+$/, '')
    if (shown === '') continue
    links.push({ start: match.index, end: match.index + shown.length, url: /^https?:\/\//i.test(shown) ? shown : `https://${shown}` })
  }
  return links
}

/**
 * File-like words in one line of terminal output: `src/main.rs:42`, `./a/b.ts`, `C:\dev\x.md`. A word
 * needs an extension that starts with a letter, so versions, numbers and bare words are left alone.
 */
export function findPathLinks(text: string): PathLink[] {
  const links: PathLink[] = []
  for (const match of text.matchAll(CANDIDATE)) {
    const whole = match[0]
    const start = match.index
    if (whole.startsWith('//') && AFTER_SCHEME.test(text.slice(0, start))) continue
    const withoutLine = whole.replace(/(?::\d+){1,2}$/, '')
    const path = withoutLine.replace(/[.,;]+$/, '')
    if (!EXTENSION.test(path)) continue
    // `app.hubspot.com` ends like a file, but it is a site: `findWebLinks` offers it.
    if (BARE_DOMAIN.test(path)) continue
    const stem = path.split(/[\\/]/).at(-1) ?? ''
    if (stem.startsWith('.') && stem.indexOf('.', 1) < 0) continue
    // A bare "e.g" or "i.e" is prose; the same short name inside a folder is a file.
    if (stem === path && stem.slice(0, stem.lastIndexOf('.')).length < MIN_STEM) continue
    const line = match[1] === undefined ? undefined : Number(match[1])
    links.push({ start, end: start + path.length + (line === undefined ? 0 : whole.length - withoutLine.length), path, ...(line === undefined ? {} : { line }) })
  }
  return links
}
