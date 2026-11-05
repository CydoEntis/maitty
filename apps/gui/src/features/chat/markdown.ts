/**
 * A small markdown reader for the agent's messages. Agent text is untrusted (docs/SECURITY.md), so
 * this parses into a plain data tree and the view builds React elements from it: there is no HTML
 * output anywhere, a raw `<script>` or `<img onerror>` in a message is just text, and a link is kept
 * only when it is an http(s) address. Images are never loaded; they show as their alt text.
 *
 * Supported: paragraphs, `#` headings, fenced code, bullet and numbered lists, `>` quotes, rules,
 * and inline code, bold, italic and links. Anything else reads as the plain text it is.
 */

export type Inline =
  | { readonly kind: 'text'; readonly text: string }
  | { readonly kind: 'code'; readonly text: string }
  | { readonly kind: 'strong'; readonly children: readonly Inline[] }
  | { readonly kind: 'em'; readonly children: readonly Inline[] }
  /** `href` is a safe http(s) address, shown to the reader beside the label. */
  | { readonly kind: 'link'; readonly href: string; readonly children: readonly Inline[] }

export type Block =
  | { readonly kind: 'paragraph'; readonly inlines: readonly Inline[] }
  | { readonly kind: 'heading'; readonly level: number; readonly inlines: readonly Inline[] }
  | { readonly kind: 'code'; readonly language: string; readonly text: string }
  | { readonly kind: 'list'; readonly ordered: boolean; readonly items: readonly (readonly Inline[])[] }
  | { readonly kind: 'quote'; readonly inlines: readonly Inline[] }
  | { readonly kind: 'rule' }

/** Inline parsing rescans for closers, so a paragraph longer than this is shown as plain text, never as a quadratic scan. */
export const INLINE_PARSE_LIMIT = 6000
const MAX_INLINE_DEPTH = 4
const ESCAPE_LENGTH = 2
const LINK_TARGET_OFFSET = 2
const SCHEME_PROBE_LENGTH = 8
const LANGUAGE_LIMIT = 24
const SAFE_PROTOCOLS: readonly string[] = ['http:', 'https:']

/** The address if it is an absolute http(s) URL; `null` for `javascript:`, `data:`, relative links and the rest. */
export function safeHref(raw: string): string | null {
  const text = raw.trim()
  if (text === '' || /[\s<>"]/.test(text)) return null
  try {
    const url = new URL(text)
    return SAFE_PROTOCOLS.includes(url.protocol) ? url.href : null
  } catch {
    return null
  }
}

const PUNCTUATION = '\\`*_{}[]()#+-.!|~<>'
const isWordChar = (char: string | undefined) => char !== undefined && /[\p{L}\p{N}]/u.test(char)

function plain(text: string): readonly Inline[] {
  return text === '' ? [] : [{ kind: 'text', text }]
}

/** The inline tree of one run of text. */
export function parseInline(source: string, depth = 0): readonly Inline[] {
  if (source.length > INLINE_PARSE_LIMIT || depth > MAX_INLINE_DEPTH) return plain(source)
  const out: Inline[] = []
  let buffer = ''
  const flush = () => {
    if (buffer !== '') out.push({ kind: 'text', text: buffer })
    buffer = ''
  }
  let i = 0
  while (i < source.length) {
    const char = source.charAt(i)
    if (char === '\\' && i + 1 < source.length && PUNCTUATION.includes(source.charAt(i + 1))) {
      buffer += source.charAt(i + 1)
      i += ESCAPE_LENGTH
      continue
    }
    if (char === '`') {
      let ticks = 1
      while (source.charAt(i + ticks) === '`') ticks += 1
      const fence = '`'.repeat(ticks)
      const end = source.indexOf(fence, i + ticks)
      if (end !== -1) {
        flush()
        out.push({ kind: 'code', text: source.slice(i + ticks, end).replace(/^ (.*) $/, '$1') })
        i = end + ticks
        continue
      }
      buffer += fence
      i += ticks
      continue
    }
    if (char === '!' && source.charAt(i + 1) === '[') {
      // An image is never loaded: only its alt text is kept.
      const label = readLink(source, i + 1)
      if (label !== null) {
        buffer += label.label
        i = label.end
        continue
      }
    }
    if (char === '[') {
      const found = readLink(source, i)
      if (found !== null) {
        const href = safeHref(found.target)
        flush()
        if (href === null) out.push(...parseInline(found.label, depth + 1))
        else out.push({ kind: 'link', href, children: parseInline(found.label, depth + 1) })
        i = found.end
        continue
      }
    }
    if ((char === '*' || char === '_') && !(char === '_' && isWordChar(source[i - 1]))) {
      const double = source.charAt(i + 1) === char
      const mark = double ? char + char : char
      const start = i + mark.length
      const end = source.indexOf(mark, start)
      const inner = end === -1 ? '' : source.slice(start, end)
      const closes = end !== -1 && inner !== '' && !/^\s|\s$/.test(inner) && !(char === '_' && isWordChar(source[end + mark.length]))
      if (closes) {
        flush()
        out.push({ kind: double ? 'strong' : 'em', children: parseInline(inner, depth + 1) })
        i = end + mark.length
        continue
      }
    }
    if ((char === 'h' || char === 'H') && /^https?:\/\//i.test(source.slice(i, i + SCHEME_PROBE_LENGTH)) && !isWordChar(source[i - 1])) {
      const match = /^\S+/.exec(source.slice(i))
      const raw = (match?.[0] ?? '').replace(/[.,;:!?)\]}'"]+$/, '')
      const href = safeHref(raw)
      if (href !== null) {
        flush()
        out.push({ kind: 'link', href, children: plain(raw) })
        i += raw.length
        continue
      }
    }
    buffer += char
    i += 1
  }
  flush()
  return out
}

/** `[label](target)` at `start`, with the index after it; `null` when it is not a link. */
function readLink(source: string, start: number): { label: string; target: string; end: number } | null {
  const close = source.indexOf(']', start + 1)
  if (close === -1 || source.charAt(close + 1) !== '(') return null
  // The target may hold balanced parentheses (`javascript:alert(1)`), which must not end it early.
  let end = -1
  let depth = 1
  for (let at = close + LINK_TARGET_OFFSET; at < source.length; at += 1) {
    const char = source.charAt(at)
    if (char === '(') depth += 1
    if (char === ')') depth -= 1
    if (depth === 0) {
      end = at
      break
    }
  }
  if (end === -1) return null
  const label = source.slice(start + 1, close)
  if (label.includes('[')) return null
  return { label, target: source.slice(close + LINK_TARGET_OFFSET, end), end: end + 1 }
}

const FENCE = /^\s{0,3}(```+|~~~+)\s*([^\s`]*)/
const HEADING = /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/
const BULLET = /^\s{0,3}[-*+]\s+(.*)$/
const NUMBERED = /^\s{0,3}\d{1,9}[.)]\s+(.*)$/
const QUOTE = /^\s{0,3}>\s?(.*)$/
const RULE = /^\s{0,3}(?:-{3,}|\*{3,}|_{3,})\s*$/

/** The blocks of a message. Never throws; any text is a valid document. */
export function parseMarkdown(source: string): readonly Block[] {
  const lines = source.replace(/\r\n?/g, '\n').split('\n')
  const blocks: Block[] = []
  let paragraph: string[] = []
  const endParagraph = () => {
    if (paragraph.length > 0) blocks.push({ kind: 'paragraph', inlines: parseInline(paragraph.join('\n')) })
    paragraph = []
  }

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i] ?? ''
    const fence = FENCE.exec(line)
    if (fence) {
      endParagraph()
      const marker = fence[1] ?? '```'
      const body: string[] = []
      i += 1
      // A fence left open runs to the end of the message.
      while (i < lines.length && !new RegExp(`^\\s{0,3}${marker.charAt(0)}{${String(marker.length)},}\\s*$`).test(lines[i] ?? '')) {
        body.push(lines[i] ?? '')
        i += 1
      }
      blocks.push({ kind: 'code', language: (fence[2] ?? '').slice(0, LANGUAGE_LIMIT), text: body.join('\n') })
      continue
    }
    if (line.trim() === '') {
      endParagraph()
      continue
    }
    const heading = HEADING.exec(line)
    if (heading) {
      endParagraph()
      blocks.push({ kind: 'heading', level: heading[1]?.length ?? 1, inlines: parseInline(heading[2] ?? '') })
      continue
    }
    if (RULE.test(line)) {
      endParagraph()
      blocks.push({ kind: 'rule' })
      continue
    }
    const bullet = BULLET.exec(line)
    const numbered = bullet ? null : NUMBERED.exec(line)
    const item = bullet ?? numbered
    if (item) {
      endParagraph()
      const items: Inline[][] = []
      let match: RegExpExecArray | null = item
      while (match) {
        let text = match[1] ?? ''
        // An indented line after a list entry continues it.
        while (i + 1 < lines.length && /^\s{2,}\S/.test(lines[i + 1] ?? '') && !BULLET.test(lines[i + 1] ?? '') && !NUMBERED.test(lines[i + 1] ?? '')) {
          i += 1
          text += `\n${(lines[i] ?? '').trim()}`
        }
        items.push([...parseInline(text)])
        const next = lines[i + 1] ?? ''
        const following: RegExpExecArray | null = bullet ? BULLET.exec(next) : NUMBERED.exec(next)
        if (following) i += 1
        match = following
      }
      blocks.push({ kind: 'list', ordered: numbered !== null, items })
      continue
    }
    const quote = QUOTE.exec(line)
    if (quote) {
      endParagraph()
      const quoted = [quote[1] ?? '']
      while (i + 1 < lines.length && QUOTE.test(lines[i + 1] ?? '')) {
        i += 1
        quoted.push(QUOTE.exec(lines[i] ?? '')?.[1] ?? '')
      }
      blocks.push({ kind: 'quote', inlines: parseInline(quoted.join('\n')) })
      continue
    }
    paragraph.push(line)
  }
  endParagraph()
  return blocks
}
